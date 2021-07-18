//! The process control module for oshit kernel
#![allow(unused)]
mod pcb;
mod manager;
mod pid;
mod kernel_stack;
mod processor;
mod proc0;
pub mod default_handlers;
pub mod kernel_stored_app_loader;

pub use pcb::{
    ProcessContext,
    ProcessControlBlock,
    ProcessStatus,
    SignalFlags,
    default_sig_handlers,
    SigAction
};
pub use manager::{
    enqueue,
    dequeue,
    get_proc_by_pid,
    PROCESS_MANAGER,
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

/// initialize the process control unit.
pub fn init() {
    debug!("Initializing process control unit...");
    verbose!("Initializing proc0...");
    init_proc0();
    verbose!("Starting hart0...");
    PROCESSOR0.run();
    info!("Process control unit initialized.");
}

/// suspend current process and switch.
/// # Description
/// Suspend current process and switch to another.  
/// Note that we need to drop locks before calling this method, to avoid potential dead lock on shared resources.
pub fn suspend_switch() {
    PROCESSOR0.suspend_switch();
}

/// Exit current process and switch
/// # Description
/// Exit current process and switch, can be used to terminate process in kernel.
pub fn exit_switch(exit_code: i32) {
    PROCESSOR0.exit_switch(exit_code);
}

/// Get current process's user memory space pagetable SATP
/// # Description
/// Get current process's user memory space pagetable SATP.  
/// Note that this function trys to lock current process, so can cause dead lock if the lock is already held.
pub fn current_satp() -> usize {
    return PROCESSOR0.current_satp();
}

/// Get current process's execution time
/// # Description
/// Get current process's execution time
/// Note that this function trys to lock current process, so can cause dead lock if the lock is already held.
pub fn current_up_since() -> u64 {
    return PROCESSOR0.current_up_since();
}

/// Get current process's execution utime
/// # Description
/// Get current process's execution utime
/// Note that this function trys to lock current process, so can cause dead lock if the lock is already held.
pub fn current_utime() -> u64 {
    return PROCESSOR0.current_utime();
}


/// Get current process's TrapContext
/// # Description
/// Get current process's TrapContext
/// Note that this function trys to lock current process, so can cause dead lock if the lock is already held.
pub fn current_trap_context() -> &'static mut TrapContext {
    return PROCESSOR0.current_trap_context();
}

/// Get current process
pub fn current_process() -> Option<Arc<ProcessControlBlock>> {      // TODO: Add multi-core support here in these current_* funcs.
    return PROCESSOR0.current();
}

/// Get current process's path
/// # Description
/// Get current process's path
/// Note that this function trys to lock current process, so can cause dead lock if the lock is already held.
pub fn current_path() -> String {
    return current_process().unwrap().get_inner_locked().path.clone();
}