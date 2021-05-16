use crate::fs::{
    FILE,
    FTYPE,
};

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
use super::{
    Pid,
    KernelStack,
    alloc_pid,
    kernel_stack_pos
};
use spin::{
    Mutex,
    MutexGuard
};
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;

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
    Zombie
}

pub struct ProcessControlBlock {
    pub pid:            Pid,
    pub kernel_stack:   KernelStack,
    inner:              Mutex<ProcessControlBlockInner>,
}

pub struct ProcessControlBlockInner {
    pub context_ptr: usize,
    pub status: ProcessStatus,
    pub layout: MemLayout,
    pub trap_context_ppn: PhysPageNum,
    pub size: usize,
    pub up_since: u64,
    pub last_start: u64,
    pub utime: u64,
    pub parent: Option<Weak<ProcessControlBlock>>,
    pub children: Vec<Arc<ProcessControlBlock>>,
    pub files: Vec<FILE>,
    pub exit_code: i32,
}

impl ProcessControlBlockInner {
    pub fn get_trap_context(&self) -> &'static mut TrapContext {
        unsafe {
            (PhysAddr::from(self.trap_context_ppn.clone()).0 as *mut TrapContext).as_mut().unwrap()
        }
    }

    pub fn get_satp(&self) -> usize {
        return self.layout.get_satp();
    }
}

impl ProcessControlBlock {
    pub fn new(elf_data: &[u8]) -> Self {
        let (layout, user_stack_top, entry) = MemLayout::new_elf(elf_data);
        let trap_context_ppn = layout.translate(VirtAddr::from(TRAP_CONTEXT).into()).unwrap().ppn();
        let pid = alloc_pid();
        let kernel_stack = KernelStack::new(&pid);
        let kernel_stack_top = kernel_stack.top();
        let context_ptr = kernel_stack.save_to_top(ProcessContext::init()) as usize;
        let status = ProcessStatus::Ready;

        let stdin = FILE {
            path: Vec::new(),
            ftype: FTYPE::TStdIn,
            fchain: Vec::new(),
            fsize: 0,
            cursor: 0,
            flag: FILE::FMOD_READ,
        };
        let stdout = FILE {
            path: Vec::new(), 
            ftype: FTYPE::TStdOut,
            fchain: Vec::new(),
            fsize: 0,
            cursor: 0,
            flag: FILE::FMOD_WRITE,
        };
        let stderr = FILE {
            path: Vec::new(),
            ftype: FTYPE::TStdErr,
            fchain: Vec::new(),
            fsize: 0,
            cursor: 0,
            flag: FILE::FMOD_WRITE,
        };
        let files = vec![stdin, stdout, stderr];

        let pcb = Self {
            pid,
            kernel_stack,
            inner: Mutex::new(ProcessControlBlockInner {
                context_ptr,
                status,
                layout,
                trap_context_ppn,
                size: user_stack_top,
                up_since: get_time(),
                last_start: 0,
                utime: 0,
                parent: None,       // FIXME: Isn't it PROC0?
                children: Vec::new(),
                files,
                exit_code: 0
            }),
        };
        let trap_context = pcb.get_inner_locked().get_trap_context();
        *trap_context = TrapContext::init(
            entry, 
            user_stack_top, 
            KERNEL_MEM_LAYOUT.lock().get_satp(), 
            kernel_stack_top.0,
            user_trap as usize
        );
        return pcb;
    }

    pub fn fork(self: &Arc<ProcessControlBlock>) -> Arc<ProcessControlBlock> {
        let mut parent_arcpcb = self.get_inner_locked();
        let layout = MemLayout::fork_from_user(&parent_arcpcb.layout);
        let trap_context_ppn = layout.translate(VirtAddr(TRAP_CONTEXT).into()).unwrap().ppn();
        let pid = alloc_pid();
        let kernel_stack = KernelStack::new(&pid);
        let kernel_stack_top = kernel_stack.top();
        let context_ptr = kernel_stack.save_to_top(ProcessContext::init()) as usize;
        let status = ProcessStatus::Ready;
        
        let pcb = Arc::new(ProcessControlBlock {
            pid,
            kernel_stack,
            inner: Mutex::new(ProcessControlBlockInner {
                context_ptr,
                status,
                layout,
                trap_context_ppn,
                size: parent_arcpcb.size,
                up_since: get_time(),
                last_start: 0,
                utime: parent_arcpcb.utime,
                parent: Some(Arc::downgrade(self)),
                children: Vec::new(),
                files: parent_arcpcb.files.clone(),
                exit_code: 0
            }),
        });

        parent_arcpcb.children.push(pcb.clone());
        let mut trap_context: &mut TrapContext = PhysAddr::from(pcb.get_inner_locked().trap_context_ppn).get_mut();
        trap_context.kernel_sp = kernel_stack_top.0;
        return pcb;
    }

    pub fn exec(&self, elf_data: &[u8]) {
        let (layout, user_stack_top, entry) = MemLayout::new_elf(elf_data);
        let trap_context_ppn = layout.translate(VirtAddr::from(TRAP_CONTEXT).into()).unwrap().ppn();
        let mut arcpcb = self.get_inner_locked();
        arcpcb.layout = layout;     // original layout dropped, thus freed.
        arcpcb.trap_context_ppn = trap_context_ppn;
        arcpcb.utime = 0;
        arcpcb.up_since = get_time();
        let trap_context = arcpcb.get_trap_context();
        *trap_context = TrapContext::init(
            entry, 
            user_stack_top, 
            KERNEL_MEM_LAYOUT.lock().get_satp(), 
            self.kernel_stack.top().0, 
            user_trap as usize
        );
    }

    pub fn get_inner_locked(&self) -> MutexGuard<ProcessControlBlockInner> {
        return self.inner.lock();
    }

    pub fn get_trap_context(&self) -> &'static mut TrapContext {
        PhysAddr::from(self.get_inner_locked().trap_context_ppn).get_mut()
    }

    pub fn get_pid(&self) -> usize {
        self.pid.0
    }

    pub fn get_ppid(&self) -> usize {
        let arcpcb = self.get_inner_locked();
        arcpcb.parent.as_ref().unwrap().upgrade().unwrap().get_pid()
    }
}