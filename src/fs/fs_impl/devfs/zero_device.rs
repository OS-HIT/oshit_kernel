use crate::fs::{CommonFile, DirFile};
use crate::fs::file::{FileStatus, FileType};
use crate::fs::SeekOp;
use super::DeviceFile;
use super::super::super::File;
use super::super::super::Path;
use alloc::sync::Arc;
use alloc::string::String;
use alloc::string::ToString;
use lazy_static::*;

use crate::memory::UserBuffer;

pub struct FZero {}

lazy_static! {
	pub static ref FILE_ZERO: Arc<FZero> = Arc::new(FZero{});
}

impl Drop for FZero {
        fn drop(&mut self) {}
}

impl File for FZero {
        fn seek(&self, offset: isize, op: SeekOp) -> Result<(), &'static str> {
                return Ok(());
        }

        fn get_cursor(&self) -> Result<usize, &'static str> {
                return Ok(0);
        }

        /// read to buffers
        /// return length read on success
        fn read(&self, buffer: &mut [u8]) -> Result<usize, &'static str> {
                for i in 0..buffer.len() {
                        buffer[i] = 0;
                }
                return Ok(buffer.len());
        }

        /// write from buffers
        /// return length written on success
        fn write(&self, buffer: &[u8]) -> Result<usize, &'static str> {
                return Ok(0);
        }

        /// read to buffers
        /// return length read on success
        fn read_user_buffer(&self, mut buffer: UserBuffer) -> Result<usize, &'static str> {
                let tmp = [0u8;512];
                let mut left = buffer.len();
                let mut off = 0;
                while left >= 512 {
                        buffer.write(off, &tmp);
                        off += 512;
                        left -= 512;
                }
                while left > 0 {
                        buffer.write(off, &tmp[0]);
                        off += 1;
                        left -= 1;
                }
                return Ok(buffer.len());
        }

        /// write from buffers
        /// return length written on success
        fn write_user_buffer(&self, buffer: UserBuffer) -> Result<usize, &'static str> {
                return Ok(0);
        }

        /// cast down to common file
        /// HACK: It is unclear how this will coop with Arc<File>, recommand no holding this but Arc<File>.
        /// return casted on success
        fn to_common_file<'a>(self: Arc<Self>) -> Option<Arc<dyn CommonFile + 'a>> where Self: 'a {
                return Some(self);
        }

        /// cast down to common file
        /// HACK: It is unclear how this will coop with Arc<File>, recommand no holding this but Arc<File>.
        /// return casted on success
        fn to_dir_file<'a>(self: Arc<Self>) -> Option<Arc<dyn DirFile + 'a>> where Self: 'a {
                return None;
        }

        /// cast down to device file
        /// HACK: It is unclear how this will coop with Arc<File>, recommand no holding this but Arc<File>.
        /// return casted on success
        fn to_device_file<'a>(self: Arc<Self>) -> Option<Arc<dyn DeviceFile + 'a>> where Self: 'a {
                return None;
        }

        /// Get file status
        fn poll(&self) -> FileStatus {
                FileStatus {
			readable: 	true,
                        writeable: 	true,
                        size: 		0,
                        name: 		"zero".to_string(),
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
                return Err("renaming zero file is not allowed");
        }

        fn get_vfs(&self) -> Result<Arc<(dyn crate::fs::VirtualFileSystem + 'static)>, &'static str> {
                Ok(super::DEV_FS.clone())
        }

        fn get_path(&self) -> Path {
                let path = vec![String::from("zero")];
                return Path {path, must_dir: false, is_abs: true}; 
        }
} 

impl CommonFile for FZero {}