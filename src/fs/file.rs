use alloc::{string::String, sync::Arc};

use crate::memory::UserBuffer;

use super::{CommonFile, DeviceFile, DirFile, VirtualFileSystem};
use bitflags::*;

/// seek types, def similar to linux man
pub enum SeekOp {
    SET,
    CUR,
    END,
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum FileType {
    Unknown=0,
    FIFO=1,
    CharDev=2,
    Directory=4,
    BlockDev=6,
    Regular=8,
    Sock=12,
}

/// File status, indicating readability/writeability/create time/etc
#[derive(Clone)]
pub struct FileStatus {
    pub readable: bool,
    pub writeable: bool,
    pub size: u64,
    pub name: String,
    pub ftype: FileType,
    pub inode: u64,
    pub dev_no: u64,
    pub mode: u32,
    pub block_sz: u32,
    pub blocks: u64,
    pub uid: u32,
    pub gid: u32,
    
    pub atime_sec: u32,
    pub atime_nsec: u32,
    pub mtime_sec: u32,
    pub mtime_nsec: u32,
    pub ctime_sec: u32,
    pub ctime_nsec: u32,
    // todo: finish this
}

/// File traits. Mostly inspired by linux file_operations struct. Implements Drop Trait.
pub trait File: Drop + Send + Sync {
    /// seek cursor. Some type of file not support this (like char device)
    fn seek(&self, offset: u64, op: SeekOp) -> Result<(), &'static str>;

    /// get cursor
    fn get_cursor(&self) -> Result<u64, &'static str>;

    /// read to buffers
    /// return length read on success
    fn read(&self, buffer: &mut [u8]) -> Result<u64, &'static str>;

    /// write from buffers
    /// return length written on success
    fn write(&self, buffer: &[u8]) -> Result<u64, &'static str>;

    /// read to buffers
    /// return length read on success
    fn read_user_buffer(&self, buffer: UserBuffer) -> Result<u64, &'static str>;

    /// write from buffers
    /// return length written on success
    fn write_user_buffer(&self, buffer: UserBuffer) -> Result<u64, &'static str>;

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