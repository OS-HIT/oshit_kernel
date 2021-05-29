//! Wrappers of file system related syscalls.
use super::super::fs::file::FILE;
use crate::fs::block_cache::BLOCK_SZ;
use crate::fs::{self, FTYPE, FileWithLock, VirtFile, make_pipe};
use crate::memory::{VirtAddr};
use crate::process::{current_process};
// use alloc::vec::Vec;
use alloc::sync::Arc;
use core::{convert::TryInto, mem::size_of};
use alloc::string::ToString;

/// The special "file descriptor" indicating that the path is relative path to process's current working directory. 
pub const AT_FDCWD: i32 =  -100;

/// Open a file at dir identified by `fd` and with name `file_name`, with `flags`. Mode is currently unsupported.
pub fn sys_openat(fd: i32, file_name: VirtAddr, flags: u32, _: u32) -> isize {
    let process = current_process().unwrap();
    let mut arcpcb = process.get_inner_locked();
    let mut buf = arcpcb.layout.get_user_cstr(file_name);

    if buf[0] == b'.' && buf[1] == b'/' {
        buf.rotate_left(2);
    }

    if let Ok(path) = core::str::from_utf8(&buf) {
        if fd == AT_FDCWD {
            verbose!("openat found AT_FDCWD!");
            let mut whole_path = arcpcb.path.clone();
            whole_path.push_str(path);
            let fs_flags: u32 = match flags {
                0x000 => FILE::FMOD_READ,
                0x001 => FILE::FMOD_WRITE,
                0x002 => FILE::FMOD_READ | FILE::FMOD_WRITE,
                0x040 => FILE::FMOD_CREATE,
                0x041 => FILE::FMOD_CREATE | FILE::FMOD_WRITE,
                0x042 => FILE::FMOD_READ | FILE::FMOD_WRITE | FILE::FMOD_CREATE,
                // 0x0200000 => FILE::FMOD_READ,
                _ => {
                    error!("Not supported combination： {:x}", flags);
                    return -1;
                }
            };
            let file = match FILE::open_file(whole_path.as_str(), fs_flags) {
                Ok(file) => file,
                Err(msg) => {
                    error!("{}", msg);
                    return -1;
                }
            };
        
            let new_fd = arcpcb.alloc_fd();
            arcpcb.files[new_fd] = Some(Arc::new(FileWithLock::new(file)));
            return new_fd.try_into().unwrap();
        }

        if fd as usize > arcpcb.files.len() {
            error!("Invalid FD");
            return -1;
        }

        if let Some(dir) = arcpcb.files[fd as usize].clone() {
            if let Ok(dir_file) = dir.to_fs_file_locked() {
                if dir_file.ftype == fs::FTYPE::TDir {
                    match dir_file.open_file_from(path, flags) {
                        Ok(fs_file) => {
                            let new_fd = arcpcb.alloc_fd();
                            arcpcb.files[new_fd] = Some(Arc::new(FileWithLock::new(fs_file)));
                            return new_fd as isize;
                        }
                        Err(err_msg) => {
                            error!("{}", err_msg);
                            return -1;
                        }
                    }
                } else {
                    error!("Not a directory!");
                    -1
                }
            } else {
                error!("Not a file!");
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

    match &arcpcb.files[fd] {
        Some(file) => {
            let file = file.clone();
            drop(arcpcb);
            return file.write(buf);
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

    match &arcpcb.files[fd] {
        Some(file) => {
            let file = file.clone();
            drop(arcpcb);
            return file.read(buf);
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
    d_name: [u8; 256],
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
    
    if let Some(file) = &arcpcb.files[fd] {
        if let Ok(mut dir) = file.to_fs_file_locked() {
            loop{
                match dir.get_dirent() {
                    Ok((dirent, name)) => {
                        if last_ptr - buf > len {
                            error!("Memory out of bound");
                            return -1;
                        }

                        let mut dirent_item = dirent {
                            // TODO: d_ino
                            d_ino : 0,
                            d_off : size_of::<dirent> as i64,
                            d_reclen: dirent.get_name().len() as u16,
                            d_type: dirent.attr,
                            d_name: [0; 256],   // How to do this?
                        };
                        dirent_item.d_name[0..dirent.name.len()].copy_from_slice(&dirent.name);
                        arcpcb.layout.write_user_data(last_ptr, &dirent_item);
                        last_ptr = buf + size_of::<dirent>();
                    },
                    Err(_) => {
                        break;
                    }
                }
            }
            (last_ptr - buf) as isize
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
    if let Ok(path) = core::str::from_utf8(&buf) {
        if dirfd == AT_FDCWD {
            verbose!("sys_unlink found AT_FDCWD!");
            let mut whole_path = arcpcb.path.clone();
            whole_path.push_str(path);

            match FILE::delete_dir(whole_path.as_str()) {
                Ok(_) => return 0,
                Err(msg) => {
                    error!("{}", msg);
                    return -1;
                }
            };
        }

        if buf[0] == b'/' {
            match FILE::delete_dir(path) {
                Ok(_) => return 0,
                Err(msg) => {
                    error!("{}", msg);
                    return -1;
                }
            };
        }

        if let Some(dir) = &arcpcb.files[dirfd as usize] {
            if let Ok(dir_file) = dir.to_fs_file_locked() {
                if dir_file.ftype == FTYPE::TDir {
                    match dir_file.delete_file_from(path) {
                        Ok(_) => return 0,
                        Err(msg) => {
                            error!("{}", msg);
                            return -1;
                        }
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

            match FILE::make_dir(whole_path.as_str()) {
                Ok(_) => return 0,
                Err(msg) => {
                    error!("{}", msg);
                    return -1;
                }
            };
        }

        if buf[0] == b'/' {
            match FILE::make_dir(path) {
                Ok(_) => return 0,
                Err(msg) => {
                    error!("{}", msg);
                    return -1;
                }
            };
        }

        if let Some(dir) = &arcpcb.files[dirfd as usize] {
            if let Ok(dir_file) = dir.to_fs_file_locked() {
                if dir_file.ftype == FTYPE::TDir {
                    match dir_file.make_dir_from(path) {
                        Ok(_) => return 0,
                        Err(msg) => {
                            error!("{}", msg);
                            return -1;
                        }
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
        if let Some(file) = op_file {
            if let Ok(fs_file) = file.to_fs_file_locked() {
                let stat = FStat {
                    st_dev: 0,
                    st_ino: 0,
                    st_mode: 0,
                    st_nlink: 0,
                    st_uid: 0,
                    st_gid: 0,
                    st_rdev: 0,
                    __pad: 0,
                    st_size: fs_file.fsize,
                    st_blksize: BLOCK_SZ as u32,
                    __pad2: 0,
                    st_blocks: fs_file.fsize as u64 / BLOCK_SZ as u64,
                    st_atime_sec: 0,
                    st_atime_nsec: 0,
                    st_mtime_sec: 0,
                    st_mtime_nsec: 0,
                    st_ctime_sec: 0,
                    st_ctime_nsec: 0,
                    __unused: [0u8; 2],
                };
                arcpcb.layout.write_user_data(ptr, &stat);
                return 0; 
            }
        }
    }
    -1
}