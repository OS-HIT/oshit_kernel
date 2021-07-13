use crate::fs::VirtualFileSystem;
use alloc::sync::Arc;
use lazy_static::*;

lazy_static! {
	pub static ref DEV_FS: Arc<DevFS> = Arc::new(DevFS::new());
}
struct DevFS {

}

impl DevFS {
	pub fn new() -> Self {
		todo!()
	}
}

impl VirtualFileSystem for DevFS {
    fn sync(&self, wait: bool) {
        todo!()
    }

    fn get_status(&self) -> crate::fs::FSStatus {
        todo!()
    }

    fn open(&self, abs_path: alloc::string::String, mode: crate::fs::OpenMode) -> Result<alloc::sync::Arc<dyn crate::fs::File>, &'static str> {
        todo!()
    }

    fn mkdir(&self, abs_path: alloc::string::String) -> Result<alloc::sync::Arc<dyn crate::fs::File>, &'static str> {
        todo!()
    }

    fn mkfile(&self, abs_path: alloc::string::String) -> Result<alloc::sync::Arc<dyn crate::fs::File>, &'static str> {
        todo!()
    }

    fn remove(&self, abs_path: alloc::string::String) -> Result<(), &'static str> {
        todo!()
    }

    fn link(&self, to_link: alloc::sync::Arc<dyn crate::fs::File>, dest: alloc::string::String) -> Result<(), &'static str> {
        todo!()
    }

    fn sym_link(&self, abs_src: alloc::string::String, rel_dst: alloc::string::String) -> Result<(), &'static str> {
        todo!()
    }

    fn rename(&self, to_rename: alloc::sync::Arc<dyn crate::fs::File>, new_name: alloc::string::String) -> Result<(), &'static str> {
        todo!()
    }
}