use alloc::{sync::Arc, vec::Vec};
use alloc::string::String;
use spin::Mutex;
use crate::fs::{CommonFile, DeviceFile, DirFile, File};
use crate::fs::{file::FileStatus, fs_impl::cache_mgr::BLOCK_SZ};
use crate::fs::VirtualFileSystem;
use crate::fs::fs_impl::vfs::OpenMode;

use super::file::FileInner;
use super::file;

pub struct FAT32File {
	inner: Mutex<FileInner>
}

unsafe impl Sync for FAT32File {}

impl Drop for FAT32File {
	fn drop(&mut self) {
		self.inner.lock().close();
	}
}

impl File for FAT32File {
    fn seek(&self, offset: isize, op: crate::fs::SeekOp) -> Result<(), &'static str> {
        self.inner.lock().seek(offset, op)
    }

    fn get_cursor(&self) -> Result<usize, &'static str> {
        self.inner.lock().get_cursor()
    }

    fn read(&self, buffer: &mut [u8]) -> Result<usize, &'static str> {
        self.inner.lock().read(buffer)
    }

    fn write(&self, buffer: &[u8]) -> Result<usize, &'static str> {
        self.inner.lock().write(buffer)
    }

    fn read_user_buffer(&self, mut buffer: crate::memory::UserBuffer) -> Result<usize, &'static str> {
        let mut temp_arr: Vec<u8> = Vec::new();
		temp_arr.resize(buffer.len(), 0);
		let res = self.inner.lock().read(&mut temp_arr);
		buffer.write_bytes(&temp_arr, 0);
		res
    }

    fn write_user_buffer(&self, buffer: crate::memory::UserBuffer) -> Result<usize, &'static str> {
        let mut temp_arr = buffer.clone_bytes();
		self.inner.lock().write(&mut temp_arr)
    }

    fn to_common_file(&self) -> Option<&dyn CommonFile> {
        Some(self)
    }

    fn to_dir_file(&self) -> Option<&dyn DirFile> {
        if self.inner.lock().is_dir() {
            return Some(self);
        } else {
            return None;
        }
    }

    fn to_device_file(&self) -> Option<&dyn DeviceFile> {
		None	
    }

    fn poll(&self) -> crate::fs::file::FileStatus {
        let inner = self.inner.lock();
		FileStatus {
			readable: inner.readable(),
			writeable: inner.writable(),
			size: inner.size() as u64,
			name: inner.name(),
			ftype: inner.ftype(),
			// TODO: inode number
			inode: 0,
			dev_no: 0,
			mode: inner.fmode() as u32,
			block_sz: BLOCK_SZ as u32,
			blocks: (inner.size() / BLOCK_SZ) as u64,
			uid: 0,
			gid: 0,
			atime_sec: inner.last_acc_time_sec() as u32,
			atime_nsec: 0,
			mtime_sec: inner.create_time_sec() as u32,
			mtime_nsec: inner.create_time_nsec() as u32,
			ctime_sec: inner.create_time_sec() as u32,
			ctime_nsec: inner.create_time_nsec() as u32,
		}
    }

    fn rename(&self, new_name: &str) -> Result<(), &'static str> {
        self.inner.lock().rename(new_name)
    }

    fn get_vfs(&self) -> Result<Arc<dyn crate::fs::VirtualFileSystem>, &'static str> {
        todo!()
    }

    fn get_path(&self) -> alloc::string::String {
        self.inner.lock().get_path()
    }
}

impl CommonFile for FAT32File {}

fn OpenMode2usize(mode: OpenMode) -> usize {
    let mut result:usize = 0;
    if mode.contains(OpenMode::READ) {
        result |= file::READ;
    }
    if mode.contains(OpenMode::WRITE) {
        result |= file::WRITE;
    }
    if mode.contains(OpenMode::CREATE) {
        result |= file::CREATE;
    }
    if mode.contains(OpenMode::DIR) {
        result |= file::DIR;
    }
    if mode.contains(OpenMode::NO_FOLLOW) {
        result |= file::NO_FOLLOW;
    }
    return result;
}

impl DirFile for FAT32File {
        /// open files under dir
        fn open(&self, path: String, mode: OpenMode) -> Result<Arc<dyn File>, &'static str> {
            let mode = OpenMode2usize(mode);
            match self.inner.lock().open(&path, mode) {
                Ok(fin) => Ok( Arc::new(
                    FAT32File {
                        inner: Mutex::new(fin),
                    }
                )),
                Err(msg) => Err(msg),
            }
        }

        /// mkdir. remember to sanitize name.
        fn mkdir(&self, name: String) -> Result<Arc<dyn File>, &'static str> {
            match self.inner.lock().mkdir(&name) {
                Ok(dir) => Ok( Arc::new(
                    FAT32File {
                        inner: Mutex::new(dir),
                    }
                )),
                Err(msg) => Err(msg),
            }
        }
    
        /// make file. remember to sanitize name.
        fn mkfile(&self, name: String) -> Result<Arc<dyn File>, &'static str> {
            match self.inner.lock().mkfile(&name) {
                Ok(file) => Ok( Arc::new(
                    FAT32File {
                        inner: Mutex::new(file),
                    }
                )),
                Err(msg) => Err(msg),
            }
        }
    
        /// delete
        fn remove(&self, path: String) -> Result<(), &'static str> {
            self.inner.lock().remove(&path)
        }

        /// list
        fn list(&self) -> Vec<Arc<dyn File>> {
            let result = Vec::<Arc<dyn File>>::new();
            let files = match self.inner.lock().list() {
                Ok(f) => f,
                Err(msg) => return result,
            };
            for file in files {
                result.push(Arc::new(
                    FAT32File {
                        inner: Mutex::new(file)
                    }
                ));
            }
            return result;
        }
}