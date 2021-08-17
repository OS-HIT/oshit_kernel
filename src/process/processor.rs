//! Abstract of the Processor, for future multi-core support.
// use super::ProcessContext;
use super::ProcessControlBlock;
use super::ProcessStatus;
use crate::trap::TrapContext;

// use crate::config::*;
use core::cell::RefCell;
use alloc::sync::Weak;
use lazy_static::*;
use crate::sbi::get_time;
use alloc::sync::Arc;
use super::{
    dequeue,
    enqueue,
    PROC0
};

global_asm!(include_str!("switch.asm"));

extern "C" {
    /// The `__switch()` function for switching kernel execution flow.
    pub fn __switch(
        current_task_cx_ptr2: *const usize,
        next_task_cx_ptr2: *const usize
    );
}

/// Processor struct, Abstract representation of a Processor
pub struct Processor {
    /// Mutable member of the processor.
    inner: RefCell<ProcessorInner>,
}


/// Mutable member of the processor.
struct ProcessorInner {
    /// The current process that is being execute.
    current: Option<Arc<ProcessControlBlock>>,
    /// Idle ProcessContext work flow context pointer, used to determin next process.
    idle_context_ptr: usize,
}

unsafe impl Sync for Processor {}

lazy_static! {
    /// The singleton of hart0. Multi-core in the future.
    pub static ref PROCESSOR0: Processor = Processor::new();
}

impl Processor {
    /// Constructor for the processor
    /// # Returns
    /// A empty processor struct.
    pub fn new() -> Self {
        Self {
            inner: RefCell::new(ProcessorInner {
                current: None,
                idle_context_ptr: 0
            })
        }
    }

    /// Take out current process, leaving a None inside.
    pub fn take_current(&self) -> Option<Arc<ProcessControlBlock>> {
        return self.inner.borrow_mut().current.take();
    }

    /// get a reference of the current process's pcb.
    pub fn current(&self) -> Option<Arc<ProcessControlBlock>> {
        return self.inner.borrow().current.as_ref().map(
            |process| {
                return Arc::clone(process);
            }
        );
    }

    /// Get the pointer pointing at the context ptr.
    /// By manipulating the contextn ptr, we can switch work flow.
    pub fn get_idle_context_ptr2(&self) -> *const usize {
        let inner = self.inner.borrow();
        return &inner.idle_context_ptr as *const usize;
    }

    /// Get current process's user memory space pagetable SATP
    /// # Description
    /// Get current process's user memory space pagetable SATP.  
    /// Note that this function trys to lock current process, so can cause dead lock if the lock is already held.
    pub fn current_satp(&self) -> usize {
        if let Some(arcpcb) = self.current() {
            return arcpcb.get_inner_locked().get_satp();
        } else {
            panic!("No process is running currently!");
        }
    }

    /// Switch executing process.  
    /// # Description
    /// By switching to the idle work flow, we can find what process to run next.
    // pub fn switch_proc(&self, proc_context2: *const usize) {
    //     let idle_context_ptr2 = self.get_idle_context_ptr2();
    //     unsafe {
    //         __switch(proc_context2, idle_context_ptr2);
    //     }
    // }

        
    /// suspend current process and switch.
    /// # Description
    /// Suspend current process and switch to another.  
    /// Note that we need to drop locks before calling this method, to avoid potential dead lock on shared resources.
    pub fn suspend_switch(&self) {
        let process = self.take_current().unwrap();
        let mut arcpcb = process.get_inner_locked();
        let context_ptr2 = &(arcpcb.context_ptr) as *const usize;
        arcpcb.status = ProcessStatus::Ready;
        arcpcb.timer_prof_now += get_time() - arcpcb.timer_real_start;
        drop(arcpcb);
        enqueue(process);
        let idle_context_ptr2 = self.get_idle_context_ptr2();
        unsafe {
            __switch(context_ptr2, idle_context_ptr2);
        }
    }

    /// Exit current process and switch
    /// # Description
    /// Exit current process and switch, can be used to terminate process in kernel.
    pub fn exit_switch(&self, exit_code: i32) {
        let process = self.take_current().unwrap();
        let mut arcpcb = process.get_inner_locked();
        arcpcb.status = ProcessStatus::Zombie;
        arcpcb.exit_code = exit_code;
            
        {
            let mut initproc_inner = PROC0.get_inner_locked();
            for child in arcpcb.children.iter() {
                child.get_inner_locked().parent = Some(Arc::downgrade(&PROC0));
                initproc_inner.children.push(child.clone());
            }
        }

        {
            if let Some(parent_proc) = Weak::upgrade(&arcpcb.parent.clone().unwrap()) {
                let mut parent_locked_inner = parent_proc.get_inner_locked();
                parent_locked_inner.dead_children_stime += get_time() - arcpcb.up_since;
                parent_locked_inner.dead_children_utime += get_time() - arcpcb.utime;
            }
        }
        
        arcpcb.children.clear();
        arcpcb.layout.drop_all();
        arcpcb.timer_prof_now += get_time() - arcpcb.timer_real_start;
        drop(arcpcb);
        drop(process);
        let _unused: usize = 0;
        let idle_context_ptr2 = self.get_idle_context_ptr2();
        unsafe {
            __switch((&_unused) as *const usize, idle_context_ptr2);
        }
    }

    /// Find next process to run.
    /// # description
    /// Find next process to run. The idle work flow will run this function indefinitly.
    pub fn run(&self) {
        loop {
            if let Some(process) = dequeue() {
                let idle_context_ptr2 = self.get_idle_context_ptr2();
                let mut arcpcb = process.get_inner_locked();
                let next_context_ptr2 = &(arcpcb.context_ptr) as *const usize;
                arcpcb.status = ProcessStatus::Running;
                if arcpcb.timer_real_next != 0 && arcpcb.timer_real_next < get_time() {
                    if arcpcb.timer_real_int != 0 {
                        arcpcb.timer_real_next += crate::config::CLOCK_FREQ / 1000 * arcpcb.timer_real_int;
                    } else {
                        arcpcb.timer_real_next = 0;
                    }
                    arcpcb.recv_signal(super::default_handlers::SIGALRM);
                }
                if arcpcb.timer_virt_next != 0 && arcpcb.timer_virt_next < arcpcb.utime {
                    if arcpcb.timer_virt_int != 0 {
                        arcpcb.timer_virt_next += crate::config::CLOCK_FREQ / 1000 * arcpcb.timer_virt_int;
                    } else {
                        arcpcb.timer_virt_next = 0;
                    }
                    arcpcb.recv_signal(super::default_handlers::SIGVTALRM);
                }
                if arcpcb.timer_prof_next != 0 && arcpcb.timer_prof_next < arcpcb.timer_prof_now {
                    if arcpcb.timer_prof_int != 0 {
                        arcpcb.timer_prof_next += crate::config::CLOCK_FREQ / 1000 * arcpcb.timer_prof_int;
                    } else {
                        arcpcb.timer_prof_next = 0;
                    }
                    arcpcb.recv_signal(super::default_handlers::SIGPROF);
                }
                arcpcb.timer_real_start = get_time();
                drop(arcpcb);
                self.inner.borrow_mut().current = Some(process);
                unsafe {
                    __switch(idle_context_ptr2, next_context_ptr2);
                }
            } else {
                warning!("No process to run! Check if the proc0 is dead?");
            }
        }
    }

    /// Get current process's execution time
    pub fn current_up_since(&self) -> u64 {
        let inner = self.inner.borrow();
        if let Some(current) = &inner.current {
            return current.get_inner_locked().up_since;
        } else {
            return 0;
        }
    }

    /// Get current process's execution utime
    pub fn current_utime(&self) -> u64 {
        let inner = self.inner.borrow();
        if let Some(current) = &inner.current {
            let arcpcb = current.get_inner_locked();
            return arcpcb.utime + get_time() - arcpcb.last_start;
        } else {
            return 0;
        }
    }

    /// Get current process's TrapContext
    /// # Description
    /// Get current process's TrapContext
    /// Note that this function trys to lock current process, so can cause dead lock if the lock is already held.
    pub fn current_trap_context(&self) -> &'static mut TrapContext {
        self.current().unwrap().get_inner_locked().get_trap_context()
    }
}