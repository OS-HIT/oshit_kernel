use super::super::{File, OpenMode, Path};
use alloc::{string::String, sync::Arc, vec::Vec};
use crate::process::ErrNo;

pub trait CommonFile : File {
    // fn follow_syn_link(&self) -> Arc<dyn File>;
}
pub trait DirFile : CommonFile {
    /// open files under dir
    fn open(&self, path: Path, mode: OpenMode) -> Result<Arc<dyn File>, ErrNo>;

    /// mkdir. remember to sanitize name.
    fn mkdir(&self, name: Path) -> Result<Arc<dyn File>, ErrNo>;

    /// make file. remember to sanitize name.
    fn mkfile(&self, name: Path) -> Result<Arc<dyn File>, ErrNo>;

    /// delete
    fn remove(&self, path: Path) -> Result<(), ErrNo>;

    /// list
    fn list(&self) -> Vec<Arc<dyn File>>;
}