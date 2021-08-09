use crate::fs::{CommonFile, DirFile, FSFlags, FSStatus, File, VirtualFileSystem, file::FileStatus, SDA_WRAPPER};
use crate::fs::Path;
use super::{CharDeviceFile, DeviceFile, TTY0};
use alloc::{string::{String, ToString}, sync::Arc, vec::Vec};
use lazy_static::*;

lazy_static! {
	pub static ref DEV_FS: Arc<DevFS> = Arc::new(DevFS::new());
    pub static ref DEV_FS_BLOCK_FOLDER: Arc<DevFSBLockFolder> = Arc::new(DevFSBLockFolder::new());
}
pub struct DevFS {

}

impl DevFS {
	pub fn new() -> Self {
        Self{}
        // Do nothing?
	}
}

pub struct DevFSBLockFolder {
    // Add list of block devices in the future
}

impl DevFSBLockFolder {
    pub fn new() -> Self {
        Self{}
    }
}

impl Drop for DevFSBLockFolder {
    fn drop (&mut self) {
        // do nothing
    }
}

impl File for DevFSBLockFolder {
    fn seek(&self, offset: isize, op: crate::fs::SeekOp) -> Result<(), &'static str> {
        Err("Cannot seek dir file")
    }

    fn get_cursor(&self) -> Result<usize, &'static str> {
        Err("No cursor in dir file")
    }

    fn read(&self, buffer: &mut [u8]) -> Result<usize, &'static str> {
        Err("Cannot read dir file")
    }

    fn write(&self, buffer: &[u8]) -> Result<usize, &'static str> {
        Err("Cannot write dir file")
    }

    fn read_user_buffer(&self, buffer: crate::memory::UserBuffer) -> Result<usize, &'static str> {
        Err("Cannot read dir file")
    }

    fn write_user_buffer(&self, buffer: crate::memory::UserBuffer) -> Result<usize, &'static str> {
        Err("Cannot write dir file")
    }

    fn to_common_file<'a>(self: Arc<Self>) -> Option<Arc<dyn CommonFile + 'a>> where Self: 'a {
        None
    }

    fn to_dir_file<'a>(self: Arc<Self>) -> Option<Arc<dyn DirFile + 'a>> where Self: 'a {
        Some(self)
    }

    fn to_device_file<'a>(self: Arc<Self>) -> Option<Arc<dyn DeviceFile + 'a>> where Self: 'a {
        None
    }

    fn poll(&self) -> crate::fs::file::FileStatus {
        FileStatus {
            readable: false,
            writeable: false,
            size: 0,
            name: "block".to_string(),
            ftype: crate::fs::file::FileType::Directory,
            inode: 0,
            dev_no: 0,
            mode: 0,
            block_sz: 0,
            blocks: 0,
            uid: 0,
            gid: 0,
            atime_sec:  0,
            atime_nsec: 0,
            mtime_sec:  0,
            mtime_nsec: 0,
            ctime_sec:  0,
            ctime_nsec: 0,
        }
    }

    fn rename(&self, new_name: &str) -> Result<(), &'static str> {
        Err("cannot rename in devfs")
    }

    fn get_vfs(&self) -> Result<Arc<(dyn VirtualFileSystem + 'static)>, &'static str> {
        Ok(DEV_FS.clone())
    }

    fn get_path(&self) -> Path {
        let path = vec![String::from("block")];
        return Path {path, must_dir: false, is_abs: true};
    }
}

impl CommonFile for DevFSBLockFolder {

}

impl DirFile for DevFSBLockFolder {
    fn open(&self, path: Path, mode: crate::fs::OpenMode) -> Result<Arc<(dyn File + 'static)>, &'static str> {
        if path.path.len() != 1 {
            return Err("invalid path");
        } 
        if path.path[0] == String::from("sda") {
            return Ok(SDA_WRAPPER.clone())
        } else {
            return Err("File not found")
        }
    }

    fn mkdir(&self, name: Path) -> Result<Arc<dyn File>, &'static str> {
        Err("Cannot mkdir in devfs")
    }

    fn mkfile(&self, name: Path) -> Result<Arc<dyn File>, &'static str> {
        Err("Cannot mkfile in devfs")
    }

    fn remove(&self, path: Path) -> Result<(), &'static str> {
        Err("Cannot remove in devfs")
    }

    fn list(&self) -> alloc::vec::Vec<Arc<dyn File>> {
        let mut list: Vec<Arc<dyn File>> = Vec::new();
        list.push(TTY0.clone());
        list
    }
}

impl VirtualFileSystem for DevFS {
    fn sync(&self, wait: bool) {
        TTY0.flush();
    }

    fn get_status(&self) -> crate::fs::FSStatus {
        FSStatus {
            name: "devfs",
            flags: FSFlags::PLACE_HOLDER,
        }
    }

    fn open(&self, abs_path: Path, mode: crate::fs::OpenMode) -> Result<alloc::sync::Arc<dyn crate::fs::File>, &'static str> {
        // hard coded
        match abs_path.path.len() {
            0 => return Err("Empty path"),
            1 => {
                if abs_path.path[0] == "tty0" {
                    return Ok(TTY0.clone());
                } else if abs_path.path[0] == "block" {
                    return Ok(DEV_FS_BLOCK_FOLDER.clone());
                }
            },
            2 => {
                if abs_path.path[0] == "block" && abs_path.path[1] == "sda" {
                    return Ok(SDA_WRAPPER.clone());
                }
            }
            _ => {},
        }
        Err("File not found.")
    }

    fn mkdir(&self, abs_path: Path) -> Result<alloc::sync::Arc<dyn crate::fs::File>, &'static str> {
        Err("Cannot mkdir in devfs")
    }

    fn mkfile(&self, abs_path: Path) -> Result<alloc::sync::Arc<dyn crate::fs::File>, &'static str> {
        Err("Cannot mkfile in devfs")
    }

    fn remove(&self, abs_path: Path) -> Result<(), &'static str> {
        Err("Cannot remove in devfs")
    }

    fn link(&self, to_link: alloc::sync::Arc<dyn crate::fs::File>, dest: Path) -> Result<(), &'static str> {
        Err("Cannot link in devfs")
    }

    fn sym_link(&self, abs_src: Path, rel_dst: Path) -> Result<(), &'static str> {
        Err("Cannot unlink in devfs")
    }

    fn rename(&self, to_rename: alloc::sync::Arc<dyn crate::fs::File>, new_name: String) -> Result<(), &'static str> {
        Err("Cannot rename in devfs")
    }
}