pub mod sdcard;
use core::any::Any;

use alloc::sync::Arc;
use lazy_static::*;
use sdcard::SDCard0WithLock;

lazy_static! {
        pub static ref BLOCK_DEVICE: Arc<SDCard0WithLock> = Arc::new(SDCard0WithLock::new());
}

pub trait BlockDevice : Send + Sync + Any {
        fn read_block(&self, block_id: usize, buf: &mut [u8]);
        fn write_block(&self, block_id: usize, buf: &[u8]);
        fn clear_block(&self, block_id: usize);
        fn block_cnt(&self) -> u64;
}

#[allow(unused)]
pub fn sdcard_test() {
        for i in 0..10 as u8 {
                let buf = [i; 512];
                BLOCK_DEVICE.write_block(i as usize, &buf);
        }

        for i in 0..10 as u8 {
                let mut buf = [0u8; 512];
                BLOCK_DEVICE.read_block(i as usize, &mut buf);
                assert_eq!(buf[i as usize], i);
        }

        info!("sdcard test passed");
}
