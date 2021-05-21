// use super::ProcessContext;
use super::ProcessControlBlock;
use super::ProcessStatus;
use crate::trap::TrapContext;

// use crate::config::*;
use core::cell::RefCell;
use lazy_static::*;
use alloc::vec::Vec;
use crate::sbi::get_time;
use alloc::sync::Arc;
use crate::memory::VirtAddr;
use super::{
    dequeue,
    enqueue,
    PROC0
};

global_asm!(include_str!("switch.asm"));

extern "C" {
    pub fn __switch(
        current_task_cx_ptr2: *const usize,
        next_task_cx_ptr2: *const usize
    );
}

pub struct Processor {
    inner: RefCell<ProcessorInner>,
}

struct ProcessorInner {
    current: Option<Arc<ProcessControlBlock>>,
    idle_context_ptr: usize,
}

unsafe impl Sync for Processor {}

// we need to initialize pcbs
lazy_static! {
    pub static ref PROCESSOR0: Processor = Processor::new();
}

impl Processor {
    pub fn new() -> Self {
        Self {
            inner: RefCell::new(ProcessorInner {
                current: None,
                idle_context_ptr: 0
            })
        }
    }

    pub fn take_current(&self) -> Option<Arc<ProcessControlBlock>> {
        return self.inner.borrow_mut().current.take();
    }

    pub fn current(&self) -> Option<Arc<ProcessControlBlock>> {
        return self.inner.borrow().current.as_ref().map(
            |process| {
                return Arc::clone(process);
            }
        );
    }

    pub fn get_idle_context_ptr2(&self) -> *const usize {
        let inner = self.inner.borrow();
        return &inner.idle_context_ptr as *const usize;
    }

    pub fn current_satp(&self) -> usize {
        if let Some(arcpcb) = self.current() {
            return arcpcb.get_inner_locked().get_satp();
        } else {
            panic!("No process is running currently!");
        }
    }

    pub fn switch_proc(&self, proc_context2: *const usize) {
        let idle_context_ptr2 = self.get_idle_context_ptr2();
        unsafe {
            __switch(proc_context2, idle_context_ptr2);
        }
    }

    pub fn suspend_switch(&self) {
        let process = self.take_current().unwrap();
        let mut arcpcb = process.get_inner_locked();
        let context_ptr2 = &(arcpcb.context_ptr) as *const usize;
        arcpcb.status = ProcessStatus::Ready;
        arcpcb.utime = arcpcb.utime + get_time() - arcpcb.last_start;
        drop(arcpcb);
        enqueue(process);
        let idle_context_ptr2 = self.get_idle_context_ptr2();
        unsafe {
            __switch(context_ptr2, idle_context_ptr2);
        }
    }

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
        arcpcb.children.clear();
        arcpcb.layout.drop_all();
        arcpcb.utime = arcpcb.utime + get_time() - arcpcb.last_start;
        drop(arcpcb);
        drop(process);
        let _unused: usize = 0;
        let idle_context_ptr2 = self.get_idle_context_ptr2();
        unsafe {
            __switch((&_unused) as *const usize, idle_context_ptr2);
        }
    }

    pub fn run(&self) {
        loop {
            if let Some(process) = dequeue() {
                let idle_context_ptr2 = self.get_idle_context_ptr2();
                let mut arcpcb = process.get_inner_locked();
                let next_context_ptr2 = &(arcpcb.context_ptr) as *const usize;
                arcpcb.status = ProcessStatus::Running;
                arcpcb.last_start = get_time();
                drop(arcpcb);
                self.inner.borrow_mut().current = Some(process);
                unsafe {
                    __switch(idle_context_ptr2, next_context_ptr2);
                }
            } else {
                warning!("No process to run!");
            }
        }
    }

    pub fn current_up_since(&self) -> u64 {
        let inner = self.inner.borrow();
        if let Some(current) = &inner.current {
            return current.get_inner_locked().up_since;
        } else {
            return 0;
        }
    }

    pub fn current_utime(&self) -> u64 {
        let inner = self.inner.borrow();
        if let Some(current) = &inner.current {
            let arcpcb = current.get_inner_locked();
            return arcpcb.utime + get_time() - arcpcb.last_start;
        } else {
            return 0;
        }
    }

    pub fn current_trap_context(&self) -> &'static mut TrapContext {
        self.current().unwrap().get_inner_locked().get_trap_context()
    }
}