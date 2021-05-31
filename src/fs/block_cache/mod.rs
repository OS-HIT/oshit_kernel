//! Manager of block caches
mod blkcache;

use alloc::sync::Arc;
use spin::Mutex;
use blkcache::BlockCache;
use alloc::collections::VecDeque;
use lazy_static::*;

use crate::fs::BLOCK_DEVICE;

pub const BLOCK_SZ: usize = 512;

const BLOCK_CACHE_SIZE: usize = 1;

/// Manager of block caches
pub struct BlockCacheManager {
        /// vector queue of block cache  
        queue: VecDeque<(usize, Arc<Mutex<BlockCache>>)>,
}

impl BlockCacheManager {
        /// Create new block cache
        pub fn new() -> Self {
                Self { queue: VecDeque::new() }
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
                                BlockCache::new(block_id)
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
                if block_id < 100 {
                        error!("clear_block_cache called on {}", block_id);
                }
                if let Some(pair) = self.queue.iter().find(|pair| pair.0 == block_id) {
                        pair.1.lock().clear();
                }
                BLOCK_DEVICE.clear_block(block_id);
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

lazy_static! {
        /// Initilize a block cache manager
        pub static ref BLOCK_CACHE_MANAGER: Mutex<BlockCacheManager> = Mutex::new(
                BlockCacheManager::new()
        );
}

/// Wrapper function of get_block_cache of singleton block cache manager
pub fn get_block_cache(
        block_id: usize,
) -> Arc<Mutex<BlockCache>> {
        let mut locked = BLOCK_CACHE_MANAGER.lock();
        // debug!("get_block_cache enter {:0x}", BlockCacheManager::get_block_cache as usize);
        locked.get_block_cache(block_id)
}

/// Wrapper function of clear_block_cache of singleton block cache manager
pub fn clear_block_cache (block_id: usize) {
        BLOCK_CACHE_MANAGER.lock().clear_block_cache(block_id);
}

/// Write specified cache back to block device without freeing cache
pub fn flush(cache: Arc<Mutex<BlockCache>>) {
        cache.lock().sync();
}

/// Wrapper function of flush_all of singleton block cache manager
pub fn flush_all() {
        BLOCK_CACHE_MANAGER.lock().flush_all();
}