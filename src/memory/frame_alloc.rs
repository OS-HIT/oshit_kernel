/// alloc physical frame

use crate::config::MEM_END;
use super::{
    PhysPageNum,
    PhysAddr
};
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::*;

trait FrameAllocator {
    // no worry we got Copy trait for PPN
    fn new(start: PhysPageNum, stop: PhysPageNum) -> Self;
    fn alloc(&mut self) -> Option<PhysPageNum>;
    fn free(&mut self, to_free: PhysPageNum);
}

// TODO: change to CLOCK algorithm
lazy_static! {
    pub static ref FRAME_ALLOCATOR: Mutex<StackFrameAllocator> = {
        verbose!("Initializing page frame allocator...");
        extern "C" {
            fn ekernel();
        }
        let start = PhysAddr::from(ekernel as usize).to_vpn_ceil();
        let stop = PhysAddr::from(MEM_END).to_vpn();
        Mutex::new(StackFrameAllocator::new(start, stop))
    };
}

// RAII, auto collect so no explicit free
// if you want to free a frame, just drop frame tracker
pub fn alloc_frame() -> Option<FrameTracker> {
    FRAME_ALLOCATOR.lock().alloc().map(|ppn| FrameTracker::new(ppn))
}

// Impl drop, to auto gc
// TODO: Lock it maybe? To avoid race
pub struct FrameTracker {
    pub ppn: PhysPageNum
}

impl FrameTracker {
    pub fn new(ppn: PhysPageNum) -> Self {
        for i in ppn.page_ptr() {
            *i = 0;
        }
        return Self {ppn};
    }
}

impl Drop for FrameTracker {
    fn drop(&mut self) {
        FRAME_ALLOCATOR.lock().free(self.ppn)
    }
}

pub struct StackFrameAllocator {
    current : PhysPageNum,
    end     : PhysPageNum,
    freed   : Vec<PhysPageNum>
}

impl FrameAllocator for StackFrameAllocator {
    fn new(start: PhysPageNum, stop: PhysPageNum) -> Self {
        Self {
            current : start,
            end     : stop,
            freed   : Vec::new()
        }
    }

    fn alloc(&mut self) -> Option<PhysPageNum> {
        if let Some(free_frame) = self.freed.pop() {    // try to pop sth out of it
            return Some(free_frame);
        } else if self.current < self.end {
            self.current += 1;
            return Some(self.current - 1);
        } else {
            warning!("Out Of Memory! Cannot alloc any more physical frame.");
            // TODO: support swap out when OOM.
            return None;
        }
    }

    fn free(&mut self, to_free: PhysPageNum) {
        // check if it as been allocated
        if to_free >= self.current || self.freed.iter().any(|&i| i==to_free) {
            error!("Trying to free a PPN that has not been allocated: {:?}", to_free);
        } else {
            self.freed.push(to_free);
        }
    }
}