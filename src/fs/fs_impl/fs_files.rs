use super::super::File;
use alloc::{string::String, sync::Arc};

pub trait CommonFile : File {
    fn follow_syn_link(&self) -> Arc<dyn File>;
}
pub trait DirFile : File {
    /// open files under dir
    fn open(&self, path: String) -> Arc<dyn File>;

    /// mkdir. remember to sanitize name.
    fn mkdir(&self, name: String) -> Result<Arc<dyn File>, &'static str>;

    /// make file. remember to sanitize name.
    fn mkfile(&self, name: String) -> Result<Arc<dyn File>, &'static str>;

    /// delete
    fn remove(&self, path: String) -> Result<(), &'static str>;
}