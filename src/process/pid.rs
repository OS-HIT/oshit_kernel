use alloc::vec::Vec;
use lazy_static::*;
use spin::Mutex;

pub struct Pid(pub usize);

impl Drop for Pid {
    fn drop(&mut self) {
        PID_ALLOCATOR.lock().free(self.0);
    }
}

struct PidAllocator {
    nxt_free: usize,
    recycled: Vec<usize>,
}

impl PidAllocator {
    pub fn new() -> Self {
        PidAllocator {
            nxt_free: 0,
            recycled: Vec::new(),
        }
    }

    pub fn alloc(&mut self) -> Pid {
        if let Some(res) = self.recycled.pop() {
            return Pid(res);
        } else {
            self.nxt_free += 1;
            return Pid(self.nxt_free - 1);
        }
    }

    pub fn free(&mut self, pid: usize) {
        assert!(pid < self.nxt_free, "This pid is free.");
        assert!(!self.recycled.iter().any(|&i| i==pid), "This pid is free.");
        self.recycled.push(pid);
    }
}

lazy_static! {
    static ref PID_ALLOCATOR: Mutex<PidAllocator> = Mutex::new(PidAllocator::new());
}

pub fn alloc_pid() -> Pid {
    return PID_ALLOCATOR.lock().alloc();
}