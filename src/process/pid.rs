//! Implementation of process id

use alloc::vec::Vec;
use lazy_static::*;
use spin::Mutex;

/// The PID struct, used for auto recycle. Kinda like the FrameTracker.
pub struct Pid(pub usize);

impl Drop for Pid {
    fn drop(&mut self) {
        PID_ALLOCATOR.lock().free(self.0);
    }
}

/// The PID allocator, a stack allocator.
struct PidAllocator {
    nxt_free: usize,
    recycled: Vec<usize>,
}

impl PidAllocator {
    /// Construct a new pid allocator
    pub fn new() -> Self {
        PidAllocator {
            nxt_free: 0,
            recycled: Vec::new(),
        }
    }

    /// Alloc a new pid.
    pub fn alloc(&mut self) -> Pid {
        if let Some(res) = self.recycled.pop() {
            return Pid(res);
        } else {
            self.nxt_free += 1;
            return Pid(self.nxt_free - 1);
        }
    }

    /// Free a pid, so that it can be used in the future.
    pub fn free(&mut self, pid: usize) {
        assert!(pid < self.nxt_free, "This pid is free.");
        assert!(!self.recycled.iter().any(|&i| i==pid), "This pid is free.");
        self.recycled.push(pid);
    }
}

lazy_static! {
    /// The singleton of the pid allocator
    static ref PID_ALLOCATOR: Mutex<PidAllocator> = Mutex::new(PidAllocator::new());
}

/// Alloc a pid.
/// # Description
/// Alloc a pid. Note that you should hold the Pid object, or the pid will be auto recycled.
pub fn alloc_pid() -> Pid {
    return PID_ALLOCATOR.lock().alloc();
}