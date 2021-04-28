use super::super::fs::{FD_STDERR, FD_STDOUT};
use super::super::sbi::{LogLevel, set_log_color, reset_color};
use crate::memory::{get_user_data, VirtAddr};
use crate::process::get_current_satp;

pub fn sys_write(fd: usize, buf: VirtAddr, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let buffers = get_user_data(get_current_satp(), buf, len);
            for buffer in buffers {
                print!("{}", core::str::from_utf8(buffer).unwrap());    // FIXME: there are chances that codepoint not aligned and break into different pages. find a way to concentrate then out put it.
            }
            len as isize
        },
        FD_STDERR => {
            set_log_color(LogLevel::Error);
            let buffers = get_user_data(get_current_satp(), buf, len);
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