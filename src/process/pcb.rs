use crate::memory::{
    MemLayout,
    PhysAddr,
    PhysPageNum,
    VirtAddr,
    Segment,
    MapType,
    SegmentFlags,
    KERNEL_MEM_LAYOUT
};
use crate::config::*;
use crate::trap::{
    TrapContext,
    user_trap,
    trap_return
};
use crate::sbi::get_time;

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
            ra  : trap_return as usize,
            s   : [0; 12],
        };
    }
}

#[allow(dead_code)]
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
    pub layout: MemLayout,
    pub trap_context_ppn: PhysPageNum,
    pub size: usize,
    pub up_since: usize,
    pub last_start: usize,
    pub utime: usize,
}

impl ProcessControlBlock {
    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        let (layout, user_stack_top, entry) = MemLayout::new_elf(elf_data);
        let trap_context_ppn = layout.translate(VirtAddr::from(TRAP_CONTEXT).into()).unwrap().ppn();
        let status = ProcessStatus::Ready;
        let kernel_stack_top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
        KERNEL_MEM_LAYOUT.lock().add_segment(
            Segment::new(
                (kernel_stack_top - KERNEL_STACK_SIZE).into(), 
                kernel_stack_top.into(), 
                MapType::Framed,
                SegmentFlags::R | SegmentFlags::W
            )
        );
        let context_ptr = (kernel_stack_top - core::mem::size_of::<ProcessContext>()) as *mut ProcessContext;
        unsafe {*context_ptr = ProcessContext::init();}
        let context_ptr = context_ptr as usize;
        let pcb = Self {
            context_ptr,
            status,
            layout,
            trap_context_ppn,
            size: user_stack_top,
            up_since: get_time(),
            last_start: 0,
            utime: 0,
        };
        let trap_context = pcb.get_trap_context();
        *trap_context = TrapContext::init(
            entry, 
            user_stack_top, 
            KERNEL_MEM_LAYOUT.lock().get_satp(), 
            kernel_stack_top,
            user_trap as usize
        );

        return pcb;
    }

    pub fn get_trap_context(&self) -> &'static mut TrapContext {
        unsafe {
            (PhysAddr::from(self.trap_context_ppn.clone()).0 as *mut TrapContext).as_mut().unwrap()
        }
    }

    pub fn get_satp(&self) -> usize {
        return self.layout.get_satp();
    }
}