mod range;
mod mem_op;

pub use range::{
    StepByOne,
    SimpleRange,
};

pub use mem_op::{
    strcpy,
    strlen
};



use crate::config::PAGE_SIZE;
use crate::config::KERNEL_STACK_SIZE;
use crate::config::TRAMPOLINE;
#[allow(unused)]
pub fn print_kernel_stack() {
    if let Some(cp) = crate::process::current_process() {
        let pid = cp.pid.0 as usize;
        let top = TRAMPOLINE - pid * (KERNEL_STACK_SIZE + PAGE_SIZE);
        let bottom = top - KERNEL_STACK_SIZE;
        let mut sp:usize = 0;
        unsafe {
            llvm_asm!(
                "mv $0, sp"
                : "=r"(sp)
                :::"volatile" 
            );
        }
        let mask = 0x7fffffffffusize;
        debug!("kstack of {}: {:#10X}-{:#10X}-{:#10X} used:{} left:{}", pid, bottom&mask, sp&mask, top&mask, top - sp, sp - bottom);
    } else {
        debug!("print_kernel_stack: No user process");
    }
}