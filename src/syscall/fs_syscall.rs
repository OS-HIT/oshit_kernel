use super::super::fs::file::FILE;
use crate::{fs::{VirtFile, make_pipe}, memory::translate_user_va};
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

pub fn sys_close(fd: usize) -> isize {
    verbose!("Closing fd");
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
    let rd = arcpcb.alloc_fd();
    arcpcb.files[rd] = Some(read);
    let wd = arcpcb.alloc_fd();
    arcpcb.files[wd] = Some(write);
    
    arcpcb.layout.write_user_data(pipe, &rd);
    arcpcb.layout.write_user_data(pipe + size_of::<usize>(), &wd);

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