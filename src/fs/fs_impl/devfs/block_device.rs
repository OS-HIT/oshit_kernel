use core::{cell::Cell, sync::atomic::{AtomicUsize, Ordering}};

use crate::fs::{CommonFile, DirFile, File, file::FileStatus};
use alloc::{string::ToString, sync::Arc, vec::Vec};
use super::{DeviceFile, device_file::BlockDeviceFile};
use crate::drivers::BLOCK_DEVICE;
use lazy_static::*;

lazy_static! {
	pub static ref SDA_WRAPPER: Arc<SDAWrapper> = Arc::new(SDAWrapper::new());
}

pub struct SDAWrapper {
	pub cursor: AtomicUsize,
	pub blk_sz: u64
}

impl SDAWrapper {
	pub fn new() -> Self {
		Self {
			cursor: AtomicUsize::new(0),
			blk_sz: 512
		}
	}
}

impl BlockDeviceFile for SDAWrapper {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        BLOCK_DEVICE.read_block(block_id, buf)
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        BLOCK_DEVICE.write_block(block_id, buf)
    }

    fn clear_block(&self, block_id: usize) {
        BLOCK_DEVICE.clear_block(block_id)
    }
}

impl DeviceFile for SDAWrapper {
    fn ioctl(&self, op: u64) -> Result<u64, &'static str> {
        todo!()
    }
}

impl File for SDAWrapper {
    fn seek(&self, offset: isize, op: crate::fs::SeekOp) -> Result<(), &'static str> {
        match op {
			crate::fs::SeekOp::CUR => {
				if offset % (self.blk_sz as isize) == 0 {
                    if offset > 0 {
                        self.cursor.fetch_add(offset as usize, Ordering::Relaxed);
                    } else {
                        self.cursor.fetch_sub((-offset) as usize, Ordering::Relaxed);
                    }
					Ok(())
				} else {
					Err("Seek not aligned.")
				}
			},
            crate::fs::SeekOp::SET => {
				if offset % (self.blk_sz as isize) == 0 {
					self.cursor.store(offset as usize, Ordering::Relaxed);
					Ok(())
				} else {
					Err("Seek not aligned.")
				}
			},
            crate::fs::SeekOp::END => Err("Cannot seek to end of Block device"),
		}
    }

    fn get_cursor(&self) -> Result<usize, &'static str> {
        Ok(self.cursor.load(Ordering::Relaxed))
    }

    fn read(&self, buffer: &mut [u8]) -> Result<usize, &'static str> {
        let mut offset = 0;
		while buffer.len() - offset > self.blk_sz as usize{
			let mut rd_buf = Vec::<u8>::new();
			rd_buf.resize(self.blk_sz as usize, 0);
			self.read_block(offset / self.blk_sz as usize, &mut rd_buf);
			buffer[offset..(offset + self.blk_sz as usize)].copy_from_slice(&rd_buf);
			offset += self.blk_sz as usize;
		}
		Ok(offset)
    }

    fn write(&self, buffer: &[u8]) -> Result<usize, &'static str> {
        let mut offset = 0;
		while buffer.len() - offset > self.blk_sz as usize{
			self.write_block(offset / self.blk_sz as usize, &buffer[offset..(offset+self.blk_sz as usize)]);
			offset += self.blk_sz as usize;
		}
		Ok(offset)
    }

    fn read_user_buffer(&self, mut buffer: crate::memory::UserBuffer) -> Result<usize, &'static str> {
		let mut offset = 0;
		while buffer.len() - offset > self.blk_sz as usize{
			let mut rd_buf = Vec::<u8>::new();
			rd_buf.resize(self.blk_sz as usize, 0);
			self.read_block(offset / self.blk_sz as usize, &mut rd_buf);
			
			for i in offset..(offset + self.blk_sz as usize) {
				buffer[i] = rd_buf[i - offset];
			}
			
			offset += self.blk_sz as usize;
		}
		Ok(offset)
    }

    fn write_user_buffer(&self, buffer: crate::memory::UserBuffer) -> Result<usize, &'static str> {
		let mut offset = 0;
		while buffer.len() - offset > self.blk_sz as usize{
			let mut wr_buf = Vec::new();
			for i in 0..self.blk_sz as usize{
				wr_buf.push(buffer[offset + i]);
			}
			self.write_block(offset / self.blk_sz as usize, &wr_buf);
			offset += self.blk_sz as usize;
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
            readable: true,
            writeable: true,
            size: BLOCK_DEVICE.block_cnt() * self.blk_sz,
            name: "sda".to_string(),
            ftype: crate::fs::file::FileType::BlockDev,
            inode: 0,
            dev_no: 0,
            mode: 0,
            block_sz: self.blk_sz as u32,
            blocks: BLOCK_DEVICE.block_cnt(),
            uid: 0,
            gid: 0,
            atime_sec: 0,
            atime_nsec:0,
            mtime_sec: 0,
            mtime_nsec:0,
            ctime_sec: 0,
            ctime_nsec:0,
        }
    }

    fn rename(&self, new_name: &str) -> Result<(), &'static str> {
        Err("Cannot rename block device")
    }

    fn get_vfs(&self) -> Result<alloc::sync::Arc<dyn crate::fs::VirtualFileSystem>, &'static str> {
        Ok(super::DEV_FS.clone())
    }

    fn get_path(&self) -> alloc::string::String {
        "/block/sda".to_string()
    }
}

impl Drop for SDAWrapper {
    fn drop(&mut self) {
        panic!("SDA wrapper dropped? what happened?")
    }
}