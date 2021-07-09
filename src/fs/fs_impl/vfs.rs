use super::super::File;
use alloc::sync::Arc;
use bitflags::*;
use alloc::string::String;


bitflags! {
    /// fs flags
    pub struct FSFlags: u64 {
        /// todo
        const PLACE_HOLDER = 1 << 0;
    }
}

/// file system status
pub struct FSStatus {
    pub name: &'static str,
    pub flags: FSFlags,
    // TODO: mounted dev etc
}


bitflags! {
    /// fs flags
    pub struct OpenMode: u64 {
        /// todo.
        const READ = 1 << 0;
        const WRITE = 1 << 1;
        const CREATE = 1 << 2;
    }
}

pub trait VirtualFileSystem {
    // ==================== fs level ops ====================

    /// force write back all dirty
    fn sync(&self, wait: bool);

    /// get status
    fn get_status(&self) -> FSStatus;

    // ==================== file level ops ====================
    /// create inode (read from disc etc), used for open files.  
    /// we first create it's inode, then opens it.
    /// todo: maybe a specific Path struct?
    fn open(&self, abs_path: String, mode: OpenMode) -> Result<Arc<dyn File>, &'static str>;

    fn mkdir(&self, abs_path: String) -> Result<Arc<dyn File>, &'static str>;

    fn mkfile(&self, abs_path: String) -> Result<Arc<dyn File>, &'static str>;

    fn remove(&self, abs_path: String) -> Result<(), &'static str>;
    
    fn link(&self, to_link: Arc<dyn File>, dest: String) -> Result<(), &'static str>;

    fn sym_link(&self, abs_src: String, rel_dst: String) -> Result<(), &'static str>;

    fn rename(&self, to_rename: Arc<dyn File>, new_name: String) -> Result<(), &'static str>;
}