use super::super::fs::file::FILE;
use crate::{fs::{self, File, VirtFile, make_pipe}, memory::translate_user_va};
use crate::memory::{VirtAddr};
use crate::process::{current_process};
// use alloc::vec::Vec;
use alloc::sync::Arc;
use core::{convert::TryInto, mem::size_of};

// #[inline]
// fn find_free_fd() -> Option<usize> {
//     let arcproc = current_process().unwrap();
//     for i in 0..arcproc.get_inner_locked().files.len() {
//         if arcproc.get_inner_locked().files[i].ftype == FTYPE::TFree {
//             return Some(i);
//         }
//     }
//     return None;
// }

// TODO: This does not comply with oscomp spec. Change it.
pub fn sys_open(path: VirtAddr, mode: u32) -> isize {
    let process = current_process().unwrap();
    let mut arcpcb = process.get_inner_locked();
    let buf = arcpcb.layout.get_user_cstr(path);

    let file = match FILE::open_file(core::str::from_utf8(&buf).unwrap(), mode) {
        Ok(file) => file,
        Err(msg) => {
            error!("{}", msg);
            return -1;
        }
    };

    let fd = arcpcb.alloc_fd();
    arcpcb.files[fd] = Some(Arc::new(VirtFile::new(file)));
    return fd.try_into().unwrap();
}

pub fn sys_openat(fd: usize, file_name: VirtAddr, flags: u32, mode: u32) -> isize {
    let process = current_process().unwrap();
    let mut arcpcb = process.get_inner_locked();
    let buf = arcpcb.layout.get_user_cstr(file_name);
    if let Some(dir) = &arcpcb.files[fd] {
        if let Ok(dir_file) = dir.to_fs_file_locked() {
            if dir_file.ftype == fs::FTYPE::TDir {
                // TODO: What to do?
                0
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
}

pub fn sys_close(fd: usize) -> isize {
    verbose!("Closing fd {}", fd);
    let process = current_process().unwrap();
    let mut arcpcb = process.get_inner_locked();
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

pub fn sys_write(fd: usize, buf: VirtAddr, len: usize) -> isize {
    let process = current_process().unwrap();
    let arcpcb = process.get_inner_locked();
    let buf = arcpcb.layout.get_user_buffer(buf, len);
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

pub fn sys_read(fd: usize, buf: VirtAddr, len: usize) -> isize {
    let process = current_process().unwrap();
    let arcpcb = process.get_inner_locked();
    let buf = arcpcb.layout.get_user_buffer(buf, len);
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

pub fn sys_dup(fd: usize) -> isize {
    let process = current_process().unwrap();
    let mut arcpcb = process.get_inner_locked();
    if let Some(src) = arcpcb.files[fd].clone() {
        let rd = arcpcb.alloc_fd();
        arcpcb.files[rd] = Some(src);
        rd as isize
    } else {
        error!("No such file descriptor.");
        -1
    }
}

pub fn sys_dup3(old_fd: usize, new_fd: usize, _: usize) -> isize {
    let process = current_process().unwrap();
    let mut arcpcb = process.get_inner_locked();
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

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct dirent {
    d_ino: u64,
    d_off: i64,
    d_reclen: u16,
    d_type: u8,
    d_name: [u8; 256],
}

pub fn sys_getdents64(fd: usize, buf: VirtAddr, len: usize) -> isize {
    let process = current_process().unwrap();
    let arcpcb = process.get_inner_locked();
    let mut last_ptr = buf;
    if let Some(file) = &arcpcb.files[fd] {
        if let Ok(mut dir) = file.to_fs_file_locked() {
            loop{
                match dir.get_dirent() {
                    Ok(dirent) => {
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
