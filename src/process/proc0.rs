use super::temp_app_loader::get_app;
use super::ProcessControlBlock;
use super::enqueue;
use lazy_static::*;
use alloc::sync::Arc;

lazy_static! {
    pub static ref PROC0: Arc<ProcessControlBlock> = Arc::new(
        ProcessControlBlock::new(get_app("proc0").unwrap())
    );
}

pub fn init_proc0() {
    enqueue(PROC0.clone());
}