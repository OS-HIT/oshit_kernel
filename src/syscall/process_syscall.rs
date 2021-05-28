//! Process related syscalls.
use crate::process::{current_path, current_process, enqueue, exit_switch, suspend_switch};

use crate::memory::{
    VirtAddr,
    get_user_cstr,
    translate_user_va,
    VMAFlags
};

use crate::process::{
    current_satp,
    ProcessStatus
};

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::convert::TryInto;
use alloc::string::ToString;
use alloc::string::String;

use crate::fs::FILE;

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
    match FILE::open_file(&app_name, FILE::FMOD_READ) {
        Ok(mut file) => {
            verbose!("File found {}", app_name);
            let mut v: Vec<u8> = Vec::with_capacity(file.fsize as usize);
            v.resize(file.fsize as usize, 0);

            match file.read_file(&mut v) {
                Ok(res) => {
                    verbose!("Loaded App {}, size = {}", app_name, res);
                    let proc = current_process().unwrap();
                    let arcpcb = proc.get_inner_locked();

                    verbose!("Loading argv");
                    let mut args: Vec<Vec<u8>> = Vec::new();
                    if argv.0 != 0 {
                        verbose!("argv @ {:0x}", argv.0);
                        let mut iter = argv;
                        loop {
                            let ptr: usize = arcpcb.layout.read_user_data(iter);
                            if ptr == 0 {
                                break;
                            }
                            args.push(arcpcb.layout.get_user_cstr(ptr.into()));
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
                            let ptr: usize = arcpcb.layout.read_user_data(iter);
                            if ptr == 0 {
                                break;
                            }
                            envs.push(arcpcb.layout.get_user_cstr(ptr.into()));
                            iter += core::mem::size_of::<usize>();
                        }
                    }
                    drop(arcpcb);
                    proc.exec(&v, app_name, args, envs)
                },
                Err(msg) => {
                    error!("Failed to read file: {}", msg);
                    1
                }
            }
        } ,
        Err(msg) =>{
            error!("Failed to open file: {}", msg);
            -1
        }
    }
}

/// Wait for a pid to end, then return it's exit status.
pub fn sys_waitpid(pid: isize, exit_code_ptr: VirtAddr, options: isize) -> isize {
    loop {
        let proc = current_process().unwrap();
        let mut arcpcb = proc.get_inner_locked();
        for (idx, child) in arcpcb.children.iter().enumerate() {
            if pid == -1 || pid as usize == child.get_pid() {
                if arcpcb.children[idx].get_inner_locked().status == ProcessStatus::Zombie {
                    let child_proc = arcpcb.children.remove(idx);
                    let child_arcpcb = child_proc.get_inner_locked();
                    assert_eq!(Arc::strong_count(&child_proc), 1, "This child process seems to be referenced more then once.");
                    if exit_code_ptr.0 != 0 {
                        // TODO: properly construct wstatus
                        arcpcb.layout.write_user_data(exit_code_ptr, &((child_arcpcb.exit_code as i32) << 8));
                    }
                    debug!("Zombie {} was killed, exit status = {}", child_proc.get_pid(), child_arcpcb.exit_code);
                    return child_proc.get_pid() as isize;
                }
            }
        }
        if options & WNOHANG != 0 {
            return 0;
        } else {
            drop(arcpcb);
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
    let arcpcb = proc.get_inner_locked();
    let mut buffer = arcpcb.layout.get_user_buffer(buf, size);
    buffer.write_bytes(arcpcb.path.as_bytes(), 0);
    return buf.0 as isize;
}

/// Change the current working directory.
pub fn sys_chdir(buf: VirtAddr) -> isize {
    verbose!("chdir start");
    let proc = current_process().unwrap();
    let mut arcpcb = proc.get_inner_locked();
    if let Ok (dir_str) = core::str::from_utf8(&arcpcb.layout.get_user_cstr(buf)) {
        if let Ok (_) = FILE::open_dir(dir_str, FILE::FMOD_READ) {
            arcpcb.path = dir_str.to_string();
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
    let mut arcpcb = proc.get_inner_locked();
    let original_size = arcpcb.size;
    arcpcb.layout.alter_segment(VirtAddr::from(original_size).to_vpn_ceil(), VirtAddr::from(sz).to_vpn_ceil());
    arcpcb.size = sz as usize;
    return 0;
}

pub fn sys_mmap(start: VirtAddr, len: usize, prot: u8, _: usize, fd: usize, offset: usize) -> isize {
    let proc = current_process().unwrap();
    let mut arcpcb = proc.get_inner_locked();
    if let Some(file) = arcpcb.files[fd].clone() {
        if let Ok(_) = file.clone().to_fs_file_locked() {
            if arcpcb.layout.add_vma(file, start, VMAFlags::from_bits(prot << 1).unwrap(), offset) {
                return 0;
            } 
        }
    }
    -1
}