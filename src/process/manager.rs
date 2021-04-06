// use super::ProcessContext;
use super::ProcessControlBlock;
use super::ProcessStatus;
use crate::trap::TrapContext;

// use crate::config::*;
use core::cell::RefCell;
use lazy_static::*;
use super::temp_app_loader::{get_app_count, get_app_data};
use alloc::vec::Vec;

global_asm!(include_str!("switch.asm"));

extern "C" {
    pub fn __switch(
        current_task_cx_ptr2: *const usize,
        next_task_cx_ptr2: *const usize
    );
}

pub struct ProcessManager {
    num_app: usize,
    inner: RefCell<ProcessManagerInner>,
}

struct ProcessManagerInner {
    processes: Vec<ProcessControlBlock>,
    current_process: usize,
}

unsafe impl Sync for ProcessManager {}

// we need to initialize pcbs
lazy_static! {
    pub static ref PROCESS_MANAGER: ProcessManager = {
        verbose!("Initializing process manager...");
        let num_app = get_app_count();
        let mut processes = Vec::new();
        for i in 0..num_app {
            processes.push(
                ProcessControlBlock::new(get_app_data(i), i)
            );
        }

        ProcessManager {
            num_app,
            inner: RefCell::new(
                ProcessManagerInner {
                    processes,
                    current_process: 0
                }
            )
        }
    };
}

impl ProcessManager {
    fn run_first_app(&self) {
        self.inner.borrow_mut().processes[0].status = ProcessStatus::Running;
        let next_proc_context_p = self.inner.borrow().processes[0].context_ptr;
        let _unused: usize = 0;
        unsafe {
            __switch(
                &_unused as * const _, 
                &next_proc_context_p
            );
        }
    }

    fn set_proc_status(&self, id: usize, new_stat: ProcessStatus) {
        self.inner.borrow_mut().processes[id].status = new_stat;
    }

    fn set_current_status(&self, new_stat: ProcessStatus) {
        let current = self.inner.borrow().current_process;
        self.set_proc_status(current, new_stat);
    }

    fn yield_current(&self) {
        self.set_current_status(ProcessStatus::Ready);
    }

    fn exit_current(&self) {
        self.set_current_status(ProcessStatus::Dead);
    }

    // return Option<id>
    fn find_nxt_available(&self) -> Option<usize> {
        let inner = self.inner.borrow();
        for i in 1..self.num_app + 1 {
            let id = (inner.current_process + i) % self.num_app;
            if inner.processes[id].status == ProcessStatus::Ready {
                return Some(id);
            }
        }
        return None;
    }

    fn next_proc(&self) {
        if let Some(nxt) = self.find_nxt_available() {
            self.set_proc_status(nxt, ProcessStatus::Running);
            let mut inner = self.inner.borrow_mut();
            let current = inner.current_process;
            inner.current_process = nxt;
            let current_context : *const usize = &inner.processes[current].context_ptr;
            let nxt_context : *const usize = &inner.processes[nxt].context_ptr;
            core::mem::drop(inner);
            unsafe {
                __switch(
                    current_context,
                    nxt_context 
                );
            }
        } else {
            panic!("All proc fin.")
        }
    }

    fn get_current_satp(&self) -> usize {
        let inner = self.inner.borrow();
        let current = inner.current_process;
        return inner.processes[current].get_satp();
    }

    fn get_current_trap_context(&self) -> &'static mut TrapContext {
        let inner = self.inner.borrow();
        let current = inner.current_process;
        return inner.processes[current].get_trap_context();
    }
}

pub fn run_first_app() {
    PROCESS_MANAGER.run_first_app();
}

pub fn next_proc() {
    PROCESS_MANAGER.next_proc();
}

pub fn yield_current() {
    PROCESS_MANAGER.yield_current();
}

pub fn exit_current() {
    PROCESS_MANAGER.exit_current();
}

pub fn get_current_satp() -> usize {
    PROCESS_MANAGER.get_current_satp()
}

pub fn get_current_trap_context() -> &'static mut TrapContext {
    PROCESS_MANAGER.get_current_trap_context()
}

pub fn suspend_switch() {
    yield_current();
    next_proc();
}

pub fn exit_switch() {
    exit_current();
    next_proc();
}