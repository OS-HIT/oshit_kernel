use core::convert::TryInto;

use super::File;
use crate::memory::UserBuffer;
use alloc::vec::Vec;
use crate::process::suspend_switch;
use crate::sbi::{
    get_byte
};
use spin::Mutex;
use alloc::sync::Arc;
use lazy_static::*;

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

    fn to_fs_file_locked(&self) -> Result<spin::MutexGuard<super::FILE>, &str> {
        Err("STDIO is not directory")
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

    fn to_fs_file_locked(&self) -> Result<spin::MutexGuard<super::FILE>, &str> {
        Err("STDIO is not directory")
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

    fn to_fs_file_locked(&self) -> Result<spin::MutexGuard<super::FILE>, &str> {
        Err("STDIO is not directory")
    }
}

pub struct LockedStdin {
    pub inner: Mutex<Stdin>
}
pub struct LockedStdout {
    pub inner: Mutex<Stdout>
}
pub struct LockedStderr {
    pub inner: Mutex<Stderr>
}

impl File for LockedStdin {
    fn read(&self, buf: UserBuffer) -> isize {
        self.inner.lock().read(buf)
    }

    fn write(&self, buf: UserBuffer) -> isize {
        self.inner.lock().write(buf)
    }

    fn to_fs_file_locked(&self) -> Result<spin::MutexGuard<super::FILE>, &str> {
        Err("STDIO is not directory")
    }
}

impl File for LockedStdout {
    fn read(&self, buf: UserBuffer) -> isize {
        self.inner.lock().read(buf)
    }

    fn write(&self, buf: UserBuffer) -> isize {
        self.inner.lock().write(buf)
    }

    fn to_fs_file_locked(&self) -> Result<spin::MutexGuard<super::FILE>, &str> {
        Err("STDIO is not directory")
    }
}

impl File for LockedStderr {
    fn read(&self, buf: UserBuffer) -> isize {
        self.inner.lock().read(buf)
    }

    fn write(&self, buf: UserBuffer) -> isize {
        self.inner.lock().write(buf)
    }

    fn to_fs_file_locked(&self) -> Result<spin::MutexGuard<super::FILE>, &str> {
        Err("STDIO is not directory")
    }
}

lazy_static!{
    pub static ref STDIN: Arc<LockedStdin> = Arc::new(LockedStdin{inner: Mutex::new(Stdin)});
    pub static ref STDOUT: Arc<LockedStdout> = Arc::new(LockedStdout{inner: Mutex::new(Stdout)});
    pub static ref STDERR: Arc<LockedStderr> = Arc::new(LockedStderr{inner: Mutex::new(Stderr)});
}