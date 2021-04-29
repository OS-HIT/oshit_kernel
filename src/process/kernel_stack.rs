use crate::memory::{
    VirtAddr,
    KERNEL_MEM_LAYOUT,
    Segment,
    MapType,
    SegmentFlags
};
use crate::config::*;
use super::Pid;

pub fn kernel_stack_pos(pid: usize) -> (usize, usize) {
    let top = TRAMPOLINE - pid * (KERNEL_STACK_SIZE + PAGE_SIZE);
    return (top - KERNEL_STACK_SIZE, top);
}

pub struct KernelStack {
    pid: usize,
}

impl KernelStack {
    pub fn new(pid: &Pid) -> Self {
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_pos(pid.0);
        KERNEL_MEM_LAYOUT
            .lock()
            .add_segment(
                Segment::new(
                    VirtAddr::from(kernel_stack_bottom), 
                    VirtAddr::from(kernel_stack_top),
                    MapType::Framed,
                    SegmentFlags::R | SegmentFlags::X
                )
            );
        return KernelStack {
            pid: pid.0
        };
    }

    pub fn save_to_top<T>(&self, value: T) -> *mut T where T: Sized {
        let top = kernel_stack_pos(self.pid).1;
        let obj_ptr = (top - core::mem::size_of::<T>()) as *mut T;
        unsafe {*obj_ptr = value;}
        return obj_ptr;
    }
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        KERNEL_MEM_LAYOUT.lock().drop_segment(kernel_stack_pos(self.pid).0.into());
    }
}
