use crate::process::{
    suspend_switch,
    exit_switch
};

pub fn sys_yield() -> isize {
    suspend_switch();
    0
}

pub fn sys_exit(code: i32) -> ! {
    info!("Application exited with code {:}", code);
    exit_switch();
    unreachable!("This part should be unreachable. Go check __switch.")
}