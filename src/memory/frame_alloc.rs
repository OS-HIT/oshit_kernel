//! Physical frame allocator for oshit kernel memory management module.

use crate::config::MEM_END;
use super::{
    PhysPageNum,
    PhysAddr
};
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::*;

/// The frame allocator trait. Anything implemented this trait can be our frame allocator
trait FrameAllocator {

    /// Construct a new frame allocator
    /// # Description
    /// Construct a new frame allocator, managing physical memory in [start, stop)
    /// # Return
    /// A frame allocator
    fn new(start: PhysPageNum, stop: PhysPageNum) -> Self;

    /// Alloc a new physical frame in the managed area
    /// # Description
    /// Alloc a new physical frame in the managed area. If failed, return None.  
    /// # Return
    /// Return the physical page number on success, or None if OOM.
    fn alloc(&mut self) -> Option<PhysPageNum>;

    /// Free the physical frame
    /// # Description
    /// Free the physical frame in the managed area, allow it to be alloced again in the future.
    fn free(&mut self, to_free: PhysPageNum);

    fn alloc_continuous(&mut self, size_in_pages: usize) -> Option<PhysPageNum>;
}

lazy_static! {
    /// Lazy initialized instance of the frame allocator implementation. Currently using StackFrameAllocator.
    pub static ref FRAME_ALLOCATOR: Mutex<StackFrameAllocator> = {
        debug!("Initializing page frame allocator...");
        extern "C" {
            fn ekernel();
        }
        let start = PhysAddr::from(ekernel as usize).to_ppn_ceil();
        let stop = PhysAddr::from(MEM_END).to_ppn();
        Mutex::new(StackFrameAllocator::new(start, stop))
    };
}

/// Alloc a frame.
/// # Description
/// Alloc a physical frame, return Some(FrameTracker) on success, and None on OOM.  
/// Note that we don't need to free explicitly, the page will be automatically freed when the FrameTracker is dropped.  
/// If you want to free a frame, just drop frame tracker
/// # Example
/// ```
/// if let Some(ft) = alloc_frame() {
///     // Do something with the FrameTracker
///     do_something(ft);
///     // The frame tracker is dropped, so the page is freed
/// } else {
///     error!("OOM!");
/// }
/// ```
/// # Return
/// Some(FrameTracker) on success, None on OOM
pub fn alloc_frame() -> Option<FrameTracker> {
    FRAME_ALLOCATOR.lock().alloc().map(|ppn| FrameTracker::new(ppn))
}

pub fn alloc_continuous(size_in_pages: usize) -> Vec<FrameTracker> {
    let mut res = Vec::new();
    let start = FRAME_ALLOCATOR.lock().alloc_continuous(size_in_pages).unwrap();
    for i in 0..size_in_pages {
        res.push(FrameTracker::new(start + i));
    }
    res
}

pub fn free_frame(ppn: PhysPageNum) {
    FRAME_ALLOCATOR.lock().free(ppn);
}

/// The frame tracker, representing a physical frame.  
/// It's created alone the alloc process, and when it's dropped it automatically free the coresponding page.
pub struct FrameTracker {
    pub ppn: PhysPageNum
}

impl FrameTracker {
    /// Constructor
    pub fn new(ppn: PhysPageNum) -> Self {
        for i in ppn.page_ptr() {
            *i = 0;
        }
        return Self {ppn};
    }
}

/// Implement drop, so that we can automatically collect used pages.
impl Drop for FrameTracker {
    fn drop(&mut self) {
        verbose!("{:?} has been dropped.", self.ppn);
        FRAME_ALLOCATOR.lock().free(self.ppn)
    }
}

/// The Frame-Allocator-of-choice.
/// A stack frame allocator, keeps records of current freed pages and unallocated pages.
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
            fatal!("Out Of Memory! Cannot alloc any more physical frame.");
            // TODO: support swap out when OOM.
            return None;
        }
    }
    
    fn alloc_continuous(&mut self, size_in_pages: usize) -> Option<PhysPageNum> {
        if self.current + size_in_pages <= self.end {
            self.current += size_in_pages;
            return Some(self.current - size_in_pages);
        } else {
            fatal!("Out Of Memory! Cannot alloc any more physical frame.");
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