mod pcb;
mod manager;
mod temp_app_loader;
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
    PROCESSOR0
};

pub use proc0::{PROC0, init_proc0};
// pub use temp_app_loader::init_app_context;

pub fn suspend_switch() {
    PROCESSOR0.suspend_switch();
}

pub fn exit_switch(exit_code: i32) {

}