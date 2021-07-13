use super::{CharDevice, DeviceFile};
use super::super::super::File;
use alloc::string::ToString;
use lazy_static::*;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use crate::fs::file::{FileStatus, FileType};
use crate::sbi::get_byte_non_block_with_echo;
use crate::sbi::put_byte;
use core::usize;
use core::convert::TryInto;

lazy_static! {
	pub static ref TTY0: Arc<SBITTY> = Arc::new(SBITTY::new());
}

const LF: u8 = b'\n';

pub struct SBITTY {
	buffer_size: usize,
	read_buffer: VecDeque<u8>,
	write_buffer: VecDeque<u8>,
}

impl SBITTY {
	pub fn new() -> Self {
		Self {
			buffer_size: 4096,
			read_buffer: VecDeque::new(),
			write_buffer: VecDeque::new()
		}
	}
}

impl Drop for SBITTY {
    fn drop(&mut self) {
        // Do nothing?
    }
}

impl File for SBITTY {
    fn seek(&self, offset: u64, op: crate::fs::SeekOp) -> Result<(), &'static str> {
        Err("Cannot seek a Char Device.")
    }

	// TODO: implement smarter flush timing, and some how intergrate this.
    fn read(&self, buffer: &[u8]) -> Result<u64, &'static str> {
		let offset = 0;
		while offset < buffer.len() {
			self.flush();
			while !self.read_buffer.is_empty() {
				let b = self.read_buffer.pop_front().unwrap();
				buffer[offset] = b;
				offset += 1;
				// return instantly on LF
				if b == LF {
					return Ok(offset as u64);
				}
			}
		}
		Ok(offset as u64) 
    }

    fn read_user_buffer(&self, buffer: crate::memory::UserBuffer) -> Result<u64, &'static str> {
		let offset = 0;
		while offset < buffer.len() {
			self.flush();
			while !self.read_buffer.is_empty() {
				let b = self.read_buffer.pop_front().unwrap();
				buffer[offset] = b;
				offset += 1;
				// return instantly on LF
				if b == LF {
					return Ok(offset as u64);
				}
			}
		}
		Ok(offset as u64) 
    }

	// TODO: implement smarter flush timing
    fn write(&self, buffer: &[u8]) -> Result<u64, &'static str> {
        let offset = 0;
		while offset < buffer.len() {
			self.flush();
			while self.write_buffer.len() < self.buffer_size as usize {
				self.write_buffer.push_back(buffer[offset]);
			}
		}
		Ok(offset as u64)
    }

    fn write_user_buffer(&self, buffer: crate::memory::UserBuffer) -> Result<u64, &'static str> {
        let offset = 0;
		while offset < buffer.len() {
			self.flush();
			while self.write_buffer.len() < self.buffer_size as usize {
				self.write_buffer.push_back(buffer[offset]);
			}
		}
		Ok(offset as u64)
    }

    fn to_common_file(&self) -> Option<alloc::sync::Arc<dyn crate::fs::CommonFile>> {
        None
    }

    fn to_dir_file(&self) -> Option<alloc::sync::Arc<dyn crate::fs::DirFile>> {
        None
    }

    fn to_device_file(&self) -> Option<alloc::sync::Arc<dyn DeviceFile>> {
        Some(Arc::new(*self))
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

    fn rename(&self, new_name: alloc::string::String) -> Result<(), &'static str> {
        Err("Cannot rename tty")
    }

    fn get_vfs(&self) -> alloc::sync::Arc<dyn crate::fs::VirtualFileSystem> {
        super::DEV_FS.clone()
    }

    fn get_path(&self) -> alloc::string::String {
     	"/tty0".to_string()
    }

    fn get_cursor(&self) -> Result<u64, &'static str> {
        Err("Char device has no cursor!")
    }
}

impl DeviceFile for SBITTY {
    fn ioctl(&self, op: u64) -> Result<u64, &'static str> {
        todo!()
    }
}

impl CharDevice for SBITTY {
    fn flush(&self) {
		let value = get_byte_non_block_with_echo();
        while value != 0xFFFFFFFFFFFFFFFF && self.read_buffer.len() < self.buffer_size {
			self.read_buffer.push_back(value.try_into().unwrap());
		}
		while !self.write_buffer.is_empty() {
			put_byte(self.write_buffer.pop_front().unwrap());
		}
    }
}