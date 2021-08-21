//! Wrappers of file system related syscalls.
// use super::super::fs::file::FILE;
// use crate::fs::block_cache::BLOCK_SZ;
use crate::fs::Path;
use crate::fs::parse_path;
use crate::fs::to_string;
use crate::fs::{self, File, OpenMode, make_pipe, mkdir, open, remove, FileType};
use crate::memory::{VirtAddr};
use crate::process::{current_process, suspend_switch, ErrNo};
use alloc::string::ToString;
use alloc::string::String;
// use alloc::vec::Vec;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::{convert::TryInto, mem::size_of};
use bitflags::*;

/// The special "file descriptor" indicating that the path is relative path to process's current working directory. 
pub const AT_FDCWD: i32 =  -100;

fn get_file_fd(dirfd: usize) -> Result<Arc<dyn File>, ErrNo> {
    let proc = current_process().unwrap();
    let arcpcb = proc.get_inner_locked();
    if dirfd == AT_FDCWD as usize {
        // debug!("fd == current dir");
        // debug!("path: {}", arcpcb.path);
        return open(arcpcb.path.clone(), OpenMode::empty());
    } else {
        if dirfd > arcpcb.files.len() {
            return Err(ErrNo::BadFileDescriptor);
        } 
        if let Some(file) = &arcpcb.files[dirfd] {
            return Ok(file.clone());
        } else {
            return Err(ErrNo::BadFileDescriptor);
        }
    }
}

fn get_file(dirfd: usize, path: &str, mode: OpenMode) -> Result<Arc<dyn File>, ErrNo> {
    if path.len() == 0 {
        return get_file_fd(dirfd);
    }

    let path = parse_path(path).map_err(|_| ErrNo::NoSuchFileOrDirectory)?;

    if path.is_abs {
        return open(path.to_string(), mode);
    } else if path.path.len() == 0 {
        return get_file_fd(dirfd);
    } else {
        let file = get_file_fd(dirfd)?;
        if let Some(dir) = file.to_dir_file() {
            return dir.open(path, mode);
        } else {
            return Err(ErrNo::IsADirectory);
        }
    } 
}

fn makeDirAt(dirfd: usize, path: &str) -> Result<(), ErrNo> {
    let path = match parse_path(path) {
        Ok(path) => path,
        Err(err) => return Err(ErrNo::NoSuchFileOrDirectory),
    };
    if path.is_abs {
        // if path.path.len() > 0 {
        //     debug!("path[0]:{}", path.path[0]);
        //     debug!("path:{}", path.to_string());
        // }
        match mkdir(path.to_string()) {
            Ok(_) => return Ok(()),
            Err(msg) => return Err(msg),
        }
    } else if path.path.len() == 0 {
        return Err(ErrNo::FileExists);
    } else {
        match get_file_fd(dirfd) {
            Ok(file) => {
                if let Some(dir) = file.to_dir_file() {
                    match dir.mkdir(path) {
                        Ok(_) => return Ok(()),
                        Err(msg) => return Err(msg),
                    }
                } else {
                    return Err(ErrNo::NotADirectory);
                }
            },
            Err(msg) => return Err(msg),
        }
    } 
}

fn unlink(dirfd: usize, path: &str) -> Result<(), ErrNo> {
    let path = match parse_path(path) {
        Ok(path) => path,
        Err(err) => return Err(ErrNo::NoSuchFileOrDirectory),
    };
    if path.is_abs {
        return remove(path.to_string());
    } else if path.path.len() == 0 {
        return Err(ErrNo::DeviceOrResourceBusy);
    } else {
        match get_file_fd(dirfd) {
            Ok(file) => {
                if let Some(dir) = file.to_dir_file() {
                    return dir.remove(path);
                } else {
                    return Err(ErrNo::NotADirectory);
                }
            },
            Err(msg) => return Err(msg),
        }
    } 
}

/// Open a file at dir identified by `fd` and with name `file_name`, with `flags`. Mode is currently unsupported.
pub fn sys_openat(fd: i32, file_name: VirtAddr, flags: u32, _: u32) -> isize {
    let process = current_process().unwrap();

    let buf = process.get_inner_locked().layout.get_user_cstr(file_name);
    let path = match core::str::from_utf8(&buf) {
        Ok(p) => p,
        Err(msg) => {
            error!("sys_openat: {}", msg);
            return -1;
        },
    };
    if path.len() == 0 {
        error!("sys_openat: empty path");
    }

    let mut fs_flags = OpenMode::READ;
    if flags & 0x001 != 0 {
        fs_flags = OpenMode::WRITE;
    }
    if flags & 0x002 != 0 {
        fs_flags |= OpenMode::WRITE;
    }
    if flags & 0x040 != 0 {
        fs_flags |= OpenMode::CREATE;
    }
    verbose!("Openat flag: {:x}", flags);

    match get_file(fd as usize, path, fs_flags) {
        Ok(file) => {
            let mut arcpcb = process.get_inner_locked();
            let new_fd = arcpcb.alloc_fd();
            arcpcb.files[new_fd] = Some(file);
            return new_fd as isize;
        },
        Err(msg) => {
            error!("sys_openat failed with msg \"{}\" on {}", msg, path);
            return -1;
        }
    }
}

/// Close the corresponing fd
pub fn sys_close(fd: usize) -> isize {
    verbose!("Closing fd {}", fd);
    let process = current_process().unwrap();
    let mut arcpcb = process.get_inner_locked();
    
    if fd as usize > arcpcb.files.len() {
        error!("Invalid FD");
        return -1;
    }

    let file = &mut arcpcb.files[fd];
    if file.is_some() {
        file.take();
    } else {
        error!("Invalid FD");
        return -1;
    }

    loop {
        if arcpcb.files.len() == 0 {
            break;
        }
        if arcpcb.files.last().is_none() {
            arcpcb.files.pop();
        } else {
            break;
        }
    }
    verbose!("Fd closed");
    return 0;
}

/// Write to spcific fd.
/// # Returns
/// How many bytes hace been really written to the fd.
pub fn sys_write(fd: usize, buf: VirtAddr, len: usize) -> isize {
    let process = current_process().unwrap();
    let arcpcb = process.get_inner_locked();
    let buf = arcpcb.layout.get_user_buffer(buf, len);
    
    if fd as usize > arcpcb.files.len() {
        error!("Invalid FD");
        return -1;
    }
    if let Some(fd_slot) = arcpcb.files.get(fd) {
        match fd_slot {
            Some(file) => {
                let file = file.clone();
                drop(arcpcb);
                match file.write_user_buffer(buf) {
                    Ok(size) => size as isize,
                    Err(msg) => {
                        error!("Write failed with msg \"{}\"", msg);
                        -1
                    }
                }
            },
            None => {
                error!("No such file descriptor!");
                return -1;
            }
        }
    } else {
        error!("No such file descriptor!");
        return -1;
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct iovec {
    pub iov_base: usize,
    pub iov_len: usize
}

/// Write multiple buffers of data described by iov to the file descriptor
/// # Returns
/// How many bytes hace been really written to the fd.
pub fn sys_writev(fd: usize, iov: VirtAddr, iovcnt: usize) -> isize {
    let process = current_process().unwrap();
    let arcpcb = process.get_inner_locked();
    
    if fd as usize > arcpcb.files.len() {
        error!("Invalid FD");
        return -1;
    }

    let mut ret = 0;
    match &arcpcb.files[fd] {
        Some(file) => {
            let file = file.clone();
            for i in 0..iovcnt {
                let iov_addr = iov + size_of::<iovec>() * i;
                let iov_struct: iovec = arcpcb.layout.read_user_data(iov_addr);
                let buf = arcpcb.layout.get_user_buffer(VirtAddr::from(iov_struct.iov_base), iov_struct.iov_len);
                match file.write_user_buffer(buf) {
                    Ok(size) => { ret += size as isize; },
                    Err(msg) => {
                        error!("Read failed with msg \"{}\"", msg);
                        return -1;
                    }
                }
            }
            drop(arcpcb);
            ret
        },
        None => {
            error!("No such file descriptor!");
            return -1;
        }
    }
}

/// Read from spcific fd.
/// # Returns
/// How many bytes hace been really read from the fd.
pub fn sys_read(fd: usize, buf: VirtAddr, len: usize) -> isize {
    let process = current_process().unwrap();
    let arcpcb = process.get_inner_locked();
    let buf = arcpcb.layout.get_user_buffer(buf, len);
    
    if fd as usize > arcpcb.files.len() {
        error!("Invalid FD");
        return -1;
    }

    if let Some(fd_slot) = arcpcb.files.get(fd) {
        match fd_slot {
            Some(file) => {
                let file = file.clone();
                verbose!("Reading from file: {}", file.poll().name);
                drop(arcpcb);
                match file.read_user_buffer(buf) {
                    Ok(size) => size as isize,
                    Err(msg) => {
                        error!("Read failed with msg \"{}\"", msg);
                        -1
                    }
                }
            },
            None => {
                error!("No such file descriptor!");
                return -1;
            }
        }
    } else {
        error!("No such file descriptor!");
        return -1;
    }
}

/// Read multiple buffers of data described by iov to the file descriptor
/// # Returns
/// How many bytes hace been really read from the fd.
pub fn sys_readv(fd: usize, iov: VirtAddr, iovcnt: usize) -> isize {
    let process = current_process().unwrap();
    let arcpcb = process.get_inner_locked();
    
    if fd as usize > arcpcb.files.len() {
        error!("Invalid FD");
        return -1;
    }

    let mut ret = 0;
    match &arcpcb.files[fd] {
        Some(file) => {
            let file = file.clone();
            for i in 0..iovcnt {
                let iov_addr = iov + size_of::<iovec>() * i;
                let iov_struct: iovec = arcpcb.layout.read_user_data(iov_addr);
                let buf = arcpcb.layout.get_user_buffer(VirtAddr::from(iov_struct.iov_base), iov_struct.iov_len);
                match file.read_user_buffer(buf) {
                    Ok(size) => { ret += size as isize; },
                    Err(msg) => {
                        error!("Read failed with msg \"{}\"", msg);
                        return -1;
                    }
                }
            }
            drop(arcpcb);
            ret
        },
        None => {
            error!("No such file descriptor!");
            return -1;
        }
    }
}

/// Create a pipe, and write the two FDs into the `pipe` array.
pub fn sys_pipe(pipe: VirtAddr) -> isize {
    let process = current_process().unwrap();
    let mut arcpcb = process.get_inner_locked();
    let (read, write) = make_pipe();
    let wd = arcpcb.alloc_fd();
    arcpcb.files[wd] = Some(write);
    let rd = arcpcb.alloc_fd();
    arcpcb.files[rd] = Some(read);
    verbose!("pipe fd: rd {}, wd {}", rd, wd);
    arcpcb.layout.write_user_data(pipe, &(rd as i32));
    arcpcb.layout.write_user_data(pipe + size_of::<i32>(), &(wd as i32));

    0
}

/// Duplicate a file descriptor
pub fn sys_dup(fd: usize) -> isize {
    let process = current_process().unwrap();
    let mut arcpcb = process.get_inner_locked();
    
    if fd as usize > arcpcb.files.len() {
        error!("Invalid FD");
        return -1;
    }

    if let Some(src) = arcpcb.files[fd].clone() {
        let rd = arcpcb.alloc_fd();
        arcpcb.files[rd] = Some(src);
        rd as isize
    } else {
        error!("No such file descriptor.");
        -1
    }
}

/// Duplicate a file descriptor, and place it into a specified fd.
pub fn sys_dup3(old_fd: usize, new_fd: usize, _: usize) -> isize {
    let process = current_process().unwrap();
    let mut arcpcb = process.get_inner_locked();
    
    if old_fd as usize > arcpcb.files.len() {
        error!("Invalid FD");
        return -1;
    }

    if let Some(src) = arcpcb.files[old_fd].clone() {
        if arcpcb.files.len() <= new_fd {
            arcpcb.files.resize(new_fd + 1, None);
        } else if arcpcb.files[new_fd].is_some() {
            arcpcb.files[new_fd].take();
        }
        arcpcb.files[new_fd] = Some(src);
        new_fd as isize
    } else {
        error!("No such file descriptor.");
        -1
    }
}

/// The Linux style dirent struct
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct dirent {
    d_ino: u64,
    d_off: u64,
    d_reclen: u16,
    d_type: u8,
    d_name: [u8; 128],
}

#[repr(u8)]
pub enum POSIXDType {
    UNKNOWN = 0,
    FIFO    = 1,
    CHR     = 2,
    DIR     = 4,
    BLK     = 6,
    REG     = 8,
    LNK     = 10,
    SOCK    = 12,
}

fn ftype2posix(ft: FileType) -> POSIXDType {
    match ft {
        FileType::Unknown     => POSIXDType::UNKNOWN,
        FileType::FIFO        => POSIXDType::FIFO   ,
        FileType::CharDev     => POSIXDType::CHR    ,
        FileType::Directory   => POSIXDType::DIR    ,
        FileType::BlockDev    => POSIXDType::BLK    ,
        FileType::Regular     => POSIXDType::REG    ,
        FileType::Link        => POSIXDType::LNK    ,
        FileType::Sock        => POSIXDType::SOCK   ,
    }
}

/// Get dirents of a directory.
pub fn sys_getdents64(fd: usize, buf: VirtAddr, len: usize) -> isize {
    let process = current_process().unwrap();
    let arcpcb = process.get_inner_locked();
    let mut last_ptr = buf;
    
    if fd as usize > arcpcb.files.len() {
        error!("Invalid FD");
        return -1;
    }
    
    if let Some(file) = arcpcb.files[fd].clone() {
        if let Some(dir) = file.to_dir_file() {
            for f in dir.list() {
                let f_stat = f.poll();
                let mut dirent_item = dirent {
                    // TODO: d_ino
                    d_ino : 0,
                    d_off : size_of::<dirent>().try_into().unwrap(),
                    d_reclen: f_stat.name.len() as u16,
                    d_name: [0; 128],
                    d_type: ftype2posix(f_stat.ftype) as u8,
                };
                verbose!("current file: {:?}", f_stat);
                let name_bytes = f_stat.name.as_bytes();
                dirent_item.d_name[0..name_bytes.len()].copy_from_slice(&name_bytes);
                arcpcb.layout.write_user_data(last_ptr, &dirent_item);
                last_ptr = last_ptr + size_of::<dirent>();
            }
            verbose!("Getdents64 returns {}", (last_ptr - buf));
            (last_ptr - buf) as i32 as isize
        } else {
            error!("Not a directory.");
            -1
        }
    } else {
        error!("No such file descriptor.");
        -1
    }
}

/// just delete the file
pub fn sys_unlink(dirfd: i32, path: VirtAddr, _: usize) -> isize{
    let proc = current_process().unwrap();
    let buf = proc.get_inner_locked().layout.get_user_cstr(path);
    let path = match core::str::from_utf8(&buf) {
        Ok(p) => p,
        Err(msg) => {
            error!("sys_openat: {}", msg);
            return -1;
        },
    };

    match unlink(dirfd as usize, path) {
        Ok(()) => return 0,
        Err(msg) => {
            error!("sys_unlink:{}", msg);
            return -1;
        }
    };
}

pub fn sys_mkdirat(dirfd: usize, path: VirtAddr, _: usize) -> isize {
    verbose!("mkdir start");
    let proc = current_process().unwrap();
    let buf = proc.get_inner_locked().layout.get_user_cstr(path);
    let path = match core::str::from_utf8(&buf) {
        Ok(p) => p,
        Err(_) => {
            error!("sys_openat: {}: invalid path string", ErrNo::InvalidArgument as isize);
            return -(ErrNo::InvalidArgument as isize);
        },
    };
    debug!("mkdir: {}", path);
    match makeDirAt(dirfd as usize, path) {
        Ok(()) => return 0,
        Err(msg) => {
            error!("sys_mkdirat: {}: {}", msg as isize, msg);
            return -(msg as isize);
        },
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct FStat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_mode: u32,
    pub st_nlink: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: u64,
    pub __pad: u32,
    pub st_size: u32,
    pub st_blksize: u32,
    pub __pad2: i32,
    pub st_blocks: u64,
    pub st_atime_sec: u32,
    pub st_atime_nsec: u32,
    pub st_mtime_sec: u32,
    pub st_mtime_nsec: u32,
    pub st_ctime_sec: u32,
    pub st_ctime_nsec: u32,
    pub __unused: [u8; 2],
}


use crate::fs::FileStatus;

fn getFStat(file: &Arc<dyn File>) -> Result<FStat, ErrNo> {
    let f_stat = file.poll();
    let mut linux_mode: u32 = 0;
    linux_mode |= f_stat.ftype as u32;
    linux_mode |= f_stat.mode;
    linux_mode |= if f_stat.readable  {0o444} else {0};
    linux_mode |= if f_stat.writeable {0o222} else {0};
    linux_mode |= 0o111;

    return Ok(FStat {
        st_dev: f_stat.dev_no,
        st_ino: f_stat.inode,
        st_mode: linux_mode,
        st_nlink: 1,
        st_uid: f_stat.uid,
        st_gid: f_stat.gid,
        st_rdev: 0,
        __pad: 0,
        st_size: f_stat.size as u32,
        st_blksize: f_stat.block_sz,
        __pad2: 0,
        st_blocks: f_stat.blocks,
        st_atime_sec:   f_stat.atime_sec,
        st_atime_nsec:  f_stat.atime_nsec,
        st_mtime_sec:   f_stat.mtime_sec,
        st_mtime_nsec:  f_stat.mtime_nsec,
        st_ctime_sec:   f_stat.ctime_sec,
        st_ctime_nsec:  f_stat.ctime_nsec,
        __unused: [0u8; 2],
    })
}

bitflags! {
    pub struct AtFlags: usize{
        const AT_SYMLINK_NOFOLLOW   = 0x100;
        const AT_REMOVEDIR          = 0x200;
        const AT_SYMLINK_FOLLOW     = 0x400;
        const AT_NO_AUTOMOUNT       = 0x800;
        const AT_EMPTY_PATH         = 0x1000;
    }
}

fn fstatat(fd: usize, path: &str, ptr: VirtAddr, flags: AtFlags) -> Result<(), ErrNo> {
    if path.len() == 0 && !flags.contains(AtFlags::AT_EMPTY_PATH) {
        return Err(ErrNo::NoSuchFileOrDirectory);
    }
    let mode = if flags.contains(AtFlags::AT_SYMLINK_NOFOLLOW) {
        OpenMode::NO_FOLLOW
    } else {
        OpenMode::empty()
    };

    match get_file(fd, path, mode) {
        Ok(file) => {
            match getFStat(&file) {
                Ok(stat) => {
                    verbose!("Stat: {:?}", stat);
                    current_process().unwrap()
                        .get_inner_locked()
                        .layout.write_user_data(ptr, &stat);
                    return Ok(());
                },
                Err(msg) => return Err(msg),
            }
        },
        Err(ErrNo::IsADirectory) => {
            match get_file(fd, path, mode | OpenMode::DIR) {
                Ok(file) => {
                    match getFStat(&file) {
                        Ok(stat) => {
                            verbose!("Stat: {:?}", stat);
                            current_process().unwrap()
                                .get_inner_locked()
                                .layout.write_user_data(ptr, &stat);
                            return Ok(());
                        },
                        Err(msg) => return Err(msg),
                    }
                },
                Err(errno) => {
                    return Err(errno);
                },
            }
        },
        Err(errno) => {
            return Err(errno);
        }
    }
}

pub fn sys_fstat(fd: usize, ptr: VirtAddr) -> isize {
    match fstatat(fd, &"", ptr, AtFlags::AT_EMPTY_PATH) {
        Ok(()) => return 0,
        Err(msg) => {
            debug!("sys_fstat: {}", msg);
            return -1;
        }
    }
}


pub fn sys_fstatat(dirfd: usize, path: VirtAddr, ptr: VirtAddr, flags:usize) -> isize{
    let buf = current_process().unwrap().get_inner_locked().layout.get_user_cstr(path);
    let path = match core::str::from_utf8(&buf) {
        Ok(path) => path,
        Err(_) => {
            debug!("sys_fstatat: invalid path string");
            return -1;
        }
    };
    let flags = match AtFlags::from_bits(flags) {
        Some(flags) => flags,
        None => {
            debug!("sys_fstatat: invalid flags");
            return -1;
        },
    };
    debug!("dirfd: {}", dirfd as isize);
    debug!("path: {}", path);
    match fstatat(dirfd, path, ptr, flags) {
        Ok(()) => return 0,
        Err(msg) => {
            debug!("sys_fstatat: {}", msg);
            return -1;
        }
    }
}

pub fn sys_ioctl_inner(fd: usize, request: u64, argp: VirtAddr) -> Result<u64, ErrNo> {
    let proc = current_process().ok_or(ErrNo::NoSuchProcess)?;
    let file = proc.get_inner_locked().files.get(fd).ok_or(ErrNo::BadFileDescriptor)?.clone().ok_or(ErrNo::BadFileDescriptor)?;
    let dev_file = file.to_device_file().ok_or(ErrNo::NotSuchDevice)?;
    dev_file.ioctl(request, argp)
}

pub fn sys_ioctl(fd: usize, request: u64, argp: VirtAddr) -> isize {
    match sys_ioctl_inner(fd, request, argp) {
        Ok(res) => res as isize,
        Err(msg) => {
            error!("IOCTL Failed: {}", msg);
            -1
        }
    }
}

pub fn read_linux_fstat(file: Arc<dyn File>) -> FStat {
    let f_stat = file.poll();
    let mut linux_mode: u32 = 0;
    linux_mode |= f_stat.ftype as u32;
    linux_mode |= f_stat.mode;
    linux_mode |= if f_stat.readable  {0o444} else {0};
    linux_mode |= if f_stat.writeable {0o222} else {0};
    linux_mode |= 0o111;

    FStat {
        st_dev: f_stat.dev_no,
        st_ino: f_stat.inode,
        st_mode: linux_mode,
        st_nlink: 1,
        st_uid: f_stat.uid,
        st_gid: f_stat.gid,
        st_rdev: 0,
        __pad: 0,
        st_size: f_stat.size as u32,
        st_blksize: f_stat.block_sz,
        __pad2: 0,
        st_blocks: f_stat.blocks,
        st_atime_sec:   f_stat.atime_sec,
        st_atime_nsec:  f_stat.atime_nsec,
        st_mtime_sec:   f_stat.mtime_sec,
        st_mtime_nsec:  f_stat.mtime_nsec,
        st_ctime_sec:   f_stat.ctime_sec,
        st_ctime_nsec:  f_stat.ctime_nsec,
        __unused: [0u8; 2],
    }
}

pub fn sys_fstatat_new(fd: i32, path: VirtAddr, ptr: VirtAddr, flags:usize) -> isize {
    let flags = AtFlags::from_bits_truncate(flags);
    let process = current_process().unwrap();
    let arcpcb = process.get_inner_locked();
    let mut buf = arcpcb.layout.get_user_cstr(path);
    buf = buf[..buf.len() - 1].to_vec(); // remove \0
    if buf.len() > 1 && buf[0] == b'/' && buf[1] == b'/' {
        buf = buf[2..].to_vec();
    }
    let mut fs_flags = OpenMode::SYS;
    if flags.contains(AtFlags::AT_SYMLINK_NOFOLLOW) {
        fs_flags |= OpenMode::NO_FOLLOW;
    }

    if let Ok(mut path) = core::str::from_utf8(&buf) {
        verbose!("Path: {}", path);
        if path.starts_with("/") {
            if let Ok(file) = open(path.to_string(), OpenMode::SYS) {
                arcpcb.layout.write_user_data(ptr, &(read_linux_fstat(file)));
                return 0;
            }
            return -1;
        } else if flags.contains(AtFlags::AT_EMPTY_PATH) {
            if let Some(slot) = arcpcb.files.get(fd as usize) {
                if let Some(file) = slot {
                    arcpcb.layout.write_user_data(ptr, &(read_linux_fstat(file.clone())));
                    return 0;
                }
                return -1;
            }
            return -1;
        } else if fd == AT_FDCWD {
            if path.starts_with("./") {
                path = path.get(2..).unwrap();
            }
            if path.starts_with(".") {
                path = path.get(1..).unwrap();
            }
            let mut whole_path = arcpcb.path.clone();
            whole_path.push_str(path);
            verbose!("FSTATAT path: {} + {}", arcpcb.path.clone(), path);
            let file = open(whole_path.to_string(), fs_flags);
            let file = match file {
                Ok(f) => f,
                Err(e) => {
                    error!("error: {}", e);
                    return -1;
                },
            };
            arcpcb.layout.write_user_data(ptr, &(read_linux_fstat(file.clone())));
            return 0;
        }
    }

    return -1;
}

pub const SEND_FILE_CHUNK_SZ: usize = 4096;

fn sys_sendfile_wrapper(write_fd: usize, read_fd: usize, offset_ptr: VirtAddr, mut count: usize) -> Result<usize, ErrNo> {
    let proc = current_process().unwrap();
    let locked_inner = proc.get_inner_locked();

    let mut result: usize = 0;
    let write_file = locked_inner.files.get(write_fd).ok_or(ErrNo::BadFileDescriptor)?.clone().ok_or(ErrNo::BadFileDescriptor)?;
    let read_file = locked_inner.files.get(read_fd).ok_or(ErrNo::BadFileDescriptor)?.clone().ok_or(ErrNo::BadFileDescriptor)?;

    if offset_ptr.0 != 0 {
        let offset: u32 = locked_inner.layout.read_user_data(offset_ptr);
        read_file.seek(offset as isize, fs::SeekOp::SET)?;
    }

    drop(locked_inner);
    drop(proc);

    verbose!("Sending from {} to {}, initial offset @ {}", read_file.poll().name, write_file.poll().name, read_file.get_cursor()?);

    count = _core::cmp::min(read_file.poll().size as usize - read_file.get_cursor()? as usize, count);

    while count > 0 {
        let mut move_sz = _core::cmp::min(count, SEND_FILE_CHUNK_SZ);
        let mut buf: Vec<u8> = Vec::with_capacity(move_sz);
        buf.resize(move_sz, 0);
        verbose!("Trying to send {} bytes", move_sz);
        loop {
            move_sz = read_file.read(&mut buf)?;
            if move_sz != 0 {
                break;
            } else {
                suspend_switch();
            }
        }
        buf = buf[..move_sz].to_vec();
        let mut write_sz_left = move_sz;
        while write_sz_left > 0 {
            let write_sz = write_file.write(&mut buf)?;
            buf = buf[..write_sz].to_vec();
            write_sz_left -= write_sz;
        }
        count -= move_sz;
        result += move_sz;
        verbose!("Sended {} bytes, {} remaining", move_sz, count);
    }

    let proc = current_process().unwrap();
    let locked_inner = proc.get_inner_locked();
    
    if offset_ptr.0 != 0 {
        let final_offset = read_file.get_cursor()? as i32;
        locked_inner.layout.write_user_data(offset_ptr, &final_offset);
    }

    Ok(result)
}

pub fn sys_sendfile(out_fd: usize, in_fd: usize, offset_ptr: VirtAddr, count: usize) -> isize {
    match sys_sendfile_wrapper(out_fd, in_fd, offset_ptr, count) {
        Ok(res) => res as isize,
        Err(msg) => {
            error!("Send file failed: {}", msg);
            -1
        }
    }
}

pub fn sys_readlinkat(dirfd: usize, path: VirtAddr, buf: VirtAddr, bufsize: usize) -> isize {
    let proc = current_process().unwrap();
    let pbuf = proc.get_inner_locked().layout.get_user_cstr(path);
    let path = match core::str::from_utf8(&pbuf) {
        Ok(p) => p,
        Err(msg) => {
            error!("sys_readlinkat: {}", msg);
            return -1;
        },
    };

    debug!("sys_readlinkat: {}", path);

    let file = match get_file(dirfd, path, OpenMode::READ | OpenMode::NO_FOLLOW) {
        Ok(f) => f,
        Err(msg) => {
            error!("sys_readlinkat: {}", msg);
            return -1;
        }
    };

    if file.poll().ftype != FileType::Link {
        error!("sys_readlinkat: file not link");
        return -1;
    }

    let buf = proc.get_inner_locked().layout.get_user_buffer(buf, bufsize);
    match file.read_user_buffer(buf){
        Ok(size) => return size as isize,
        Err(msg) => {
            error!("sys_readlikat: {}", msg);
            return -1;
        }
    };
}

// TODO: implement this.
pub fn sys_ppoll() -> isize {
    0
}