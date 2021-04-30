// use super::ProcessContext;
use super::ProcessControlBlock;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use spin::Mutex;

// use crate::config::*;
use lazy_static::*;
pub struct ProcessManager {
    processes: VecDeque<Arc<ProcessControlBlock>>,
}

unsafe impl Sync for ProcessManager {}

// we need to initialize pcbs
lazy_static! {
    pub static ref PROCESS_MANAGER: Mutex<ProcessManager> = Mutex::new(ProcessManager::new());
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            processes: VecDeque::new()
        }
    }

    pub fn enqueue(&mut self, process: Arc<ProcessControlBlock>) {
        self.processes.push_back(process);
    }

    pub fn dequeue(&mut self) -> Option<Arc<ProcessControlBlock>> {
        if let Some(process) = self.processes.pop_front() {
            return Some(process);
        } else {
            warning!("No process in Process Manager!");
            return None;
        }
    }
}

pub fn enqueue(process: Arc<ProcessControlBlock>) {
    PROCESS_MANAGER.lock().enqueue(process);
}

pub fn dequeue() -> Option<Arc<ProcessControlBlock>> {
    return PROCESS_MANAGER.lock().dequeue();
}