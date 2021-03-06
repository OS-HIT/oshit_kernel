//! In-Memory Cache for Block Device

use core::fmt::Display;
use core::mem::size_of;

use crate::drivers::BlockDevice;
use crate::drivers::BLOCK_DEVICE;
use alloc::sync::Arc;

use super::BLOCK_SZ;

/// Struct of cache for a block (size: 512B)
pub struct BlockCache {
        /// Block content
        pub cache: [u8; BLOCK_SZ],
        /// Id of a block, whose value equals to block offset in the block device
        block_id: usize,
        /// Indecate whe the block has been modified
        modified: bool,
}

impl BlockCache {
        // const block_device: Arc<SDCard0WithLock> = BLOCK_DEVICE.clone();

        /// Get Block device (SDCard for k210)
        fn device() -> Arc<dyn BlockDevice> {
                return BLOCK_DEVICE.clone();
        }
        /// Load a new BlockCache from disk.
        pub fn new(
                block_id: usize,
        ) -> Self {

                let mut cache = [0u8; BLOCK_SZ];
                BlockCache::device().read_block(block_id, &mut cache);
                if block_id == 32 {
                        debug!("new block 32!!!!!!!!!!!!!!!!!!!!!!!!!!!");
                }
                let ret = Self {
                        cache,
                        block_id,
                        modified: false,
                };
                
                
                // if ret.block_id == 32 {
                //         info!("new called on block 32");
                //         for i in 0..16 {
                //                 for j in 0..8 {
                //                         print!("{:08X} ", ret.get_ref::<u32>((i*8 + j) * 4))
                //                 }
                //                 println!();
                //         }
                // }
                return ret;
        }

        /// Get the memory address that points to the content from cache at the specified offset
        fn addr_of_offset(&self, offset: usize) -> usize {
                &self.cache[offset] as *const _ as usize
        }
        
        /// Get a reference to a object in cache
        /// # Description
        /// Reference returned is read only. Panic when object is out of block baoundary
        pub fn get_ref<T>(&self, offset: usize) -> &T where T: Sized {
                // if self.block_id < 35 && self.block_id > 30 {
                //         // debug!("get_ref called on block 32");
                //         for i in 0..128 {
                //                 unsafe {
                //                         let addr = self.addr_of_offset(i*4);
                //                         let content = *(addr as *const u32);
                //                         if content == 0 {
                //                                 error!("something is wrong with {:#X} at {}", addr, self.block_id);
                //                         }
                //                 }
                //         }
                // }

                let type_size = core::mem::size_of::<T>();
                assert!(offset + type_size <= BLOCK_SZ);
                let addr = self.addr_of_offset(offset);
                unsafe { &*(addr as *const T) }
        }
        
        /// Get a mutable reference to a object in cache
        /// # Description
        /// Panic when object is out of block baoundary
        pub fn get_mut<T>(&mut self, offset: usize) -> &mut T where T: Sized {
                
                // if self.block_id < 35 && self.block_id > 30 {
                //         // debug!("get_ref called on block 32");
                //         for i in 0..128 {
                //                 unsafe {
                //                         let addr = self.addr_of_offset(i*4);
                //                         let content = *(addr as *const u32);
                //                         if content == 0 {
                //                                 error!("something is wrong with {:#X} at {}", addr, self.block_id);
                //                         }
                //                 }
                //         }
                // }

                if self.block_id == 32 {
                        debug!("get_mut called on block 32 {}", size_of::<T>());
                }
                let type_size = core::mem::size_of::<T>();
                assert!(offset + type_size <= BLOCK_SZ);
                self.modified = true;
                let addr = self.addr_of_offset(offset);
                unsafe { &mut *(addr as *mut T) }
        }

        /// Clear cache
        /// # Description 
        /// Set content to zero and reset modified without sync to block device
        pub fn clear(&mut self) {
                if self.block_id == 32 {
                        debug!("clear called on block 32");
                }
                self.modified = false;
                for i in 0..BLOCK_SZ {
                        self.cache[i] = 0;
                }
        }

        /// Write cache content back to block device
        /// # Description
        /// Write only occured when 'modified' flag is set
        /// 'Modified' flag will be reset during this operation 
        pub fn sync(&mut self) {
                // if self.block_id == 32 {
                //         info!("sync called on block 32");
                //         for i in 0..16 {
                //                 for j in 0..8 {
                //                         print!("{:08X} ", self.get_ref::<u32>((i*8 + j) * 4))
                //                 }
                //                 println!();
                //         }
                // }
                if self.modified {
                        self.modified = false;
                        BlockCache::device().write_block(self.block_id, &self.cache);
                }
        }

        /// Not in use
        pub fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V {
                f(self.get_ref(offset))
        }
        
        /// Not in use
        pub fn modify<T, V>(&mut self, offset:usize, f: impl FnOnce(&mut T) -> V) -> V {
                f(self.get_mut(offset))
        }
}

impl Drop for BlockCache {

        /// Drop trait for BlockCache
        /// # Description
        /// Call sync before dropping blockcache
        fn drop(&mut self) {
                if self.block_id == 32 {
                        debug!("dropping block 32");
                }
                self.sync()
        }
}