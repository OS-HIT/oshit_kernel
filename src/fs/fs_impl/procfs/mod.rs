use alloc::{string::ToString, sync::Arc, vec::Vec};

use crate::{fs::{File, FileStatus, Path, parse_path}, process::current_process};

use super::VirtualFileSystem;
use crate::process::ErrNo;

use lazy_static::*;

pub struct ProcSelfExe {}

impl Drop for ProcSelfExe {
    fn drop(&mut self) {
    }
}

impl File for ProcSelfExe {
    fn seek(&self, offset: isize, op: crate::fs::SeekOp) -> Result<(), ErrNo> {
        Err(ErrNo::IllegalSeek)
    }

    fn get_cursor(&self) -> Result<usize, ErrNo> {
        Err(ErrNo::IllegalSeek)
    }

    fn read(&self, buffer: &mut [u8]) -> Result<usize, ErrNo> {
		let mut name: Vec<u8> = current_process().unwrap().immu_infos.exec_path.as_bytes().to_vec();
		name.push(0);
		let min_len = core::cmp::min(buffer.len(), name.len());
		buffer[..min_len].copy_from_slice(&name[..min_len]);
		Ok(min_len)
    }

    fn write(&self, buffer: &[u8]) -> Result<usize, ErrNo> {
        Err(ErrNo::PermissionDenied)
    }

    fn read_user_buffer(&self, mut buffer: crate::memory::UserBuffer) -> Result<usize, ErrNo> {
		let mut name: Vec<u8> = current_process().unwrap().immu_infos.exec_path.as_bytes().to_vec();
		name.push(0);
		let min_len = core::cmp::min(buffer.len(), name.len());
		for i in 0..min_len {
			buffer[i] = name[i];
		}
		Ok(min_len)
    }

    fn write_user_buffer(&self, buffer: crate::memory::UserBuffer) -> Result<usize, ErrNo> {
        Err(ErrNo::PermissionDenied)
    }

    fn to_common_file<'a>(self: alloc::sync::Arc<Self>) -> Option<alloc::sync::Arc<dyn super::CommonFile + 'a>> where Self: 'a {
        None
    }

    fn to_dir_file<'a>(self: alloc::sync::Arc<Self>) -> Option<alloc::sync::Arc<dyn super::DirFile + 'a>> where Self: 'a {
        None
    }

    fn to_device_file<'a>(self: alloc::sync::Arc<Self>) -> Option<alloc::sync::Arc<dyn super::DeviceFile + 'a>> where Self: 'a {
        None
    }

    fn poll(&self) -> crate::fs::FileStatus {
        FileStatus {
            readable: 	true,
            writeable: 	false,
            size: 		(current_process().unwrap().immu_infos.exec_path.as_bytes().len() + 1) as u64,
            name: 		"exe".to_string(),
            ftype: 		crate::fs::FileType::Link,
            inode: 		0,
            dev_no: 	0,
            mode: 		0,
            block_sz: 	512,
            blocks: 	1,
            uid: 		0,
            gid: 		0,
            atime_sec:  0,
            atime_nsec:	0,
            mtime_sec:	0,
            mtime_nsec:	0,
            ctime_sec:	0,
            ctime_nsec:	0,
        }
    }

    fn rename(&self, new_name: &str) -> Result<(), ErrNo> {
		Err(ErrNo::ReadonlyFileSystem)
    }

    fn get_vfs(&self) -> Result<alloc::sync::Arc<dyn super::VirtualFileSystem>, ErrNo> {
        Ok(PROC_FS.clone())
    }

    fn get_path(&self) -> crate::fs::Path {
        parse_path("/self/exe").unwrap()
    }
}

pub struct ProcFS {}

lazy_static! {
	pub static ref PROC_FS: Arc<ProcFS> = Arc::new(ProcFS{});
}

impl VirtualFileSystem for ProcFS {
    fn sync(&self, wait: bool) {
		
    }

    fn get_status(&self) -> super::FSStatus {
        todo!()
    }

    fn open(&self, abs_path: crate::fs::Path, mode: super::OpenMode) -> Result<alloc::sync::Arc<dyn File>, ErrNo> {
        if abs_path.to_string() == "/self/exe" {
			return Ok(Arc::new(ProcSelfExe{}));
		}
		Err(ErrNo::NoSuchFileOrDirectory)
    }

    fn mkdir(&self, abs_path: crate::fs::Path) -> Result<alloc::sync::Arc<dyn File>, ErrNo> {
		Err(ErrNo::ReadonlyFileSystem)
    }

    fn mkfile(&self, abs_path: crate::fs::Path) -> Result<alloc::sync::Arc<dyn File>, ErrNo> {
		Err(ErrNo::ReadonlyFileSystem)
    }

    fn remove(&self, abs_path: crate::fs::Path) -> Result<(), ErrNo> {
		Err(ErrNo::ReadonlyFileSystem)
    }

    fn link(&self, to_link: alloc::sync::Arc<dyn File>, dest: crate::fs::Path) -> Result<(), ErrNo> {
		Err(ErrNo::ReadonlyFileSystem)
    }

    fn sym_link(&self, abs_src: crate::fs::Path, rel_dst: crate::fs::Path) -> Result<(), ErrNo> {
		Err(ErrNo::ReadonlyFileSystem)
    }

    fn rename(&self, to_rename: alloc::sync::Arc<dyn File>, new_name: alloc::string::String) -> Result<(), ErrNo> {
		Err(ErrNo::PermissionDenied)
    }
}