//! Process related syscalls.
use core::mem::size_of;

use crate::process::{PROCESS_MANAGER, current_path, current_process, enqueue, exit_switch, get_proc_by_pid, suspend_switch};

use crate::memory::{
    VirtAddr,
    get_user_cstr,
    // translate_user_va,
    VMAFlags
};

use crate::process::{
    current_satp,
    ProcessStatus,
    SigAction
};
use crate::trap::TrapContext;

use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::ToString;

use crate::fs::{
    File,
    open,
    OpenMode
};

pub const WNOHANG: isize = 1;
pub const WUNTRACED: isize = 2;
pub const WCONTINUED: isize = 4;

/// Give up CPU.
pub fn sys_yield() -> isize {
    suspend_switch();
    0
}

/// Process exit.
pub fn sys_exit(code: i32) -> ! {
    debug!("Application {} exited with code {:}", current_process().unwrap().pid.0, code);
    exit_switch(code);
    unreachable!("This part should be unreachable. Go check __switch.")
}

/// Process fork a copyed version of itself as child 
pub fn sys_fork() -> isize {
    let current_proc = current_process().unwrap();
    let new_proc = current_proc.fork();
    let new_pid = new_proc.pid.0;
    // return 0 for child process in a0
    new_proc.get_inner_locked().get_trap_context().regs[10] = 0;
    enqueue(new_proc);
    return new_pid as isize;
}

/// Process fork a copyed version of itself as child, with more arguments
/// TODO: Finish it.
pub fn sys_clone(_: usize, stack: usize, _: usize, _: usize, _: usize) -> isize {
    let current_proc = current_process().unwrap();
    let new_proc = current_proc.fork();
    let new_pid = new_proc.pid.0;
    // return 0 for child process in a0
    new_proc.get_inner_locked().get_trap_context().regs[10] = 0;
    if stack != 0 {
        new_proc.get_inner_locked().get_trap_context().regs[2] = stack;
    }
    enqueue(new_proc);
    return new_pid as isize;
}

/// Execute a program in the process
pub fn sys_exec(app_name: VirtAddr, argv: VirtAddr, envp: VirtAddr) -> isize {
    let mut app_name = get_user_cstr(current_satp(), app_name);
    if !app_name.starts_with("/") {
        let mut path = current_path();
        path.push_str(app_name.as_str());
        app_name = path;
    }
    verbose!("Exec {}", app_name);

    match open(app_name.clone(), OpenMode::READ) {
        Ok(file) => {
            verbose!("File found {}", app_name);
            let length = file.poll().size as usize;
            let mut v: Vec<u8> = Vec::with_capacity(length);
            v.resize(length, 0);

            match file.read(&mut v) {
                Ok(res) => {
                    verbose!("Loaded App {}, size = {}", app_name, res);
                    let proc = current_process().unwrap();
                    let locked_inner = proc.get_inner_locked();

                    verbose!("Loading argv");
                    let mut args: Vec<Vec<u8>> = Vec::new();
                    if argv.0 != 0 {
                        verbose!("argv @ {:0x}", argv.0);
                        let mut iter = argv;
                        loop {
                            let ptr: usize = locked_inner.layout.read_user_data(iter);
                            if ptr == 0 {
                                break;
                            }
                            args.push(locked_inner.layout.get_user_cstr(ptr.into()));
                            iter += core::mem::size_of::<usize>();
                        }
                    }
                    for (idx, a) in args.iter().enumerate() {
                        verbose!("argc [{}]: {}", idx, core::str::from_utf8(a).unwrap())
                    }

                    verbose!("Loading envp");
                    let mut envs: Vec<Vec<u8>> = Vec::new();
                    if envp.0 != 0 {
                        verbose!("envp @ {:0x}", envp.0);
                        let mut iter = envp;
                        loop {
                            let ptr: usize = locked_inner.layout.read_user_data(iter);
                            if ptr == 0 {
                                break;
                            }
                            envs.push(locked_inner.layout.get_user_cstr(ptr.into()));
                            iter += core::mem::size_of::<usize>();
                        }
                    }
                    drop(locked_inner);
                    proc.exec(&v, app_name, args, envs)
                },
                Err(msg) => {
                    error!("Failed to read file: {}", msg);
                    1
                }
            }
        } ,
        Err(msg) =>{
            error!("Failed to open file {}: {}", app_name, msg);
            -1
        }
    }
}

/// Wait for a pid to end, then return it's exit status.
pub fn sys_waitpid(pid: isize, exit_code_ptr: VirtAddr, options: isize) -> isize {
    loop {
        let proc = current_process().unwrap();
        let mut locked_inner = proc.get_inner_locked();
        for (idx, child) in locked_inner.children.iter().enumerate() {
            if pid == -1 || pid as usize == child.get_pid() {
                if locked_inner.children[idx].get_inner_locked().status == ProcessStatus::Zombie {
                    let child_proc = locked_inner.children.remove(idx);
                    let child_arcpcb = child_proc.get_inner_locked();
                    assert_eq!(Arc::strong_count(&child_proc), 1, "This child process seems to be referenced more then once.");
                    if exit_code_ptr.0 != 0 {
                        // TODO: properly construct wstatus
                        locked_inner.layout.write_user_data(exit_code_ptr, &((child_arcpcb.exit_code as i32) << 8));
                    }
                    debug!("Zombie {} was killed, exit status = {}", child_proc.get_pid(), child_arcpcb.exit_code);
                    return child_proc.get_pid() as isize;
                }
            }
        }
        if options & WNOHANG != 0 {
            return 0;
        } else {
            drop(locked_inner);
            suspend_switch();
        }
    }
}

/// Get pid of itself.
pub fn sys_getpid() -> isize {
    return current_process().unwrap().get_pid() as isize;
}

/// Get pid of it's parent
pub fn sys_getppid() -> isize {
    return current_process().unwrap().get_ppid() as isize;
}

/// Get current working directory of the process.
pub fn sys_getcwd(buf: VirtAddr, size: usize) -> isize {
    if buf.0 == 0 {
        return 0;
    }

    let proc = current_process().unwrap();
    let locked_inner = proc.get_inner_locked();
    let mut buffer = locked_inner.layout.get_user_buffer(buf, size);
    buffer.write_bytes(locked_inner.path.as_bytes(), 0);
    return buf.0 as isize;
}

/// Change the current working directory.
pub fn sys_chdir(buf: VirtAddr) -> isize {
    verbose!("chdir start");
    let proc = current_process().unwrap();
    let mut locked_inner = proc.get_inner_locked();
    if let Ok (dir_str) = core::str::from_utf8(&locked_inner.layout.get_user_cstr(buf)) {
        if let Ok (_) = open(dir_str.to_string(), OpenMode::READ) {
            locked_inner.path = dir_str.to_string();
            return 0;
        } else {
            error!("No such directory!");
            return -1;
        }
    } else {
        error!("Invalid charactor in chdir");
        return -1;
    }
}

pub fn sys_sbrk(sz: usize) -> isize {
    verbose!("Brk sz: {}", sz);
    if sz == 0 {
        return current_process().unwrap().get_inner_locked().size as isize;
    }
    let proc = current_process().unwrap();
    let mut locked_inner = proc.get_inner_locked();
    let original_size = locked_inner.size;
    locked_inner.layout.alter_segment(VirtAddr::from(original_size).to_vpn_ceil(), VirtAddr::from(sz).to_vpn_ceil());
    locked_inner.size = sz as usize;
    return 0;
}

pub fn sys_mmap(start: VirtAddr, len: usize, prot: u8, _: usize, fd: usize, offset: usize) -> isize {
    verbose!("sys_mmap");
    let proc = current_process().unwrap();
    let mut locked_inner = proc.get_inner_locked();
    if let Some(file) = locked_inner.files[fd].clone() {
        if let Ok(addr) = locked_inner.layout.add_vma(file, start, VMAFlags::from_bits(prot << 1).unwrap(), offset, len) {
            return addr.0 as isize;
        } 
    }
    -1
}

pub fn sys_munmap(start: VirtAddr, len: usize) -> isize {
    verbose!("sys_munmap");
    let proc = current_process().unwrap();
    let mut locked_inner = proc.get_inner_locked();
    match locked_inner.layout.drop_vma(start.into(), (start + len).to_vpn_ceil()) {
        Ok(()) => 0,
        Err(msg) => {
            error!("munmap failed: {}", msg);
            -1
        }
    }
}

pub fn sys_kill(target_pid: usize, signal: usize) -> isize {
    if let Some(proc) = get_proc_by_pid(target_pid) {
        let mut locked_inner = proc.get_inner_locked();
        locked_inner.pending_sig.push_back(signal);
        0
    } else {
        error!("No such process with pid {}, failed to send signal", target_pid);
        -1
    }
}

// TODO: consider edge cases of act is nullptr
// TODO: reference to https://elixir.bootlin.com/linux/latest/source/kernel/signal.c#L4015 (do_sigaction), implement reporting unsupport
pub fn sys_sigaction(signum: usize, act: VirtAddr, oldact: VirtAddr) -> isize {
    let proc = current_process().unwrap();
    let mut locked_inner = proc.get_inner_locked();
    
    let new_act: SigAction = locked_inner.layout.read_user_data(act);
    let old_act_op = locked_inner.handlers.insert(signum, new_act);

    if oldact.0 != 0 {
        if let Some(mut old_act) = old_act_op {
            old_act.mask = locked_inner.sig_mask;
            locked_inner.layout.write_user_data(oldact, &old_act);
        } else {
            return -1;
        }
    }

    0
}

pub const SIG_BLOCK     : isize = 0;
pub const SIG_UNBLOCK   : isize = 1;
pub const SIG_SETMASK   : isize = 2;

pub fn sys_sigprocmask(how: isize, oldmask: VirtAddr, newmask: VirtAddr) -> isize {
    let proc = current_process().unwrap();
    let locked_inner = proc.get_inner_locked();
    if oldmask.0 != 0 {
        locked_inner.layout.write_user_data(oldmask, &locked_inner.sig_mask);
    }

    let new_mask: u64 = locked_inner.layout.read_user_data(newmask);

    if how == SIG_BLOCK {
        locked_inner.sig_mask |= new_mask;
    } else if how == SIG_UNBLOCK {
        locked_inner.sig_mask &= !new_mask;
    } else if how == SIG_SETMASK {
        locked_inner.sig_mask = new_mask;
    } else {
        return -1;
    }

    0
}

pub fn sys_sigreturn() -> isize {
    // go check trap.asm -> __restore_to_signal_handler
    let proc = current_process().unwrap();
    let locked_inner = proc.get_inner_locked();
    // reg2 (x2) is sp
    let old_trap_context: TrapContext = locked_inner.layout.read_user_data((locked_inner.get_trap_context().regs[2] - size_of::<TrapContext>()).into());
    locked_inner.write_trap_context(&old_trap_context);
    0
}