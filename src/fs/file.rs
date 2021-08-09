use alloc::{string::String, sync::Arc};

use crate::memory::UserBuffer;

use super::{CommonFile, DeviceFile, DirFile, VirtualFileSystem};
use super::Path;
use bitflags::*;

/// seek types, def similar to linux man
pub enum SeekOp {
    SET,
    CUR,
    END,
}

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub enum FileType {
    Unknown     = 0o000000,
    FIFO        = 0o010000,
    CharDev     = 0o020000,
    Directory   = 0o040000,
    BlockDev    = 0o060000,
    Regular     = 0o100000,
    Link        = 0o120000,
    Sock        = 0o140000,
}

/// File status, indicating readability/writeability/create time/etc
#[derive(Clone, Debug)]
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
    fn seek(&self, offset: isize, op: SeekOp) -> Result<(), &'static str>;

    /// get cursor
    fn get_cursor(&self) -> Result<usize, &'static str>;

    /// read to buffers
    /// return length read on success
    fn read(&self, buffer: &mut [u8]) -> Result<usize, &'static str>;

    /// write from buffers
    /// return length written on success
    fn write(&self, buffer: &[u8]) -> Result<usize, &'static str>;

    /// read to buffers
    /// return length read on success
    fn read_user_buffer(&self, buffer: UserBuffer) -> Result<usize, &'static str>;

    /// write from buffers
    /// return length written on success
    fn write_user_buffer(&self, buffer: UserBuffer) -> Result<usize, &'static str>;

    /// cast down to common file
    /// HACK: It is unclear how this will coop with Arc<File>, recommand no holding this but Arc<File>.
    /// return casted on success
    fn to_common_file<'a>(self: Arc<Self>) -> Option<Arc<dyn CommonFile + 'a>> where Self: 'a;

    /// cast down to common file
    /// HACK: It is unclear how this will coop with Arc<File>, recommand no holding this but Arc<File>.
    /// return casted on success
    fn to_dir_file<'a>(self: Arc<Self>) -> Option<Arc<dyn DirFile + 'a>> where Self: 'a;

    /// cast down to device file
    /// HACK: It is unclear how this will coop with Arc<File>, recommand no holding this but Arc<File>.
    /// return casted on success
    fn to_device_file<'a>(self: Arc<Self>) -> Option<Arc<dyn DeviceFile + 'a>> where Self: 'a;

    /// Get file status
    fn poll(&self) -> FileStatus;

    /// rename
    fn rename(&self, new_name: &str) -> Result<(), &'static str>;

    fn get_vfs(&self) -> Result<Arc<dyn VirtualFileSystem>, &'static str>;

    fn get_path(&self) -> Path;
}