// saved on top of the kernel stack of corresponding process.
#[repr(C)]
pub struct ProcessContext {
    ra  : usize,
    s   : [usize; 12],
}

impl ProcessContext {
    // constructor (?)
    // load in __restore as ra
    pub fn init() -> Self {
        extern "C" { fn __restore(); }
        return Self {
            ra  : __restore as usize,
            s   : [0; 12],
        };
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum ProcessStatus {
    New,
    Ready,
    Running,
    Dead
}

pub struct ProcessControlBlock {
    pub context_ptr: usize,
    pub status: ProcessStatus,
    // TODO: others add in future
}