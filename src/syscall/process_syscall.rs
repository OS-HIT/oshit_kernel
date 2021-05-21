use crate::process::{current_path, current_process, enqueue, exit_switch, suspend_switch};

use crate::memory::{
    VirtAddr,
    get_user_cstr,
    translate_user_va
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


pub fn sys_chdir(buf: VirtAddr) -> isize {
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