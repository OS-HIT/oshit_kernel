use core::{cell::Cell, sync::atomic::{AtomicUsize, Ordering}};

use crate::fs::Path;
use crate::{fs::{CommonFile, DirFile, File, SeekOp, file::FileStatus}, memory::VirtAddr};
use alloc::{string::ToString, sync::Arc, vec::Vec};
use alloc::string::String;
use super::{CharDeviceFile, DeviceFile, device_file::BlockDeviceFile};
use crate::drivers::BLOCK_DEVICE;
use lazy_static::*;
use crate::process::ErrNo;

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
    fn ioctl(&self, op: u64, argp: VirtAddr) -> Result<u64, ErrNo> {
        warning!("IOCTL logged for /block/sda: op={}", op);
        Err(ErrNo::PermissionDenied)
    }

    fn to_char_dev<'a>(self: Arc<Self>) -> Option<Arc<dyn CharDeviceFile + 'a>> where Self: 'a  {
        None
    }

    fn to_blk_dev<'a>(self: Arc<Self>) -> Option<Arc<dyn BlockDeviceFile + 'a>> where Self: 'a  {
        Some(self)
    }
}

impl File for SDAWrapper {
    fn seek(&self, offset: isize, op: crate::fs::SeekOp) -> Result<(), ErrNo> {
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
					Err(ErrNo::IllegalSeek)
				}
			},
            crate::fs::SeekOp::SET => {
				if offset % (self.blk_sz as isize) == 0 {
					self.cursor.store(offset as usize, Ordering::Relaxed);
					Ok(())
				} else {
					Err(ErrNo::IllegalSeek)
				}
			},
            crate::fs::SeekOp::END =>
                Err(ErrNo::IllegalSeek)
		}
    }

    fn get_cursor(&self) -> Result<usize, ErrNo> {
        Ok(self.cursor.load(Ordering::Relaxed))
    }

    fn read(&self, buffer: &mut [u8]) -> Result<usize, ErrNo> {
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

    fn write(&self, buffer: &[u8]) -> Result<usize, ErrNo> {
        let mut offset = 0;
		while buffer.len() - offset > self.blk_sz as usize{
			self.write_block(offset / self.blk_sz as usize, &buffer[offset..(offset+self.blk_sz as usize)]);
			offset += self.blk_sz as usize;
		}
		Ok(offset)
    }

    fn read_user_buffer(&self, mut buffer: crate::memory::UserBuffer) -> Result<usize, ErrNo> {
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

    fn write_user_buffer(&self, buffer: crate::memory::UserBuffer) -> Result<usize, ErrNo> {
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

    fn to_common_file<'a>(self: Arc<Self>) -> Option<Arc<dyn CommonFile + 'a>> where Self: 'a {
        None
    }

    fn to_dir_file<'a>(self: Arc<Self>) -> Option<Arc<dyn DirFile + 'a>> where Self: 'a {
        None
    }

    fn to_device_file<'a>(self: Arc<Self>) -> Option<Arc<dyn DeviceFile + 'a>> where Self: 'a {
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

    fn rename(&self, new_name: &str) -> Result<(), ErrNo> {
        Err(ErrNo::PermissionDenied)
    }

    fn get_vfs(&self) -> Result<alloc::sync::Arc<dyn crate::fs::VirtualFileSystem>, ErrNo> {
        Ok(super::DEV_FS.clone())
    }

    fn get_path(&self) -> Path {
        let path = vec![String::from("block"),String::from("sda")];
        return Path {path, must_dir: false, is_abs: true};
    }
}

impl Drop for SDAWrapper {
    fn drop(&mut self) {
        panic!("SDA wrapper dropped? what happened?")
    }
}

pub struct CommonFileAsBlockDevice {
    inner: Arc<dyn File>,
    blk_sz: usize
}

impl CommonFileAsBlockDevice {
    pub fn new(file: Arc<dyn File>, blk_sz: usize) -> Self {
        if blk_sz & (blk_sz - 1) != 0 {
            panic!("Block size must be power of 2!")
        }

        Self {
            inner: file,
            blk_sz
        }
    }
}

impl Drop for CommonFileAsBlockDevice {
    fn drop(&mut self) {
        // auto drop
    }
}

impl File for CommonFileAsBlockDevice {
    fn seek(&self, offset: isize, op: crate::fs::SeekOp) -> Result<(), ErrNo> {
        self.inner.seek(offset, op)
    }

    fn get_cursor(&self) -> Result<usize, ErrNo> {
        self.inner.get_cursor()
    }

    fn read(&self, buffer: &mut [u8]) -> Result<usize, ErrNo> {
        self.inner.read(buffer)
    }

    fn write(&self, buffer: &[u8]) -> Result<usize, ErrNo> {
        self.inner.write(buffer)
    }

    fn read_user_buffer(&self, buffer: crate::memory::UserBuffer) -> Result<usize, ErrNo> {
        self.inner.read_user_buffer(buffer)
    }

    fn write_user_buffer(&self, buffer: crate::memory::UserBuffer) -> Result<usize, ErrNo> {
        self.inner.write_user_buffer(buffer)
    }

    fn to_common_file<'a>(self: Arc<Self>) -> Option<Arc<dyn CommonFile + 'a>> where Self: 'a {
        None
    }

    fn to_dir_file<'a>(self: Arc<Self>) -> Option<Arc<dyn DirFile + 'a>> where Self: 'a {
        None
    }

    fn to_device_file<'a>(self: Arc<Self>) -> Option<Arc<dyn DeviceFile + 'a>> where Self: 'a {
        Some(self)
    }

    fn poll(&self) -> FileStatus {
        self.inner.poll()
    }

    fn rename(&self, new_name: &str) -> Result<(), ErrNo> {
        self.inner.rename(new_name)
    }

    fn get_vfs(&self) -> Result<Arc<dyn crate::fs::VirtualFileSystem>, ErrNo> {
        self.inner.get_vfs()
    }

    fn get_path(&self) -> Path {
        self.inner.get_path()
    }
}

impl DeviceFile for CommonFileAsBlockDevice {
    fn ioctl(&self, op: u64, argp: VirtAddr) -> Result<u64, ErrNo> {
        Err(ErrNo::PermissionDenied)
    }

    fn to_char_dev<'a>(self: Arc<Self>) -> Option<Arc<dyn CharDeviceFile + 'a>> where Self: 'a  {
        None
    }

    fn to_blk_dev<'a>(self: Arc<Self>) -> Option<Arc<dyn BlockDeviceFile + 'a>> where Self: 'a  {
        Some(self)
    }
}

impl BlockDeviceFile for CommonFileAsBlockDevice {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        assert_eq!(buf.len(), self.blk_sz, "Buffer size != blk_sz!");
        self.seek((self.blk_sz * block_id) as isize, SeekOp::SET);
        self.read(buf);
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        assert_eq!(buf.len(), self.blk_sz, "Buffer size != blk_sz!");
        self.seek((self.blk_sz * block_id) as isize, SeekOp::SET);
        self.write(buf);
    }

    fn clear_block(&self, block_id: usize) {
        self.seek((self.blk_sz * block_id) as isize, SeekOp::SET);
        let mut v: Vec<u8> = Vec::new();
        v.resize(self.blk_sz, 0);
        self.write(&v);
    }
}