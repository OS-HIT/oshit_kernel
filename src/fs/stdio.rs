use core::convert::TryInto;

use super::File;
use crate::memory::UserBuffer;
use alloc::vec::Vec;
use crate::process::suspend_switch;
use crate::sbi::{
    get_byte
};
use spin::Mutex;

pub struct Stdin;
pub struct Stdout;
pub struct Stderr;

impl File for Stdin {
    fn read(&self, mut buf: UserBuffer) -> isize {
        let mut i: usize = 0;
        loop {
            let c = get_byte();
            if c == 0 {
                suspend_switch();
            } else {
                buf[i] = c;
                i += 1;
                if i < buf.len() - 1 && c == b'\n' {     // TODO: check if this actually complys with syscall spec
                    buf[i] = b'\0';
                    return i.try_into().unwrap();
                }

                if i >= buf.len() {
                    return i.try_into().unwrap();
                }
            }
        }
    }

    fn write(&self, _: UserBuffer) -> isize {
        panic!("Cannot write to STDIN!");
    }
}

impl File for Stdout {
    fn read(&self, _: UserBuffer) -> isize {
        panic!("Cannot read from STDOUT!");
    }

    fn write(&self, buf: UserBuffer) -> isize {
        let mut s: Vec<u8> = Vec::new();
        for i in buf.into_iter() {
            s.push(i);
        }
        print!("{}", core::str::from_utf8(&s).unwrap());
        buf.len().try_into().unwrap()
    }
}

impl File for Stderr {
    fn read(&self, _: UserBuffer) -> isize {
        panic!("Cannot read from STDOUT!");
    }

    fn write(&self, buf: UserBuffer) -> isize {
        let mut s: Vec<u8> = Vec::new();
        for i in buf.into_iter() {
            s.push(i);
        }
        print!("\033[91m{}\033[0m", core::str::from_utf8(&s).unwrap());
        buf.len().try_into().unwrap()
    }
}