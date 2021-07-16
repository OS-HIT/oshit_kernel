//! The process manager for oshit kernel

// use super::ProcessContext;
use super::ProcessControlBlock;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use spin::Mutex;

// use crate::config::*;
use lazy_static::*;

/// The ProcessManager of choice: Round Robin.
pub struct ProcessManager {
    processes: VecDeque<Arc<ProcessControlBlock>>,
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