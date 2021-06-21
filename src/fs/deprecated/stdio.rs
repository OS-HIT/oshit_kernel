//! The VirtFile implementation for STDIOs

use core::convert::TryInto;

use super::VirtFile;
use crate::memory::UserBuffer;
use alloc::vec::Vec;
use crate::process::suspend_switch;
use crate::sbi::{
    get_byte
};
use spin::Mutex;
use alloc::sync::Arc;
use lazy_static::*;

/// A zero length struct representing Stdin.
pub struct Stdin;
/// A zero length struct representing Stdout.
pub struct Stdout;
/// A zero length struct representing Stderr.
pub struct Stderr;

impl VirtFile for Stdin {
    /// read from stdin
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

    /// always fail, for you cannot write to stdin.
    fn write(&self, _: UserBuffer) -> isize {
        error!("Cannot write to STDIN!");
        -1
    }

    /// always fail, for the concret type is not FileWithLock
    fn to_fs_file_locked(&self) -> Result<spin::MutexGuard<super::FILE>, &str> {
        Err("STDIO is not FS File")
    }
}

impl VirtFile for Stdout {
    /// always fail, for you cannot read from stdout
    fn read(&self, _: UserBuffer) -> isize {
        error!("Cannot read from STDOUT!");
        -1
    }

    /// write to stdout
    fn write(&self, buf: UserBuffer) -> isize {
        let mut s: Vec<u8> = Vec::new();
        for i in buf.into_iter() {
            s.push(i);
        }
        print!("{}", core::str::from_utf8(&s).unwrap());
        buf.len().try_into().unwrap()
    }

    /// always fail, for the concret type is not FileWithLock
    fn to_fs_file_locked(&self) -> Result<spin::MutexGuard<super::FILE>, &str> {
        Err("STDIO is not FS File")
    }
}

impl VirtFile for Stderr {
    /// always fail, for you cannot read from stderr
    fn read(&self, _: UserBuffer) -> isize {
        error!("Cannot read from STDOUT!");
        -1
    }

    /// write to stderr. Will be in bright red.
    fn write(&self, buf: UserBuffer) -> isize {
        let mut s: Vec<u8> = Vec::new();
        for i in buf.into_iter() {
            s.push(i);
        }
        print!("\033[91m{}\033[0m", core::str::from_utf8(&s).unwrap());
        buf.len().try_into().unwrap()
    }

    /// always fail, for the concret type is not FileWithLock
    fn to_fs_file_locked(&self) -> Result<spin::MutexGuard<super::FILE>, &str> {
        Err("STDIO is not FS File")
    }
}

/// Locked version of stdin.
pub struct LockedStdin {
    pub inner: Mutex<Stdin>
}

/// Locked version of stdout.
pub struct LockedStdout {
    pub inner: Mutex<Stdout>
}

/// Locked version of stderr.
pub struct LockedStderr {
    pub inner: Mutex<Stderr>
}

impl VirtFile for LockedStdin {
    /// locked read from stdin. Give up on failed to lock, let other finish first.
    fn read(&self, buf: UserBuffer) -> isize {
        loop {
            if let Some(inner) = self.inner.try_lock() {
                return inner.read(buf);
            } else {
                suspend_switch();
            }
        }
    }

    /// locked write.
    fn write(&self, buf: UserBuffer) -> isize {
        loop {
            if let Some(inner) = self.inner.try_lock() {
                return inner.write(buf);
            } else {
                suspend_switch();
            }
        }
    }

    /// always fail, for the concret type is not FileWithLock
    fn to_fs_file_locked(&self) -> Result<spin::MutexGuard<super::FILE>, &str> {
        Err("STDIO is not FS File")
    }
}

impl VirtFile for LockedStdout {
    /// locked read
    fn read(&self, buf: UserBuffer) -> isize {
        loop {
            if let Some(inner) = self.inner.try_lock() {
                return inner.read(buf);
            } else {
                suspend_switch();
            }
        }
    }

    /// locked write to stdout. Give up on failed to lock, let other finish first.
    fn write(&self, buf: UserBuffer) -> isize {
        loop {
            if let Some(inner) = self.inner.try_lock() {
                return inner.write(buf);
            } else {
                suspend_switch();
            }
        }
    }

    /// always fail, for the concret type is not FileWithLock
    fn to_fs_file_locked(&self) -> Result<spin::MutexGuard<super::FILE>, &str> {
        Err("STDIO is not FS File")
    }
}

impl VirtFile for LockedStderr {
    /// locked read
    fn read(&self, buf: UserBuffer) -> isize {
        loop {
            if let Some(inner) = self.inner.try_lock() {
                return inner.read(buf);
            } else {
                suspend_switch();
            }
        }
    }

    /// locked write to stderr. Give up on failed to lock, let other finish first.
    fn write(&self, buf: UserBuffer) -> isize {
        loop {
            if let Some(inner) = self.inner.try_lock() {
                return inner.write(buf);
            } else {
                suspend_switch();
            }
        }
    }

    /// always fail, for the concret type is not FileWithLock
    fn to_fs_file_locked(&self) -> Result<spin::MutexGuard<super::FILE>, &str> {
        Err("STDIO is not FS File")
    }
}

lazy_static!{
    /// Singleton of locked version stdin.
    pub static ref LOCKED_STDIN: Arc<LockedStdin> = Arc::new(LockedStdin{inner: Mutex::new(Stdin)});
    /// Singleton of locked version stdout.
    pub static ref LOCKED_STDOUT: Arc<LockedStdout> = Arc::new(LockedStdout{inner: Mutex::new(Stdout)});
    /// Singleton of locked version stderr.
    pub static ref LOCKED_STDERR: Arc<LockedStderr> = Arc::new(LockedStderr{inner: Mutex::new(Stderr)});
}