use alloc::sync::Arc;

use crate::fs::CommonFile;

use super::super::super::File;

pub trait DeviceFile : File {
    /// Good old IOCTL, device spcific commands.
    fn ioctl(&self, op: u64) -> Result<u64, &'static str>;

    fn to_char_dev<'a>(self: Arc<Self>) -> Option<Arc<dyn CharDeviceFile + 'a>> where Self: 'a ;

    fn to_blk_dev<'a>(self: Arc<Self>) -> Option<Arc<dyn BlockDeviceFile + 'a>> where Self: 'a ;
}

pub trait CharDeviceFile : DeviceFile {
    fn flush(&self);
}

pub trait BlockDeviceFile : DeviceFile {

    /// Read a block from the block device.
    /// # Description
    /// Read the block with id=`block_id` from the block device.
    /// # Examples
    /// ```
    /// pub const BLK_SZ = 512;
    /// let mut buf = [0u8; BLK_SZ];
    /// let block_id: isize = 10;
    /// BLOCK_DEVICE.read_block(block_id, &mut buf)
    /// ```
    /// # Returns
    /// No returns
    fn read_block(&self, block_id: usize, buf: &mut [u8]);

    /// Write a block to the block device.
    /// # Description
    /// Write the block with idblock_id to the block device.
    /// # Examples
    /// ```
    /// pub const BLK_SZ = 512;
    /// let buf = [10u8; BLK_SZ];
    /// let block_id: isize = 10;
    /// BLOCK_DEVICE.write_block(block_id, buf)
    /// ```
    /// # Returns
    /// No returns
    fn write_block(&self, block_id: usize, buf: &[u8]);

    /// Clear a spcific block in the block device.
    /// # Description
    /// Clear the block with id=`block_id` on the block device.
    /// # Examples
    /// ```
    /// BLOCK_DEVICE.clear_block(10)
    /// ```
    /// # Returns
    /// No returns
    fn clear_block(&self, block_id: usize);
}

pub trait NetworkDevice : DeviceFile {
    // todo
}