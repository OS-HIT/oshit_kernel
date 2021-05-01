use super::super::fs::{FD_STDERR, FD_STDOUT, FD_STDIN};
use super::super::sbi::{LogLevel, set_log_color, reset_color, get_byte};
use crate::memory::{get_user_data, write_user_data, VirtAddr};
use crate::process::{current_satp, suspend_switch};
use alloc::vec::Vec;

pub fn sys_write(fd: usize, buf: VirtAddr, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let buffers = get_user_data(current_satp(), buf, len);
            for buffer in buffers {
                print!("{}", core::str::from_utf8(buffer).unwrap());    // FIXME: there are chances that codepoint not aligned and break into different pages. find a way to concentrate then out put it.
            }
            len as isize
        },
        FD_STDERR => {
            set_log_color(LogLevel::Error);
            let buffers = get_user_data(current_satp(), buf, len);
            for buffer in buffers {
                print!("{}", core::str::from_utf8(buffer).unwrap());    // FIXME: (same as above)
            }
            reset_color();
            len as isize
        },
        _ => {
            panic!("Unsupported fd in sys_write!");
        }
    }
}

pub fn sys_read(fd: usize, buf: VirtAddr, len: usize) -> isize {
    match fd {
        FD_STDIN => {
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
        _ => panic!("Unsupported fd in sys_read!")
    }
}