//! Wrappers of file system related syscalls.
// use super::super::fs::file::FILE;
// use crate::fs::block_cache::BLOCK_SZ;
use crate::fs::{self, File, OpenMode, make_pipe, mkdir, open, remove};
use crate::memory::{VirtAddr};
use crate::process::{current_process};
use alloc::string::ToString;
// use alloc::vec::Vec;
use alloc::sync::Arc;
use core::{convert::TryInto, mem::size_of};

/// The special "file descriptor" indicating that the path is relative path to process's current working directory. 
pub const AT_FDCWD: i32 =  -100;

/// Open a file at dir identified by `fd` and with name `file_name`, with `flags`. Mode is currently unsupported.
pub fn sys_openat(fd: i32, file_name: VirtAddr, flags: u32, _: u32) -> isize {
    let process = current_process().unwrap();
    let mut arcpcb = process.get_inner_locked();
    let mut buf = arcpcb.layout.get_user_cstr(file_name);
    if buf[0] == b'.' && buf[1] == b'/' {
        buf = buf[2..].iter().cloned().collect();
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
    if let Ok(path) = core::str::from_utf8(&buf) {
        debug!("Openat path: {}", path);
        if fd == AT_FDCWD {
            verbose!("openat found AT_FDCWD!");
            let mut whole_path = arcpcb.path.clone();
            whole_path.push_str(path);
            verbose!("Openat path: {} + {}", arcpcb.path.clone(), path);
 

            let res = open(path.to_string(), fs_flags);
            let file = match res {
                Ok(f) => f,
                Err(e) => return -1,
            };
            let new_fd = arcpcb.alloc_fd();
            arcpcb.files[new_fd] = Some(file);
            return new_fd.try_into().unwrap();
        }

        if fd as usize > arcpcb.files.len() {
            error!("Invalid FD");
            return -1;
        }

        if let Some(dir) = arcpcb.files[fd as usize].clone() {
            if let Some(dir_file) = dir.to_dir_file() {
                if let Ok(new_file) = dir_file.open(path.to_string(), fs_flags) {
                    let new_fd = arcpcb.alloc_fd();
                    arcpcb.files[new_fd] = Some(new_file);
                    verbose!("Openat success");
                    new_fd as isize
                } else {
                    error!("Cannot open such file");
                    -1
                }
            } else {
                error!("Not a directory!");
                -1
            }
        } else {
            error!("No such fd");
            return -1;
        }
    } else {
        error!("Invalid UTF-8 sequence in path");
        -1
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
    d_off: i64,
    d_reclen: u16,
    d_type: u8,
    d_name: [u8; 128],
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
                    d_off : size_of::<dirent> as i64,
                    d_reclen: f_stat.name.len() as u16,
                    d_type: f_stat.ftype as u8,
                    d_name: [0; 128],   // How to do this?
                };
                dirent_item.d_name[0..f_stat.name.as_bytes().len()].copy_from_slice(&f_stat.name.as_bytes());
                arcpcb.layout.write_user_data(last_ptr, &dirent_item);
                last_ptr = buf + size_of::<dirent>();
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
pub fn sys_unlink(dirfd: i32, file_name: VirtAddr, _: usize) -> isize{
    let proc = current_process().unwrap();
    let arcpcb = proc.get_inner_locked();
    let buf = arcpcb.layout.get_user_cstr(file_name);
    if let Ok(mut path) = core::str::from_utf8(&buf) {
        if dirfd == AT_FDCWD {
            verbose!("sys_unlink found AT_FDCWD!");
            let mut whole_path = arcpcb.path.clone();
            if buf[0] == b'.' && buf[1] == b'/' {
                path = &path[2..];
            }
            whole_path.push_str(path);
            verbose!("Deleting at_fdcwd path {}", whole_path);
            match remove(whole_path) {
                Ok(_) => return 0,
                Err(msg) => {
                    error!("{}", msg);
                    return -1;
                }
            };
        }

        if buf[0] == b'/' {
            match remove(path.to_string()) {
                Ok(_) => return 0,
                Err(msg) => {
                    error!("{}", msg);
                    return -1;
                }
            };
        }

        if let Some(dir) = arcpcb.files[dirfd as usize].clone() {
            if let Some(dir_file) = dir.to_dir_file() {
                verbose!("Deleting file at {}", path);
                match dir_file.remove(path.to_string()) {
                    Ok(_) => return 0,
                    Err(msg) => {
                        error!("{}", msg);
                        return -1;
                    }
                }
            }
        }
    }
    return -1;
}

pub fn sys_mkdirat(dirfd: usize, path: VirtAddr, _: usize) -> isize {
    verbose!("mkdir start");
    let proc = current_process().unwrap();
    let arcpcb = proc.get_inner_locked();
    let buf = arcpcb.layout.get_user_cstr(path);
    if let Ok(path) = core::str::from_utf8(&buf) {
        if dirfd as i32 == AT_FDCWD {
            verbose!("sys_mkdirat found AT_FDCWD!");
            let mut whole_path = arcpcb.path.clone();
            whole_path.push_str(path);
            verbose!("Whole path = {}", whole_path);

            match mkdir(whole_path) {
                Ok(_) => return 0,
                Err(msg) => {
                    error!("{}", msg);
                    return -1;
                }
            };
        }

        if buf[0] == b'/' {
            match mkdir(path.to_string()) {
                Ok(_) => return 0,
                Err(msg) => {
                    error!("{}", msg);
                    return -1;
                }
            };
        }

        if let Some(dir) = arcpcb.files[dirfd as usize].clone() {
            if let Some(dir_file) = dir.to_dir_file() {
                match dir_file.mkdir(path.to_string()) {
                    Ok(_) => return 0,
                    Err(msg) => {
                        error!("{}", msg);
                        return -1;
                    }
                }
                
            }
        }
    }
    -1
}

#[repr(C)]
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

pub fn sys_fstat(fd: usize, ptr: VirtAddr) -> isize {
    let proc = current_process().unwrap();
    let arcpcb = proc.get_inner_locked();
    if let Some(op_file) = arcpcb.files.get(fd) {
        if let Some(file) = op_file.clone() {
            if let Some(fs_file) = file.to_common_file() {
                let f_stat = fs_file.poll();
                let stat = FStat {
                    st_dev: f_stat.dev_no,
                    st_ino: f_stat.inode,
                    st_mode: f_stat.mode,
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
                };
                arcpcb.layout.write_user_data(ptr, &stat);
                return 0; 
            }
        }
    }
    -1
}
