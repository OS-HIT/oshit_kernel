//! Driver module for oshit-kernel

pub mod sdcard;
mod virt;
use core::any::Any;

pub use sdcard::SDCard0WithLock;

use lazy_static::*;
use alloc::sync::Arc;

/// Selecting block device depending on conditional compiling
#[cfg(feature = "board_qemu")]
type BlockDeviceImpl = virt::VirtIOBlock;

/// Selecting block device depending on conditional compiling
#[cfg(feature = "board_k210")]
type BlockDeviceImpl = sdcard::SDCard0WithLock;

lazy_static! {
        /// This is where the rootfs at.
        pub static ref BLOCK_DEVICE: Arc<dyn BlockDevice> = Arc::new(BlockDeviceImpl::new());
}

/// A trait representing any block devices. If a struct implemented this trait, it can be mounted.
pub trait BlockDevice : Send + Sync + Any {

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

        /// Get block count from a block device.
        /// # Description
        /// Get block count from a block device
        /// # Examples
        /// ```
        /// let blk_cnt = BLOCK_DEVICE.block_cnt();
        /// ```
        /// # Returns
        /// The block count of the block device
        fn block_cnt(&self) -> u64;
}
