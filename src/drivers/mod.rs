pub mod sdcard;
use core::any::Any;

pub use sdcard::SDCard0WithLock;

pub trait BlockDevice : Send + Sync + Any {
        fn read_block(&self, block_id: usize, buf: &mut [u8]);
        fn write_block(&self, block_id: usize, buf: &[u8]);
        fn clear_block(&self, block_id: usize);
        fn block_cnt(&self) -> u64;
}
