use alloc::{string::String, sync::Arc};

use super::{CommonFile, DeviceFile, DirFile, VirtualFileSystem};


/// seek types, def similar to linux man
pub enum SeekOp {
    SET,
    CUR,
    END,
}

/// File status, indicating readability/writeability/create time/etc
#[derive(Clone, Copy)]
pub struct FileStatus {
    pub readable: bool,
    pub writeable: bool,
    // todo: finish this
}

/// File traits. Mostly inspired by linux file_operations struct. Implements Drop Trait.
pub trait File: Drop + Send + Sync {
    /// seek cursor. Some type of file not support this (like char device)
    fn seek(&self, offset: u64, op: SeekOp) -> Result<(), &'static str>;

    /// read to buffers
    /// return length read on success
    fn read(&self, buffer: &[u8], length: u64) -> Result<u64, &'static str>;

    /// write from buffers
    /// return length written on success
    fn write(&self, buffer: &[u8], length: u64) -> Result<u64, &'static str>;

    /// cast down to common file
    /// HACK: It is unclear how this will coop with Arc<File>, recommand no holding this but Arc<File>.
    /// return casted on success
    fn to_common_file(&self) -> Option<Arc<dyn CommonFile>>;

    /// cast down to common file
    /// HACK: It is unclear how this will coop with Arc<File>, recommand no holding this but Arc<File>.
    /// return casted on success
    fn to_dir_file(&self) -> Option<Arc<dyn DirFile>>;

    /// cast down to device file
    /// HACK: It is unclear how this will coop with Arc<File>, recommand no holding this but Arc<File>.
    /// return casted on success
    fn to_device_file(&self) -> Option<Arc<dyn DeviceFile>>;

    /// Get file status
    fn poll(&self) -> FileStatus;

    /// rename
    fn rename(&self, new_name: String) -> Result<(), &'static str>;

    fn get_vfs(&self) -> Arc<dyn VirtualFileSystem>;

    fn get_path(&self) -> String;
}