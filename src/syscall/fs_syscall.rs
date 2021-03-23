use super::super::fs::{FD_STDERR, FD_STDOUT};
use super::super::sbi::{LogLevel, set_log_color, reset_color};

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    match fd {
        FD_STDOUT => {
            let slice = unsafe { core::slice::from_raw_parts(buf, len) };
            let str = core::str::from_utf8(slice).unwrap();
            print!("{}", str);
            len as isize
        },
        FD_STDERR => {
            let slice = unsafe { core::slice::from_raw_parts(buf, len) };
            let str = core::str::from_utf8(slice).unwrap();
            set_log_color(LogLevel::Error);
            print!("{}", str);
            reset_color();
            len as isize
        },
        _ => {
            panic!("Unsupported fd in sys_write!");
        }
    }
}