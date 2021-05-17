use super::super::fs::file::{FTYPE, FILE};
use super::super::sbi::{LogLevel, set_log_color, reset_color, get_byte};
use crate::memory::{get_user_data, write_user_data, VirtAddr};
use crate::process::{current_satp, suspend_switch, current_process};
use alloc::vec::Vec;
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

pub fn sys_open(path: &str, mode: u32) -> isize {
    let file = match FILE::open_file(path, mode) {
        Ok(file) => file,
        Err(msg) => {
            error!("{}", msg);
            return -1;
        }
    };

    let mut arcproc = current_process().unwrap();
    let mut arcpcb = arcproc.get_inner_locked();
    let fd = arcpcb.alloc_fd();
    arcpcb.files[fd] = Some(Arc::new(file));
    return fd.try_into().unwrap();

    // match find_free_fd() {
    //     Some(fd) => {
    //         arcproc.get_inner_locked().files[fd] = file;
    //         return fd as isize;
    //     },
    //     None => {
    //         let fd = arcproc.get_inner_locked().files.len() as isize;
    //         arcproc.get_inner_locked().files.push(file);
    //         return fd;
    //     }
    // }
}

pub fn sys_close(fd: usize) -> isize {
    let mut arcproc = current_process().unwrap();
    if fd >= arcproc.get_inner_locked().files.len() {
        error!("sys_close: invalid fd");
        return -1;
    }

    let mut file = arcproc.get_inner_locked().files[fd].clone();
    match file.ftype {
        FTYPE::TFile => {
            arcproc.get_inner_locked().files[fd].ftype = FTYPE::TFree;
            if let Err((_f, msg)) = file.close_file() {
                error!("{}", msg);
                panic!("what now?");
            }
        },
        FTYPE::TFree => {
            error!("sys_close: invalid fd");
            return -1;
        },
        _ => {
            arcproc.get_inner_locked().files[fd].ftype = FTYPE::TFree;
        }
    }

    loop {
        let len = arcproc.get_inner_locked().files.len();
        if len == 0 {
            break;
        }
        if arcproc.get_inner_locked().files[len - 1].ftype == FTYPE::TFree {
            arcproc.get_inner_locked().files.pop().unwrap();
        } else {
            break;
        }
    }
    return 0;
}

pub fn sys_write(fd: usize, buf: VirtAddr, len: usize) -> isize {
    let cp = current_process().unwrap();
    let ftype = cp.get_inner_locked().files[fd].ftype;
    match ftype {
        FTYPE::TStdOut => {
            let buffers = get_user_data(current_satp(), buf, len);
            for buffer in buffers {
                print!("{}", core::str::from_utf8(buffer).unwrap());    // FIXME: there are chances that codepoint not aligned and break into different pages. find a way to concentrate then out put it.
            }
            len as isize
        },
        FTYPE::TStdErr => {
            set_log_color(LogLevel::Error);
            let buffers = get_user_data(current_satp(), buf, len);
            for buffer in buffers {
                print!("{}", core::str::from_utf8(buffer).unwrap());    // FIXME: (same as above)
            }
            reset_color();
            len as isize
        },
        FTYPE::TFile => {
            let buffers = get_user_data(current_satp(), buf, len);
            let mut len: isize = 0;
            for buffer in buffers {
                if let Ok(l) = cp.get_inner_locked().files[fd].write_file(buffer) {
                    len += l as isize;
                } else {
                    len = -1;
                    break;
                }
            }
            len
        },
        _ => {
            panic!("Unsupported file type in sys_write!");
        }
    }
}

pub fn sys_read(fd: usize, buf: VirtAddr, len: usize) -> isize {
    let cp = current_process().unwrap();
    let ftype = cp.get_inner_locked().files[fd].ftype;
    match ftype {
        FTYPE::TStdIn => {
            let mut res: Vec<u8> = Vec::new();
            loop {
                let c = get_byte();
                if c == 0 {
                    suspend_switch();
                } else {
                    res.push(c);
                    if res.len() >= len - 1 || c == b'\n' {     // TODO: check if this actually complys with syscall spec
                        res.push(b'\0');
                        write_user_data(current_satp(), &res, buf, res.len());
                        return 1;
                    }
                }
            }
        },
        FTYPE::TFile => {
            let mut buffer = [0u8; 512];
            let mut rest = len;
            let mut read: usize = 0;
            
            while rest > 0 {
                let rlen = if rest > buffer.len() {
                    buffer.len()
                } else {
                    rest
                };
                let mut rbuf = &mut buffer[0..rlen];
                match cp.get_inner_locked().files[fd].read_file(rbuf) {
                    Ok(l) => {
                        write_user_data(current_satp(), &rbuf, buf, rlen);
                        read += l as usize;
                        if l < rlen as u32 {
                            break;
                        }
                    },
                    Err(msg) => {
                        error!("{}", msg);
                        break;
                    }
                } 
            }
            read as isize
        },
        _ => panic!("Unsupported file type in sys_read!")
    }
}