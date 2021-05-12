pub mod sdcard;
mod virt;
use core::any::Any;

pub use sdcard::SDCard0WithLock;

use lazy_static::*;
use alloc::sync::Arc;

#[cfg(feature = "board_qemu")]
type BlockDeviceImpl = virt::VirtIOBlock;

#[cfg(feature = "board_k210")]
type BlockDeviceImpl = sdcard::SDCard0WithLock;

lazy_static! {
        pub static ref BLOCK_DEVICE: Arc<dyn BlockDevice> = Arc::new(BlockDeviceImpl::new());
}

pub trait BlockDevice : Send + Sync + Any {
        fn read_block(&self, block_id: usize, buf: &mut [u8]);
        fn write_block(&self, block_id: usize, buf: &[u8]);
        fn clear_block(&self, block_id: usize);
        fn block_cnt(&self) -> u64;
}
