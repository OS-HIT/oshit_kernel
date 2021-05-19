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
use core::convert::TryInto;

pub fn sys_yield() -> isize {
    suspend_switch();
    0
}

pub fn sys_exit(code: i32) -> ! {
    debug!("Application {} exited with code {:}", current_process().unwrap().pid.0, code);
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
    let mut cand_idx = 0;
    for (idx, child) in arcpcb.children.iter().enumerate() {
        if pid == -1 || pid as usize == child.get_pid() {
            found = true;
            cand_idx = idx;
        }
    }
    if found {
        if arcpcb.children[cand_idx].get_inner_locked().status == ProcessStatus::Zombie {
            let child_proc = arcpcb.children.remove(cand_idx);
            let child_arcpcb = child_proc.get_inner_locked();
            assert_eq!(Arc::strong_count(&child_proc), 1, "This child process seems to be referenced more then once.");
            unsafe {*translate_user_va(arcpcb.layout.get_satp(), exit_code_ptr) = child_arcpcb.exit_code;}
            debug!("Zombie {} was killed.", child_proc.get_pid());
            return child_proc.get_pid() as isize;
        }
    }
    return if found {-2} else {-1};
}

pub fn sys_getpid() -> isize {
    return current_process().unwrap().get_pid() as isize;
}

pub fn sys_getppid() -> isize {
    return current_process().unwrap().get_ppid() as isize;
}