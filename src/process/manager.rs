//! The process manager for oshit kernel

// use super::ProcessContext;
use super::{ProcessControlBlock, ProcessStatus, current_process};
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use spin::Mutex;

// use crate::config::*;
use lazy_static::*;

/// The ProcessManager of choice: Round Robin.
pub struct ProcessManager {
    pub processes: VecDeque<Arc<ProcessControlBlock>>,
}

unsafe impl Sync for ProcessManager {}

// we need to initialize pcbs
lazy_static! {
    /// The singleton of the round-robin process manager.
    pub static ref PROCESS_MANAGER: Mutex<ProcessManager> = Mutex::new(ProcessManager::new());
}

impl ProcessManager {
    /// Construct a new ProcessManager
    pub fn new() -> Self {
        Self {
            processes: VecDeque::new()
        }
    }

    /// enqueue a new process, i.e. mark it ready and is waiting for execution.
    pub fn enqueue(&mut self, process: Arc<ProcessControlBlock>) {
        self.processes.push_back(process);
    }

    /// dequeue a new process, i.e. it's either running or dead.
    pub fn dequeue(&mut self) -> Option<Arc<ProcessControlBlock>> {
        if let Some(process) = self.processes.pop_front() {
            return Some(process);
        } else {
            warning!("No process in Process Manager!");
            return None;
        }
    }

    pub fn get_idle_proc_by_pid(&self, pid: usize) -> Option<Arc<ProcessControlBlock>> {
        for proc in &self.processes {
            if proc.pid.0 == pid {
                return Some(proc.clone())
            }
        }
        None
    }

    pub fn remove_proc_by_pid(&mut self, pid: usize) -> Option<Arc<ProcessControlBlock>> {
        let proc_count = self.processes.len();
        for i in 0..proc_count {
            let proc = self.processes.pop_back()?;
            if proc.pid.0 != pid {
                self.processes.push_front(proc);
            } else {
                return Some(proc);
            }
        }
        None
    }
}

/// enqueue a new process, i.e. mark it ready and is waiting for execution.  
/// Use locked to access the manager, to prevent data racing.
pub fn enqueue(process: Arc<ProcessControlBlock>) {
    PROCESS_MANAGER.lock().enqueue(process);
}

/// dequeue a new process, i.e. it's either running or dead.
/// Use locked to access the manager, to prevent data racing.
pub fn dequeue() -> Option<Arc<ProcessControlBlock>> {
    return PROCESS_MANAGER.lock().dequeue();
}

pub fn get_proc_by_pid(pid: usize) -> Option<Arc<ProcessControlBlock>> {
    PROCESS_MANAGER
        .lock()
        .get_idle_proc_by_pid(pid)
        .map_or(
            if current_process().unwrap().pid.0 == pid {
                current_process()
            } else {
                None
            }, 
            |found| {
                Some(found.clone())
            }
        )
}

pub fn remove_proc_by_pid(pid: usize) -> Option<Arc<ProcessControlBlock>> {
    PROCESS_MANAGER.lock().remove_proc_by_pid(pid)
}