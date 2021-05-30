mod blkcache;

use alloc::sync::Arc;
use spin::Mutex;
use blkcache::BlockCache;
use alloc::collections::VecDeque;
use lazy_static::*;

use crate::fs::BLOCK_DEVICE;

pub const BLOCK_SZ: usize = 512;

const BLOCK_CACHE_SIZE: usize = 16;

pub struct BlockCacheManager {
    queue: VecDeque<(usize, Arc<Mutex<BlockCache>>)>,
}

impl BlockCacheManager {
        pub fn new() -> Self {
                Self { queue: VecDeque::new() }
        }

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
                        self.queue.push_back((block_id, Arc::clone(&block_cache)));
                        block_cache
                }
        }

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

        pub fn flush_all(&self) {
                for cache in self.queue.iter() {
                        cache.1.lock().sync();
                }
        }

}

lazy_static! {
        pub static ref BLOCK_CACHE_MANAGER: Mutex<BlockCacheManager> = Mutex::new(
                BlockCacheManager::new()
        );
}

pub fn get_block_cache(
        block_id: usize,
) -> Arc<Mutex<BlockCache>> {
        let mut locked = BLOCK_CACHE_MANAGER.lock();
        // debug!("get_block_cache enter {:0x}", BlockCacheManager::get_block_cache as usize);
        locked.get_block_cache(block_id)
}

pub fn clear_block_cache (block_id: usize) {
        BLOCK_CACHE_MANAGER.lock().clear_block_cache(block_id);
}

pub fn flush(cache: Arc<Mutex<BlockCache>>) {
        cache.lock().sync();
}

pub fn flush_all() {
        BLOCK_CACHE_MANAGER.lock().flush_all();
}