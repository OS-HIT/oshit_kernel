use super::super::fs::file::FILE;
use crate::fs::VirtFile;
use crate::memory::{VirtAddr};
use crate::process::{current_process};
// use alloc::vec::Vec;
use alloc::sync::Arc;
use core::convert::TryInto;

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

pub fn sys_open(path: VirtAddr, mode: u32) -> isize {
    let process = current_process().unwrap();
    let mut arcpcb = process.get_inner_locked();
    let buf = arcpcb.layout.get_user_bytes(path);

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

pub fn sys_close(fd: usize) -> isize {
    let process = current_process().unwrap();
    let mut arcpcb = process.get_inner_locked();
    let file = &mut arcpcb.files[fd];
    if file.is_some() {
        *file = None;
    } else {
        error!("Invalid FD")
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
    return 0;
}

pub fn sys_write(fd: usize, buf: VirtAddr, len: usize) -> isize {
    let process = current_process().unwrap();
    let arcpcb = process.get_inner_locked();
    let buf = arcpcb.layout.get_user_buffer(buf, len);
    match &arcpcb.files[fd] {
        Some(file) => return file.write(buf),
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
        Some(file) => return file.read(buf),
        None => {
            error!("No such file descriptor!");
            return -1;
        }
    }
}