//! Kernel stack for each process
use crate::memory::{
    VirtAddr,
    KERNEL_MEM_LAYOUT,
    Segment,
    MapType,
    SegmentFlags,
    VMAFlags,
};
use crate::config::*;
use super::Pid;

/// Return the kernel stack position for pid
/// # Description
/// Return the kernel stack position for pid. The positon is tied to the pid, and is located on the top of the kernel memory space.
pub fn kernel_stack_pos(pid: usize) -> (VirtAddr, VirtAddr) {
    let top = TRAMPOLINE - pid * (KERNEL_STACK_SIZE + PAGE_SIZE);
    return ((top - KERNEL_STACK_SIZE).into(), top.into());
}

/// The kernel stack struct, implemented the drop trait to auto free resource
pub struct KernelStack {
    pid: usize,
}

impl KernelStack {
    /// Construct a new kernel stack for `pid`
    /// # Description
    /// Construct a new kernel stack for `pid`, and map it in the kernel memroy layout.
    pub fn new(pid: &Pid) -> Self {
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_pos(pid.0);
        KERNEL_MEM_LAYOUT
            .lock()
            .add_segment(
                Segment::new(
                    kernel_stack_bottom, 
                    kernel_stack_top,
                    MapType::Framed,
                    SegmentFlags::R | SegmentFlags::W,
                    VMAFlags::empty(),
                    None,
                    0
                )
            );
        return KernelStack {
            pid: pid.0
        };
    }

    /// Save something to the top of the kernel stack, usually a ProcessContext.
    pub fn save_to_top<T>(&self, value: T) -> *mut T where T: Sized, {
        let top = self.top();
        let obj_ptr = (top.0 - core::mem::size_of::<T>()) as *mut T;
        unsafe {*obj_ptr = value;}
        return obj_ptr;
    }

    /// get the top of the kernel stack.
    pub fn top(&self) -> VirtAddr {
        return kernel_stack_pos(self.pid).1;
    }
}

/// auto drop the segment when the kernel stack is dropped.
impl Drop for KernelStack {
    fn drop(&mut self) {
        KERNEL_MEM_LAYOUT.lock().drop_segment(kernel_stack_pos(self.pid).0.into());
    }
}
