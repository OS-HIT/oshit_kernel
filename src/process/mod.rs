mod pcb;
mod manager;
pub mod temp_app_loader;
mod pid;
mod kernel_stack;
mod processor;
mod proc0;

pub use pcb::ProcessContext;
pub use pcb::ProcessControlBlock;
pub use pcb::ProcessStatus;
pub use manager::{
    enqueue,
    dequeue,
};
pub use pid::{
    Pid,
    alloc_pid,
};
pub use kernel_stack::{
    kernel_stack_pos,
    KernelStack
};

pub use processor::{
    PROCESSOR0,
};

pub use proc0::{PROC0, init_proc0};
// pub use temp_app_loader::init_app_context;

use crate::trap::TrapContext;
use alloc::sync::Arc;
use alloc::string::String;

pub fn init() {
    debug!("Initializing process control unit...");
    verbose!("Initializing proc0...");
    init_proc0();
    verbose!("Starting hart0...");
    PROCESSOR0.run();
    info!("Process control unit initialized.");
}

pub fn suspend_switch() {
    PROCESSOR0.suspend_switch();
}

pub fn exit_switch(exit_code: i32) {
    PROCESSOR0.exit_switch(exit_code);
}

pub fn current_satp() -> usize {
    return PROCESSOR0.current_satp();
}

pub fn current_up_since() -> u64 {
    return PROCESSOR0.current_up_since();
}

pub fn current_utime() -> u64 {
    return PROCESSOR0.current_utime();
}

pub fn current_trap_context() -> &'static mut TrapContext {
    return PROCESSOR0.current_trap_context();
}

pub fn current_process() -> Option<Arc<ProcessControlBlock>> {      // TODO: Add multi-core support here in these current_* funcs.
    return PROCESSOR0.current();
}

pub fn current_path() -> String {
    return current_process().unwrap().get_inner_locked().path.clone();
}