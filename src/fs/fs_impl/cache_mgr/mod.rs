//! Manager of block caches
pub mod blkcache;

use alloc::sync::Arc;
use alloc::collections::VecDeque;
use spin::Mutex;
use blkcache::BlockCache;

use super::blkdevice::BlockDevice;

pub const BLOCK_SZ: usize = 512;

const BLOCK_CACHE_SIZE: usize = 16;

/// Manager of block caches
pub struct BlockCacheManager {
        /// vector queue of block cache  
        queue: VecDeque<(usize, Arc<Mutex<BlockCache>>)>,
        device: Arc<Mutex<dyn BlockDevice>>,
}

impl BlockCacheManager {
        /// Create new block cache
        pub fn new(device: Arc<Mutex<dyn BlockDevice>>) -> Self {
                Self { 
                        queue: VecDeque::new(),
                        device: device.clone(),
                }
        }

        /// Get a block cache
        /// # Description 
        /// Returns a cache of a block at specified offset of the block device 
        /// Drops earliest allocate cache when necessary
        pub fn get_block_cache(
                &mut self,
                block_id: usize,
        ) -> Arc<Mutex<BlockCache>> {
                // debug!("inner get block cache");
                if let Some(pair) = self.queue
                .iter()
                .find(|pair| pair.0 == block_id) {
                        Arc::clone(&pair.1)
                } else {
                        // substitute
                        if self.queue.len() == BLOCK_CACHE_SIZE {
                                // from front to tail
                                if let Some((idx, _)) = self.queue
                                .iter()
                                .enumerate()
                                .find(|(_, pair)| Arc::strong_count(&pair.1) == 1) {
                                        self.queue.drain(idx..=idx);
                                } else {
                                        panic!("Run out of BlockCache!");
                                }
                        }
                        // load block into mem and push back
                        let block_cache = Arc::new(Mutex::new(
                                BlockCache::new(block_id, self.device.clone())
                        ));
                        // debug!("New Block Cache, addr @ {:x}", (&block_cache.lock().cache[0]) as *const u8 as usize);
                        self.queue.push_back((block_id, Arc::clone(&block_cache)));
                        block_cache
                }
        }

        /// clear block content
        /// # Description 
        /// Reset content of a block at specified offset 
        /// Block cache will be cleared if it is allocated
        pub fn clear_block_cache(&mut self, block_id: usize) {
                if let Some(pair) = self.queue.iter().find(|pair| pair.0 == block_id) {
                        pair.1.lock().clear();
                }
                self.device.lock().clear_block(block_id);
                return;
        }

        /// Flush all caches
        /// # Description  
        /// Write all caches back to Block device without freeing them
        pub fn flush_all(&self) {
                for cache in self.queue.iter() {
                        cache.1.lock().sync();
                }
        }

}

pub type BCMgr = Arc<Mutex<BlockCacheManager>>; 

#[allow(unused)]
/// Wrapper function of get_block_cache of singleton block cache manager
pub fn get_block_cache(
        bcmgr: BCMgr,
        block_id: usize,
) -> Arc<Mutex<BlockCache>> {
        let mut locked = bcmgr.lock();
        // debug!("get_block_cache enter {:0x}", BlockCacheManager::get_block_cache as usize);
        locked.get_block_cache(block_id)
}

#[allow(unused)]
/// Wrapper function of clear_block_cache of singleton block cache manager
pub fn clear_block_cache (bcmgr: BCMgr, block_id: usize) {
        bcmgr.lock().clear_block_cache(block_id);
}

#[allow(unused)]
/// Write specified cache back to block device without freeing cache
pub fn flush(cache: Arc<Mutex<BlockCache>>) {
        cache.lock().sync();
}

#[allow(unused)]
/// Wrapper function of flush_all of singleton block cache manager
pub fn flush_all(bcmgr: BCMgr) {
        bcmgr.lock().flush_all();
}