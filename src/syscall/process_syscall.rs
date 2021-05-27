use core::convert::TryInto;

use crate::process::{
    suspend_switch,
    exit_switch,
    current_process,
    enqueue
};

use crate::memory::{
    VirtAddr,
    get_user_cstr,
    translate_user_va
};

use crate::process::{
    current_satp,
    temp_app_loader::get_app,
    ProcessStatus
};

use alloc::sync::Arc;

pub fn sys_yield() -> isize {
    suspend_switch();
    0
}

pub fn sys_exit(code: i32) -> ! {
    info!("Application exited with code {:}", code);
    exit_switch(code);
    unreachable!("This part should be unreachable. Go check __switch.")
}

pub fn sys_fork() -> isize {
    let current_proc = current_process().unwrap();
    let new_proc = current_proc.fork();
    let new_pid = new_proc.pid.0;
    // return 0 for child process in a0
    new_proc.get_inner_locked().get_trap_context().regs[10] = 0;
    enqueue(new_proc);
    return new_pid as isize;
}

// TODO: add argc and argv support
pub fn sys_exec(app_name: VirtAddr) -> isize {
    let app_name = get_user_cstr(current_satp(), app_name);
    if let Some(elf_data) = get_app(app_name.as_str()) {
        let proc = current_process().unwrap();
        proc.exec(elf_data);
        0
    } else {
        error!("No such command or application: {}", app_name);
        -1
    }
}

pub fn sys_waitpid(pid: isize, exit_code_ptr: VirtAddr) -> isize {
    let proc = current_process().unwrap();
    let mut arcpcb = proc.get_inner_locked();
    let mut found: bool = false;
    for (idx, child) in arcpcb.children.iter().enumerate() {
        if pid == -1 || pid as usize == child.get_pid() {
            found = true;
            let child_arcpcb = child.get_inner_locked();
            if child_arcpcb.status == ProcessStatus::Zombie {
                assert_eq!(Arc::strong_count(child), 1, "This child process seems to be referenced more then once.");
                unsafe {*translate_user_va(current_satp(), exit_code_ptr) = child_arcpcb.exit_code;}
                return child.get_pid() as isize;
            }
        }
    }
    return if found {-2} else {-1};
}

pub fn sys_sbrk(sz: isize) -> isize {
    let proc = current_process().unwrap();
    let mut arcpcb = proc.get_inner_locked();
    let original_size = arcpcb.size as isize;
    arcpcb.layout.alter_segment(VirtAddr::from(original_size as usize).into(), VirtAddr::from((original_size + sz) as usize).into());

    return 0;
}
