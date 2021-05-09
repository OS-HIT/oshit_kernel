use crate::drivers::BlockDevice;
use crate::drivers::sdcard::SDCard0WithLock;
use crate::drivers::BLOCK_DEVICE;
use alloc::sync::Arc;

use super::BLOCK_SZ;

pub struct BlockCache {
        cache: [u8; BLOCK_SZ],
        block_id: usize,
        modified: bool,
}

impl BlockCache {
        // const block_device: Arc<SDCard0WithLock> = BLOCK_DEVICE.clone();
        fn device() -> Arc<SDCard0WithLock> {
                return BLOCK_DEVICE.clone();
        }
        /// Load a new BlockCache from disk.
        pub fn new(
                block_id: usize,
        ) -> Self {
                let mut cache = [0u8; BLOCK_SZ];
                BlockCache::device().read_block(block_id, &mut cache);
                Self {
                        cache,
                        block_id,
                        modified: false,
                }
        }

        fn addr_of_offset(&self, offset: usize) -> usize {
                &self.cache[offset] as *const _ as usize
        }
        
        pub fn get_ref<T>(&self, offset: usize) -> &T where T: Sized {
                let type_size = core::mem::size_of::<T>();
                assert!(offset + type_size <= BLOCK_SZ);
                let addr = self.addr_of_offset(offset);
                unsafe { &*(addr as *const T) }
        }
        
        pub fn get_mut<T>(&mut self, offset: usize) -> &mut T where T: Sized {
                let type_size = core::mem::size_of::<T>();
                assert!(offset + type_size <= BLOCK_SZ);
                self.modified = true;
                let addr = self.addr_of_offset(offset);
                unsafe { &mut *(addr as *mut T) }
        }

        pub fn clear(&mut self) {
                self.modified = false;
                for i in 0..BLOCK_SZ {
                        self.cache[i] = 0;
                }
        }

        pub fn sync(&mut self) {
                if self.modified {
                        self.modified = false;
                        BlockCache::device().write_block(self.block_id, &self.cache);
                }
        }

        pub fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V {
                f(self.get_ref(offset))
        }
        
        pub fn modify<T, V>(&mut self, offset:usize, f: impl FnOnce(&mut T) -> V) -> V {
                f(self.get_mut(offset))
        }
}

impl Drop for BlockCache {
        fn drop(&mut self) {
            self.sync()
        }
}