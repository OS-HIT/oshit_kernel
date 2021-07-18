use super::{CharDeviceFile, DeviceFile};
use super::super::super::File;
use alloc::string::ToString;
use lazy_static::*;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use spin::Mutex;
use crate::fs::{CommonFile, DirFile};
use crate::fs::file::{FileStatus, FileType};
use crate::sbi::get_byte_non_block_with_echo;
use crate::sbi::put_byte;
use core::cell::RefCell;
use core::usize;
use core::convert::TryInto;

lazy_static! {
	pub static ref TTY0: Arc<SBITTY> = Arc::new(SBITTY::new());
}

const LF: u8 = b'\n';

pub struct SBITTY {
	buffer_size: usize,
	inner: Mutex<TTYInner>
}

struct TTYInner {
	read_buffer: VecDeque<u8>,
	write_buffer: VecDeque<u8>,
}

impl SBITTY {
	pub fn new() -> Self {
		Self {
			buffer_size: 4096,
			inner: Mutex::new(
				TTYInner {
					read_buffer: VecDeque::new(),
					write_buffer: VecDeque::new(),
				}
			)
		}
	}
}

impl Drop for SBITTY {
    fn drop(&mut self) {
        // do nothing
    }
}

impl File for SBITTY {
    fn seek(&self, offset: isize, op: crate::fs::SeekOp) -> Result<(), &'static str> {
        Err("Cannot seek a Char Device.")
    }

	// TODO: implement smarter flush timing, and some how intergrate this.
    fn read(&self, buffer: &mut [u8]) -> Result<usize, &'static str> {
		let mut offset = 0;
		while offset < buffer.len() {
			self.flush();
			let mut inner_locked = self.inner.lock();
			while !inner_locked.read_buffer.is_empty() {
				let b = inner_locked.read_buffer.pop_front().unwrap();
				buffer[offset] = b;
				offset += 1;
				// return instantly on LF
				if b == LF {
					return Ok(offset);
				}
			}
		}
		Ok(offset) 
    }

    fn read_user_buffer(&self, mut buffer: crate::memory::UserBuffer) -> Result<usize, &'static str> {
		let mut offset = 0;
		while offset < buffer.len() {
			self.flush();
			let mut inner_locked = self.inner.lock();
			while !inner_locked.read_buffer.is_empty() {
				let b = inner_locked.read_buffer.pop_front().unwrap();
				buffer[offset] = b;
				offset += 1;
				// return instantly on LF
				if b == LF {
					return Ok(offset);
				}
			}
		}
		Ok(offset) 
    }

	// TODO: implement smarter flush timing
    fn write(&self, buffer: &[u8]) -> Result<usize, &'static str> {
        let offset = 0;
		while offset < buffer.len() {
			self.flush();
			let mut inner_locked = self.inner.lock();
			while inner_locked.write_buffer.len() < self.buffer_size as usize {
				inner_locked.write_buffer.push_back(buffer[offset]);
			}
		}
		Ok(offset)
    }

    fn write_user_buffer(&self, buffer: crate::memory::UserBuffer) -> Result<usize, &'static str> {
        let offset = 0;
		while offset < buffer.len() {
			self.flush();
			let mut inner_locked = self.inner.lock();
			while inner_locked.write_buffer.len() < self.buffer_size as usize {
				inner_locked.write_buffer.push_back(buffer[offset]);
			}
		}
		Ok(offset)
    }

    fn to_common_file(&self) -> Option<&dyn CommonFile> {
        None
    }

    fn to_dir_file(&self) -> Option<&dyn DirFile> {
        None
    }

    fn to_device_file(&self) -> Option<&dyn DeviceFile> {
        Some(self)
    }

    fn poll(&self) -> crate::fs::file::FileStatus {
        FileStatus {
			readable: 	true,
            writeable: 	true,
            size: 		0,
            name: 		"tty0".to_string(),
            ftype: 		FileType::CharDev,
            inode: 		0,
            dev_no: 	0,
            mode: 		0,	// TODO: check impl
            block_sz: 	0,
            blocks: 	0,
            uid: 		0,
            gid: 		0,
            atime_sec: 	0,
            atime_nsec:	0,
            mtime_sec: 	0,
            mtime_nsec:	0,
            ctime_sec: 	0,
            ctime_nsec:	0,
		}
    }

    fn rename(&self, new_name: &str) -> Result<(), &'static str> {
        Err("Cannot rename tty")
    }

    fn get_vfs(&self) -> Result<Arc<(dyn crate::fs::VirtualFileSystem + 'static)>, &'static str> {
        Ok(super::DEV_FS.clone())
    }

    fn get_path(&self) -> alloc::string::String {
     	"/tty0".to_string()
    }

    fn get_cursor(&self) -> Result<usize, &'static str> {
        Err("Char device has no cursor!")
    }
}

impl DeviceFile for SBITTY {
    fn ioctl(&self, op: u64) -> Result<u64, &'static str> {
        todo!()
		// TODO: Check tty's ioctl
		
    }
}

impl CharDeviceFile for SBITTY {
    fn flush(&self) {
		let value = get_byte_non_block_with_echo();
		let mut inner_locked = self.inner.lock();
        while value != 0xFFFFFFFFFFFFFFFF && inner_locked.read_buffer.len() < self.buffer_size {
			inner_locked.read_buffer.push_back(value.try_into().unwrap());
		}
		while !inner_locked.write_buffer.is_empty() {
			put_byte(inner_locked.write_buffer.pop_front().unwrap());
		}
    }
}