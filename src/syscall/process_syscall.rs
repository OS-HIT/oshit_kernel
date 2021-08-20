//! Process related syscalls.
use core::mem::size_of;
use core::slice::{from_raw_parts, from_raw_parts_mut};
use crate::process::{PROC0, ProcessControlBlockInner, remove_proc_by_pid};

use crate::config::PAGE_SIZE;
use crate::config::CLOCK_FREQ;
use crate::process::{CloneFlags, PROCESS_MANAGER, current_path, current_process, enqueue, exit_switch, get_proc_by_pid, suspend_switch};

use crate::memory::{PhysAddr, Segment, VMAFlags, VirtAddr, alloc_continuous, get_user_cstr, SegmentFlags, PTEFlags};

use crate::process::{
    current_satp,
    ProcessStatus,
    SigAction
};
use crate::sbi::get_time;
use crate::trap::TrapContext;

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use bit_field::BitField;
use spin::{Mutex, MutexGuard};

use crate::fs::{
    File,
    open,
    OpenMode
};

pub const WNOHANG: isize = 1;
pub const WUNTRACED: isize = 2;
pub const WCONTINUED: isize = 4;



pub const PROT_READ		    :usize = 0x1		;/* page can be read */
pub const PROT_WRITE	    :usize = 0x2		;/* page can be written */
pub const PROT_EXEC		    :usize = 0x4		;/* page can be executed */
pub const PROT_SEM		    :usize = 0x8		;/* page may be used for atomic ops */
pub const PROT_NONE		    :usize = 0x0		;/* page can not be accessed */
pub const PROT_GROWSDOWN    :usize = 0x01000000	;/* mprotect flag: extend change to start of growsdown vma */
pub const PROT_GROWSUP	    :usize = 0x02000000	;/* mprotect flag: extend change to end of growsup vma */

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
#[deprecated]
pub fn sys_fork() -> isize {
    let current_proc = current_process().unwrap();
    let new_proc = current_proc.fork(CloneFlags::from_bits_truncate(0));
    let new_pid = new_proc.pid.0;
    // return 0 for child process in a0
    new_proc.get_inner_locked().get_trap_context().regs[10] = 0;
    enqueue(new_proc);
    return new_pid as isize;
}

/// Process fork a copyed version of itself as child, with more arguments
/// TODO: Finish it.
pub fn sys_clone(clone_flags: CloneFlags, stack: usize, parent_tid_ptr: VirtAddr, _tls: usize, child_tid_ptr: VirtAddr) -> isize {
    let current_proc = current_process().unwrap();
    let new_proc = current_proc.fork(clone_flags);
    let new_pid = new_proc.pid.0;
    // return 0 for child process in a0
    new_proc.get_inner_locked().get_trap_context().regs[10] = 0;
    if stack != 0 {
        new_proc.get_inner_locked().get_trap_context().regs[2] = stack;
    }
    if clone_flags.contains(CloneFlags::PARENT_SETTID) {
        current_proc.get_inner_locked().layout.write_user_data(parent_tid_ptr, &current_proc.tgid);
    }
    if clone_flags.contains(CloneFlags::CHILD_SETTID) {
        new_proc.get_inner_locked().layout.write_user_data(child_tid_ptr, &current_proc.tgid);
    }
    if clone_flags.contains(CloneFlags::CHILD_CLEARTID) {
        new_proc.get_inner_locked().layout.write_user_data(child_tid_ptr, &(0 as usize));
    }
    // new_proc.get_inner_locked().layout.print_layout();
    enqueue(new_proc);
    
    return new_pid as isize;
}

pub fn sys_set_tid_address(tidptr: VirtAddr) -> isize {
    let current_proc = current_process().unwrap();
    let locked_inner = current_proc.get_inner_locked();
    locked_inner.layout.write_user_data(tidptr, &current_proc.pid.0);
    return current_proc.pid.0 as isize;
}

/// Execute a program in the process
pub fn sys_exec(app_path_ptr: VirtAddr, argv: VirtAddr, envp: VirtAddr) -> isize {
    let mut app_path = get_user_cstr(current_satp(), app_path_ptr);
    if !app_path.starts_with("/") {
        let mut path = current_path();
        path.push_str(app_path.as_str());
        app_path = path;
    }
    if app_path.starts_with("//") {
        app_path = app_path.get(1..).unwrap().to_string();
    }
    verbose!("Exec {}", app_path);

    match sys_exec_inner(app_path, argv, envp) {
        Ok(_) => {
            // current_process().unwrap().get_inner_locked().layout.print_layout();
            0
        },
        Err(msg) => {
            error!("Exec failed: {}", msg);
            -1
        }
    }

    // match open(app_name.clone(), OpenMode::READ) {
    //     Ok(file) => {
    //         verbose!("File found {}", app_name);
    //         let length = file.poll().size as usize;
    //         // alloc continious pages
    //         let page_holder = alloc_continuous(length / PAGE_SIZE + 1);
    //         let head_addr: PhysAddr = page_holder[0].ppn.into();
    //         let head_ptr = head_addr.0 as *mut u8;
    //         let arr: &mut [u8] = unsafe {
    //             from_raw_parts_mut(head_ptr, length)
    //         };

    //         match file.read(arr) {
    //             Ok(res) => {
    //                 verbose!("Loaded App {}, size = {}", app_name, res);

    //                 let proc = current_process().unwrap();
    //                 let locked_inner = proc.get_inner_locked();

    //                 verbose!("Loading argv");
    //                 let mut args: Vec<Vec<u8>> = Vec::new();

    //                 if argv.0 != 0 {
    //                     verbose!("argv @ {:0x}", argv.0);
    //                     let mut iter = argv;
    //                     loop {
    //                         let ptr: usize = locked_inner.layout.read_user_data(iter);
    //                         if ptr == 0 {
    //                             break;
    //                         }
    //                         args.push(locked_inner.layout.get_user_cstr(ptr.into()));
    //                         iter += core::mem::size_of::<usize>();
    //                     }
    //                 }
    //                 for (idx, a) in args.iter().enumerate() {
    //                     verbose!("argc [{}]: {}", idx, core::str::from_utf8(a).unwrap());
    //                 }

    //                 verbose!("Loading envp");
    //                 let mut envs: Vec<Vec<u8>> = Vec::new();
    //                 if envp.0 != 0 {
    //                     verbose!("envp @ {:0x}", envp.0);
    //                     let mut iter = envp;
    //                     loop {
    //                         let ptr: usize = locked_inner.layout.read_user_data(iter);
    //                         if ptr == 0 {
    //                             break;
    //                         }
    //                         envs.push(locked_inner.layout.get_user_cstr(ptr.into()));
    //                         iter += core::mem::size_of::<usize>();
    //                     }
    //                 }
    //                 for (idx, a) in envs.iter().enumerate() {
    //                     verbose!("envp [{}]: {}", idx, core::str::from_utf8(a).unwrap());
    //                 }
    //                 drop(locked_inner);
    //                 proc.exec(arr, app_name, args, envs)
    //             },
    //             Err(msg) => {
    //                 error!("Failed to read file: {}", msg);
    //                 1
    //             }
    //         }
    //     } ,
    //     Err(msg) =>{
    //         error!("Failed to open file {}: {}", app_name, msg);
    //         -1
    //     }
    // }
}

fn sys_exec_inner(app_path: String, argv_ptr: VirtAddr, envp_ptr: VirtAddr) -> Result<isize, &'static str> {
    let current_proc = current_process().unwrap();
    let locked_inner = current_proc.get_inner_locked();

    let argv = load_args(&locked_inner, argv_ptr);
    let envp = load_args(&locked_inner, envp_ptr);

    drop(locked_inner);
    do_exec(app_path, argv, envp)
}

fn load_args(locked_inner: &MutexGuard<ProcessControlBlockInner>, start_ptr: VirtAddr) -> Vec<Vec<u8>> {
    let mut args: Vec<Vec<u8>> = Vec::new();
    if start_ptr.0 != 0 {
        let mut iter = start_ptr;
        loop {
            let ptr: usize = locked_inner.layout.read_user_data(iter);
            if ptr == 0 {
                break;
            }
            args.push(locked_inner.layout.get_user_cstr(ptr.into()));
            iter += core::mem::size_of::<usize>();
        }
    }
    args
}

fn do_exec(mut app_path: String, argv: Vec<Vec<u8>>, envp: Vec<Vec<u8>>) -> Result<isize, &'static str> {
    let elf_file = open(app_path.clone(), OpenMode::READ)?;
    verbose!("File found {}", app_path);
    let length = elf_file.poll().size as usize;
    // alloc continious pages
    let page_holder = alloc_continuous(length / PAGE_SIZE + 1);
    let head_addr: PhysAddr = page_holder[0].ppn.into();
    let head_ptr = head_addr.0 as *mut u8;
    let arr: &mut [u8] = unsafe {
        from_raw_parts_mut(head_ptr, length)
    };
    elf_file.read(arr)?;

    if arr.len() >= 2 && arr[0] == b'#' && arr[1] == b'!' {
        let mut vdq_argv: VecDeque<Vec<u8>> = VecDeque::from(argv);
        vdq_argv.pop_front();
        vdq_argv.push_front(arr.to_vec());
        let mut b_app_path: Vec<u8> = Vec::new();
        let mut b_addi_arg: Vec<Vec<u8>> = Vec::new();
        enum FSMState {
            Name,
            Space,
            Args,
            Fin
        }
        let mut state: FSMState = FSMState::Name;
        for b in arr[2..].iter() {
            match state {
                FSMState::Name => {
                    match *b {
                        b' ' => {
                            state = FSMState::Space;
                        },
                        b'\n' => {
                            state = FSMState::Fin;
                        },
                        good => {
                            b_app_path.push(good);
                        }
                    }
                },
                FSMState::Space => {
                    match *b {
                        b' ' => {
                            state = FSMState::Space;
                        },
                        b'\n' => {
                            state = FSMState::Fin;
                        },
                        good => {
                            // b_app_path.push(good);
                            b_addi_arg.push(Vec::new());
                            b_addi_arg.last_mut().unwrap().push(good);
                            state = FSMState::Args;
                        }
                    }
                },
                FSMState::Args => {
                    match *b {
                        b' ' => {
                            state = FSMState::Space;
                        },
                        b'\n' => {
                            state = FSMState::Fin;
                        },
                        good => {
                            // b_app_path.push(good);
                            b_addi_arg.last_mut().unwrap().push(good);
                            state = FSMState::Args;
                        }
                    }
                }
                FSMState::Fin => {
                    // HACK: No this shouldn't be right
                    vdq_argv.push_front("-c".as_bytes().to_vec());
                    for addi_arg in b_addi_arg {
                        vdq_argv.push_front(addi_arg);
                    }
                    vdq_argv.push_front(b_app_path.clone());
                    app_path = String::from_utf8(b_app_path).map_err(|_| "Invalid utf-8 sequence")?;
                    break;
                },
            }
        }
        let argv = Vec::from(vdq_argv);
        do_exec(app_path, argv, envp)
    } else {
        info!("exec!");
        for (idx, a) in argv.iter().enumerate() {
            info!("argv [{}]: {}", idx, core::str::from_utf8(a).unwrap());
        }
        for (idx, a) in envp.iter().enumerate() {
            verbose!("envp [{}]: {}", idx, core::str::from_utf8(a).unwrap());
        }
        Ok(current_process().unwrap().exec(arr, app_path, argv, envp))
    }
}

/// Wait for a pid to end, then return it's exit status.
pub fn sys_waitpid(pid: isize, exit_code_ptr: VirtAddr, options: isize) -> isize {
    info!("Waitpid {} called by {}!", pid, current_process().unwrap().pid.0);
    loop {
        let proc = current_process().unwrap();
        let mut locked_inner = proc.get_inner_locked();
        let mut corpse: Option<usize> = None;
        for (idx, child) in locked_inner.children.iter().enumerate() {
            if pid == -1 || pid as usize == child.get_pid() {
                if child.get_inner_locked().status == ProcessStatus::Zombie {
                    corpse = Some(idx);
                }
            }
        }
        if let Some(idx) = corpse {
            let child_proc = locked_inner.children.remove(idx);
            let child_arcpcb = child_proc.get_inner_locked();
            assert_eq!(Arc::strong_count(&child_proc), 1, "This child process seems to be referenced more then once.");
            if exit_code_ptr.0 != 0 {
                locked_inner.layout.write_user_data(exit_code_ptr, &((child_arcpcb.exit_code as i32) << 8));
            }
            debug!("Zombie {} was killed, exit status = {}", child_proc.get_pid(), child_arcpcb.exit_code);
            debug!("Waitpid returned! (caller {}, dead child {})", current_process().unwrap().pid.0, child_proc.pid.0);
            return child_proc.get_pid() as isize;
        }
        // WNOHANG @ bit 0
        if options.get_bit(0) {
            debug!("Nohang waitpid, instant return. options={}", options);
            return 0;
        } else {
            drop(locked_inner);
            suspend_switch();
            // crate::trap::trap_return();
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

pub fn sys_brk(sz: usize) -> isize {
    if sz == 0 {
        return current_process().unwrap().get_inner_locked().size as isize;
    }
    let proc = current_process().unwrap();
    let mut locked_inner = proc.get_inner_locked();
    let original_size = locked_inner.size;
    if locked_inner.layout.alter_segment(VirtAddr::from(original_size).to_vpn_ceil(), VirtAddr::from(sz).to_vpn_ceil()).is_some() {
        locked_inner.size = sz as usize;
        sz as isize
    } else {
        fatal!("sbrk failed! OOM!");
        -1
    }
}

pub fn sys_mmap(mut start: VirtAddr, len: usize, prot: usize, _: usize, fd: usize, offset: usize) -> isize {
    let proc = current_process().unwrap();
    let mut locked_inner = proc.get_inner_locked();
    if fd == usize::MAX {
        // if start.0 == 0 {
            match locked_inner.layout.get_continuous_space(len) {
                Some(start_vpn) => {
                    start = start_vpn.into();
                    verbose!("Found space @ {:?}, len {}", start, len);
                },
                None => {
                    fatal!("No virtual space left!");
                    locked_inner.recv_signal(crate::process::default_handlers::SIGSEGV);
                    return -1;
                }
            }
        // }

        let mut flags = SegmentFlags::empty();
        if prot & PROT_NONE == 0 {
            flags |= SegmentFlags::U;
        }
        if prot & PROT_READ != 0 {
            flags |= SegmentFlags::R;
        }
        if prot & PROT_WRITE != 0 {
            flags |= SegmentFlags::W;
        }
        if prot & PROT_EXEC != 0 {
            flags |= SegmentFlags::X;
        }
        locked_inner.layout.add_segment(Arc::new(Mutex::new(
            Segment::new(
                start, 
                start + len, 
                crate::memory::MapType::Framed, 
                flags, 
                VMAFlags::empty(), 
                None, 
                0
            )
        )));
        return start.0 as isize;
    } else if let Some(file) = locked_inner.files[fd].clone() {
        if let Ok(addr) = locked_inner.layout.add_vma(file, start, VMAFlags::from_bits((prot << 1) as u8).unwrap(), offset, len) {
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

pub fn sys_kill(target_pid: isize, signal: usize) -> isize {
    if target_pid == 0 {
        let parent = current_process().unwrap();
        let parent_inner = parent.get_inner_locked();
        let mut all_fail = true;
        for child in &parent_inner.children {
            if child.recv_signal(signal).is_some() {
                all_fail = false;
            }
        }
        if all_fail {
            -1
        } else {
            0
        }
    } else if target_pid == -1 {
        let pm_inner = PROCESS_MANAGER.lock();
        let mut all_fail = true;
        for proc in &pm_inner.processes {
            // hard code: init process never dies.
            if proc.pid.0 != 0 {
                if proc.recv_signal(signal).is_some() {
                    all_fail = false;
                }
            }
        }
        if all_fail {
            -1
        } else {
            0
        }
    } else if target_pid < 0 {
        // process group not implemented
        -1
    } else if let Some(proc) = get_proc_by_pid(target_pid as usize) {
        match proc.recv_signal(signal) {
            Some(_) => 0,
            None => -1
        }
    } else {
        error!("No such process with pid {}, failed to send signal", target_pid);
        -1
    }
}

pub fn sys_tgkill(target_tgid: isize, target_tid: isize, signal: usize) -> isize {
    if let Some(proc) = get_proc_by_pid(target_tid as usize) {
        if proc.tgid as isize == target_tgid {
            match proc.recv_signal(signal) {
                Some(_) => 0,
                None => -1
            }
        } else {
        error!("no such proc");
        -1
        }
    } else {
        error!("no such proc");
        -1
    }
}

// TODO: consider edge cases of act is nullptr
// TODO: reference to https://elixir.bootlin.com/linux/latest/source/kernel/signal.c#L4015 (do_sigaction), implement reporting unsupport
pub fn sys_sigaction(signum: usize, act_ptr: VirtAddr, old_act_ptr: VirtAddr) -> isize {
    let proc = current_process().unwrap();
    let mut locked_inner = proc.get_inner_locked();

    if act_ptr.0 != 0 {
        let new_act: SigAction = locked_inner.layout.read_user_data(act_ptr);
        let old_act_op = locked_inner.handlers.insert(signum, new_act);
    
        if old_act_ptr.0 != 0 {
            if let Some(mut old_act) = old_act_op {
                old_act.mask = locked_inner.sig_mask;
                locked_inner.layout.write_user_data(old_act_ptr, &old_act);
            } else {
                return -1;
            }
        }
        return 0;
    } else {
        let old_act_op = locked_inner.handlers.get_mut(&signum);
        if old_act_ptr.0 != 0 {
            if let Some(old_act_orig) = old_act_op {
                let mut old_act: SigAction = old_act_orig.clone();
                old_act.mask = locked_inner.sig_mask;
                locked_inner.layout.write_user_data(old_act_ptr, &old_act);
            } else {
                return -1;
            }
        }
        return 0;
    }
}

pub const SIG_BLOCK     : isize = 0;
pub const SIG_UNBLOCK   : isize = 1;
pub const SIG_SETMASK   : isize = 2;

pub fn sys_sigprocmask(how: isize, oldmask: VirtAddr, newmask: VirtAddr) -> isize {
    let proc = current_process().unwrap();
    let mut locked_inner = proc.get_inner_locked();
    if oldmask.0 != 0 {
        locked_inner.layout.write_user_data(oldmask, &locked_inner.sig_mask);
    }

    let new_mask: u64 = if newmask.0 == 0 {
        0
    } else {
        locked_inner.layout.read_user_data(newmask)
    };

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
    let mut locked_inner = proc.get_inner_locked();
    if let Some(last_signal) = locked_inner.last_signal {
        // reg2 (x2) is sp
        let old_trap_context: TrapContext = locked_inner.signal_trap_contexts.pop().unwrap();
        info!("triggered sigreturn, pc going to: {:x}", old_trap_context.sepc);
        locked_inner.write_trap_context(&old_trap_context);
        locked_inner.sig_mask &= !(1u64 << last_signal);
        locked_inner.last_signal = None;
        0
    } else {
        -1
    }
}

pub fn sys_mprotect(addr: VirtAddr, len: usize, prot: usize) -> isize {
    let proc = current_process().unwrap();
    let mut locked_inner = proc.get_inner_locked();
    let mut flags = PTEFlags::empty();
    if prot != PROT_NONE {
        flags |= PTEFlags::U;
    }
    if prot & PROT_READ != 0 {
        flags |= PTEFlags::R;
    }
    if prot & PROT_WRITE != 0 {
        flags |= PTEFlags::W;
    }
    if prot & PROT_EXEC != 0 {
        flags |= PTEFlags::X;
    }
    let grow_up = prot & PROT_GROWSUP != 0;
    let grow_down = prot & PROT_GROWSDOWN != 0;
    // locked_inner.layout.print_layout();
    verbose!("m_protect flag: {:?}", flags);
    match locked_inner.layout.modify_access(addr.into(), len, flags, grow_up, grow_down) {
        Some(_) => {
            // locked_inner.layout.print_layout();
            0
        },
        None => {
            locked_inner.recv_signal(crate::process::default_handlers::SIGSEGV);
            -1
        }
    }
}

pub fn sys_exit_group(exit_status: i32) -> ! {
    let proc = current_process().unwrap();
    let mut pids: Vec<usize> = Vec::new();
    for process in  PROCESS_MANAGER.lock().processes.iter() {
        if process.tgid == proc.tgid {
            pids.push(process.pid.0);
        }
    }
    for pid in pids {
        let group_process = remove_proc_by_pid(pid).unwrap();
        let mut group_inner = group_process.get_inner_locked();
        debug!("Application {} exited with code {:}", group_process.pid.0, exit_status);
        // mark as dead
        group_inner.status = ProcessStatus::Zombie;
        group_inner.exit_code = exit_status;
        
        // adopt children
        let mut initproc_inner = PROC0.get_inner_locked();
        for child in group_inner.children.iter() {
            child.get_inner_locked().parent = Some(Arc::downgrade(&PROC0));
            initproc_inner.children.push(child.clone());
        }
        
        group_inner.children.clear();
        group_inner.layout.drop_all();
        group_inner.utime = group_inner.utime + get_time() - group_inner.last_start;
    }
    debug!("Application {} exited with code {:}", proc.pid.0, exit_status);
    drop(proc);
    exit_switch(exit_status);
    unreachable!("This part should be unreachable. Go check __switch.");
}

pub fn sys_gettid() -> isize {
    return current_process().unwrap().pid.0 as isize;
}

#[repr(C)]
#[derive(Copy, Clone)]
struct timeval {
    tv_sec: i64,
    tv_usec: i64,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct itimerval {
    it_interval: timeval,
    it_value: timeval,
}

const ITIMER_REAL: i32 = 0;
const ITIMER_VIRTUAL: i32 = 1;
const ITIMER_PROF: i32 = 2;

pub fn sys_getitimer(which: i32, old: VirtAddr) -> isize {
    let process = current_process().unwrap();
    let mut lock = process.get_inner_locked();
    match which {
        ITIMER_REAL => {
            let now = get_time() * 1000 / CLOCK_FREQ;
            let val = if now <= lock.timer_real_next {
                lock.timer_real_int as i64
            } else {
                (now - lock.timer_real_next) as i64
            };
            if old.0 != 0 {
                let tmp = itimerval {
                    it_interval: timeval {
                        tv_sec: (lock.timer_real_int / 1000) as i64,
                        tv_usec: (lock.timer_real_int % 1000) as i64,
                    },
                    it_value: timeval {
                        tv_sec: val / 1000,
                        tv_usec: val % 1000,
                    },
                };
                lock.layout.write_user_data(old, &tmp);
            }
        },
        ITIMER_VIRTUAL => {
            let now = lock.utime * 1000 / CLOCK_FREQ;
            let val = if now <= lock.timer_virt_next {
                lock.timer_virt_int as i64
            } else {
                (now - lock.timer_virt_next) as i64
            };
            if old.0 != 0 {
                let tmp = itimerval {
                    it_interval: timeval {
                        tv_sec: (lock.timer_virt_int / 1000) as i64,
                        tv_usec: (lock.timer_virt_int % 1000) as i64,
                    },
                    it_value: timeval {
                        tv_sec: val / 1000,
                        tv_usec: val % 1000,
                    },
                };
                lock.layout.write_user_data(old, &tmp);
            }
        },
        ITIMER_PROF => {
            let now = lock.timer_prof_now * 1000 / CLOCK_FREQ;
            let val = if now <= lock.timer_prof_next {
                lock.timer_prof_int as i64
            } else {
                (now - lock.timer_prof_next) as i64
            };
            if old.0 != 0 {
                let tmp = itimerval {
                    it_interval: timeval {
                        tv_sec: (lock.timer_prof_int / 1000) as i64,
                        tv_usec: (lock.timer_prof_int % 1000) as i64,
                    },
                    it_value: timeval {
                        tv_sec: val / 1000,
                        tv_usec: val % 1000,
                    },
                };
                lock.layout.write_user_data(old, &tmp);
            }
        },
        _ => {
            error!("sys_getitimer: invalid which");
            return -1;
        }
    }
    return 0;
}

pub fn sys_setitimer(which: i32, new: VirtAddr, old: VirtAddr) -> isize {
    info!("sys_setitimer: {} {:#18X} {:#18X}", which, new.0, old.0);
    if old.0 != 0 {
        if sys_getitimer(which, old) == -1 {
            return -1;
        }
    }
    let process = current_process().unwrap();
    info!("sys_setitimer: pid {}", process.pid.0);
    let mut lock = process.get_inner_locked();
    let new: itimerval = lock.layout.read_user_data(new);
    info!("sys_setitimer: {} {} {} {} {}", which, new.it_interval.tv_sec, new.it_interval.tv_usec, new.it_value.tv_sec, new.it_value.tv_usec);
    if new.it_interval.tv_sec < 0 || new.it_interval.tv_usec < 0 || new.it_interval.tv_usec > 999999 {
        error!("sys_setitimer: invalid new value");
        return -1;
    }
    if new.it_value.tv_sec < 0 || new.it_value.tv_usec < 0 || new.it_value.tv_usec > 999999 {
        error!("sys_setitimer: invalid new value");
        return -1;
    }
    match which {
        ITIMER_REAL => {
            let now = get_time();
            lock.timer_real_int = (new.it_interval.tv_sec * 1000000 + new.it_interval.tv_usec) as u64;
            lock.timer_real_next = (new.it_value.tv_sec * 1000000 + new.it_value.tv_usec) as u64 * (CLOCK_FREQ / 100000) / 10 + now;
            info!("timer_real_int = {}", lock.timer_real_int);
            info!("timer_real_next = {}", lock.timer_real_next);
            info!("timer_real_now = {}", now);
        },
        ITIMER_VIRTUAL => {
            lock.timer_virt_int = (new.it_interval.tv_sec * 1000000 + new.it_interval.tv_usec) as u64;
            lock.timer_virt_next = (new.it_value.tv_sec * 1000000 + new.it_value.tv_usec) as u64 * (CLOCK_FREQ / 100000) / 10;
        },
        ITIMER_PROF => {
            lock.timer_prof_int = (new.it_interval.tv_sec * 1000000 + new.it_interval.tv_usec) as u64;
            lock.timer_prof_next = (new.it_value.tv_sec * 1000000 + new.it_value.tv_usec) as u64 * (CLOCK_FREQ / 100000) / 10;
        },
        _ => {
            error!("sys_setitimer: invalid which");
            return -1;
        }
    }
    return 0;
}