//! Implementation of Process Control Block of oshit kernel

use crate::fs::{File, open};

use crate::memory::{
    MemLayout,
    PhysAddr,
    PhysPageNum,
    VirtAddr,
    KERNEL_MEM_LAYOUT
};
use crate::config::*;
use crate::trap::{
    TrapContext,
    user_trap,
    trap_return
};
use crate::sbi::get_time;
use super::{
    Pid,
    KernelStack,
    alloc_pid,
};
use _core::clone;
use _core::mem::size_of;
use alloc::collections::{BTreeMap, VecDeque};
use spin::{
    Mutex,
    MutexGuard
};
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use crate::process::default_handlers::*;
use crate::fs::OpenMode;

use bitflags::*;
use bit_field::*;


bitflags! {
    pub struct SignalFlags: u32 {
        const NOCLDSTOP   = 0x00000001;
        const NOCLDWAIT   = 0x00000002;
        const SIGINFO     = 0x00000004;
        const ONSTACK     = 0x08000000;
        const RESTART     = 0x10000000;
        const NODEFER     = 0x40000000;
        const UNSUPPORTED = 0x00000400;
        const RESETHAND   = 0x80000000;
    }
}

#[repr(usize)]
#[derive(Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum AuxType {
    NULL            = 0x00 ,       /* end of vector */
    IGNORE          = 0x01 ,       /* entry should be ignored */
    EXECFD          = 0x02 ,       /* file descriptor of program */
    PHDR            = 0x03 ,       /* program headers for program */
    PHENT           = 0x04 ,       /* size of program header entry */
    PHNUM           = 0x05 ,       /* number of program headers */
    PAGESZ          = 0x06 ,       /* system page size */
    BASE            = 0x07 ,       /* base address of interpreter */
    FLAGS           = 0x08 ,       /* flags */
    ENTRY           = 0x09 ,       /* entry point of program */
    NOTELF          = 0x0a,       /* program is not ELF */
    UID             = 0x0b,       /* real uid */
    EUID            = 0x0c,       /* effective uid */
    GID             = 0x0d,       /* real gid */
    EGID            = 0x0e,       /* effective gid */
    PLATFORM        = 0x0f,       /* string identifying CPU for optimizations */
    HWCAP           = 0x10,       /* arch dependent hints at CPU capabilities */
    CLKTCK          = 0x11,       /* frequency at which times() increments */
    /* 18 through 22 are reserved */
    SECURE          = 0x17,       /* secure mode boolean */
    BASE_PLATFORM   = 0x18,       /* string identifying real platform, may differ from AT_PLATFORM. */
    RANDOM          = 0x19,       /* address of 16 random bytes */
    HWCAP2          = 0x1a,       /* extension of AT_HWCAP */
    EXECFN          = 0x1f,       /* filename of program */
    SYSINFO_EHDR    = 0x21,
    NULL28          = 0x28,
    NULL29          = 0x29,
    NULL2a          = 0x2a,
    NULL2b          = 0x2b,
    NULL2c          = 0x2c,
    NULL2d          = 0x2d,
}

#[repr(C)]
pub struct AuxHeader {
    pub aux_type: AuxType,
    pub value   : usize
}


/// The process context used in `__switch` (kernel execution flow) 
/// Saved on top of the kernel stack of corresponding process.
#[repr(C)]
pub struct ProcessContext {
    ra  : usize,
    s   : [usize; 12],
}

impl ProcessContext {
    /// Construct a new ProcessContext.
    /// # Description
    /// Construct a new ProcessContext, and use `__restore` as return address, thus will go to sret and execute user program.
    pub fn init() -> Self {
        extern "C" { fn __restore(); }
        return Self {
            ra  : trap_return as usize,
            s   : [0; 12],
        };
    }
}

/// Representing status of a process
#[derive(Copy, Clone, PartialEq)]
pub enum ProcessStatus {
    /// A new process, and is not initialized.
    New,
    /// A process that is ready to run.
    Ready,
    /// A running process.
    Running,
    /// A dead process, but it's resources are not collected yet.
    Zombie
}

bitflags! {
    pub struct CloneFlags: usize {
        const VM                = 0x00000100;	/* set if VM shared between processes */
        const FS                = 0x00000200;	/* set if fs info shared between processes */
        const FILES             = 0x00000400;	/* set if open files shared between processes */
        const SIGHAND           = 0x00000800;	/* set if signal handlers and blocked signals shared */
        const PIDFD             = 0x00001000;	/* set if a pidfd should be placed in parent */
        const PTRACE            = 0x00002000;	/* set if we want to let tracing continue on the child too */
        const VFORK             = 0x00004000;	/* set if the parent wants the child to wake it up on mm_release */
        const PARENT            = 0x00008000;	/* set if we want to have the same parent as the cloner */
        const THREAD            = 0x00010000;	/* Same thread group? */
        const NEWNS             = 0x00020000;	/* New mount namespace group */
        const SYSVSEM           = 0x00040000;	/* share system V SEM_UNDO semantics */
        const SETTLS            = 0x00080000;	/* create a new TLS for the child */
        const PARENT_SETTID     = 0x00100000;	/* set the TID in the parent */
        const CHILD_CLEARTID    = 0x00200000;	/* clear the TID in the child */
        const DETACHED          = 0x00400000;	/* Unused, ignored */
        const UNTRACED          = 0x00800000;	/* set if the tracing process can't force CLONE_PTRACE on this clone */
        const CHILD_SETTID      = 0x01000000;	/* set the TID in the child */
        const NEWCGROUP         = 0x02000000;	/* New cgroup namespace */
        const NEWUTS            = 0x04000000;	/* New utsname namespace */
        const NEWIPC            = 0x08000000;	/* New ipc namespace */
        const NEWUSER           = 0x10000000;	/* New user namespace */
        const NEWPID            = 0x20000000;	/* New pid namespace */
        const NEWNET            = 0x40000000;	/* New network namespace */
        const IO                = 0x80000000;	/* Clone io context */
    }
}

#[derive(Clone)]
pub struct ImmuInfos {
    pub exec_path: String,
}

/// The process control block
pub struct ProcessControlBlock {
    /// Pid of the process
    pub pid:            Pid,
    /// tgid of the process, aka PID from user view
    pub tgid:           usize,
    /// The kernel stack of the process. PCB holds it so the resource is not dropped.
    pub kernel_stack:   KernelStack,
    pub immu_infos:     ImmuInfos,
    /// The mutable inner, protected by a Mutex
    pub inner:          Mutex<ProcessControlBlockInner>,
}

#[derive(Clone, Copy)]
pub struct SigAction {
    pub sighandler: VirtAddr,
    pub sigaction: VirtAddr,
    pub mask: u64,
    pub flags: SignalFlags,
    pub restorer: VirtAddr // deprecated, go with zero
}

/// The mutable part of the process control block
pub struct ProcessControlBlockInner {
    /// The ProcessContext pointer
    pub context_ptr: usize,
    /// Process status
    pub status: ProcessStatus,
    /// user memory layout
    pub layout: MemLayout,
    /// Physical page number of the trap context
    pub trap_context_ppn: PhysPageNum,
    /// process data segment size
    pub size: usize,
    /// time when the process started running
    pub up_since: u64,
    /// last time the process started executing in u mode
    pub last_start: u64,
    /// total process executed in u mode
    pub utime: u64,
    /// Parent of the process. proc0 has no parent.
    pub parent: Option<Weak<ProcessControlBlock>>,
    /// childres processes.
    pub children: Vec<Arc<ProcessControlBlock>>,
    /// Opened file descriptors
    /// TODO: Change to hash_map<Arc<dyn VirtFile + Send + Sync>>>
    pub files: Vec<Option<Arc<dyn File>>>,
    /// Current working directory
    pub path: String,
    /// Exit code of the process
    pub exit_code: i32,
    /// pending signals
    pub pending_sig: VecDeque<usize>,
    /// signal handlers
    /// FIXME: THE SigAction mask HAS NO USE. USE ONLY THE pcb's sig_mask!!!
    pub handlers: BTreeMap<usize, SigAction>,
    /// signal masks
    pub sig_mask: u64,
    pub signal_trap_contexts: Vec<TrapContext>,
    pub last_signal: Option<usize>,
    pub dead_children_stime: u64,
    pub dead_children_utime: u64,
    pub timer_real_next: u64,
    pub timer_real_int: u64,
    pub timer_real_start: u64,
    pub timer_virt_next: u64,
    pub timer_virt_int: u64,
    pub timer_prof_next: u64,
    pub timer_prof_int: u64,
    pub timer_prof_now: u64,
}

impl ProcessControlBlockInner {

    pub fn print_debug_msg_inner(&self) {
        if let Some(parent_weak) = self.parent.clone() {
            if let Some(parent_proc) = parent_weak.upgrade() {
                println!("Parent pid: {}", parent_proc.pid.0);
            } else {
                println!("Parent dead???");
            }
        } else {
            println!("No Parent.");
        }
        println!("Current Working dir: {}", self.path);
        self.layout.print_layout();
    } 

    /// Read trap context from physical memory
    pub fn get_trap_context(&self) -> &'static mut TrapContext {
        unsafe {
            (PhysAddr::from(self.trap_context_ppn.clone()).0 as *mut TrapContext).as_mut().unwrap()
        }
    }

    pub fn write_trap_context(&self, ctx: &TrapContext) {
        let ptr = PhysAddr::from(self.trap_context_ppn.clone()).0 as *mut TrapContext;
        unsafe {
            *ptr = *ctx;
        }
    }

    /// return SATP of the memory layout
    pub fn get_satp(&self) -> usize {
        return self.layout.get_satp();
    }
    
    /// Alloc a new file descriptor.
    pub fn alloc_fd(&mut self) -> usize {
        let empty_slot = (0..self.files.len()).find(
            |i|
                self.files[*i].is_none()
        );
        match empty_slot {
            Some(fd ) => fd,
            None => {
                self.files.push(None);
                self.files.len() - 1
            }
        }
    }

    pub fn recv_signal(&mut self, signal: usize) -> Option<()> {
        if signal >= 64 {
            None
        } else if self.sig_mask.get_bit(signal) {
            None
        } else {
            self.pending_sig.push_back(signal);
            Some(())
        }
    }
}

pub fn default_sig_handlers() -> BTreeMap<usize, SigAction> {
    extern "C" {fn strampoline(); fn sutrampoline(); }
    let mut map = BTreeMap::new();
    let terminate_self_va   = VirtAddr::from(def_terminate_self as usize - sutrampoline as usize + U_TRAMPOLINE);
    let ignore_va           = VirtAddr::from(def_ignore         as usize - sutrampoline as usize + U_TRAMPOLINE);
    let dump_core_va        = VirtAddr::from(def_dump_core      as usize - sutrampoline as usize + U_TRAMPOLINE);
    let cont_va             = VirtAddr::from(def_cont           as usize - sutrampoline as usize + U_TRAMPOLINE);
    let stop_va             = VirtAddr::from(def_stop           as usize - sutrampoline as usize + U_TRAMPOLINE);
    let terminate_self_va   = SigAction { sighandler: terminate_self_va, sigaction: 0.into(), mask: 0, flags: SignalFlags::empty(), restorer: 0.into()};
    let ignore_va           = SigAction { sighandler: ignore_va        , sigaction: 0.into(), mask: 0, flags: SignalFlags::empty(), restorer: 0.into()};
    let dump_core_va        = SigAction { sighandler: dump_core_va     , sigaction: 0.into(), mask: 0, flags: SignalFlags::empty(), restorer: 0.into()};
    let cont_va             = SigAction { sighandler: cont_va          , sigaction: 0.into(), mask: 0, flags: SignalFlags::empty(), restorer: 0.into()};
    let stop_va             = SigAction { sighandler: stop_va          , sigaction: 0.into(), mask: 0, flags: SignalFlags::empty(), restorer: 0.into()};
    map.insert(SIGHUP   , terminate_self_va.clone());
    map.insert(SIGINT   , terminate_self_va.clone());
    map.insert(SIGQUIT  , terminate_self_va.clone());
    map.insert(SIGILL   , terminate_self_va.clone());
    map.insert(SIGTRAP  , ignore_va        .clone());
    map.insert(SIGABRT  , dump_core_va     .clone());
    map.insert(SIGBUS   , dump_core_va     .clone());
    map.insert(SIGFPE   , dump_core_va     .clone());
    map.insert(SIGKILL  , terminate_self_va.clone());
    map.insert(SIGUSR1  , ignore_va        .clone());
    map.insert(SIGSEGV  , dump_core_va     .clone());
    map.insert(SIGUSR2  , ignore_va        .clone());
    map.insert(SIGPIPE  , terminate_self_va.clone());
    map.insert(SIGALRM  , terminate_self_va.clone());
    map.insert(SIGTERM  , terminate_self_va.clone());
    map.insert(SIGSTKFLT, terminate_self_va.clone());
    map.insert(SIGCHLD  , ignore_va        .clone());
    map.insert(SIGCONT  , cont_va          .clone());
    map.insert(SIGSTOP  , stop_va          .clone());
    map.insert(SIGTSTP  , stop_va          .clone());
    map.insert(SIGTTIN  , stop_va          .clone());
    map.insert(SIGTTOU  , stop_va          .clone());
    map.insert(SIGURG   , ignore_va        .clone());
    map.insert(SIGXCPU  , terminate_self_va.clone());
    map.insert(SIGXFSZ  , terminate_self_va.clone());
    map.insert(SIGVTALRM, ignore_va        .clone());
    map.insert(SIGPROF  , terminate_self_va.clone());
    map.insert(SIGWINCH , ignore_va        .clone());
    map.insert(SIGIO    , ignore_va        .clone());
    map.insert(SIGPWR   , ignore_va        .clone());
    map.insert(SIGSYS   , terminate_self_va.clone());
    for i in SIGRTMIN..SIGRTMAX {
        map.insert(i, ignore_va.clone());
    }
    map
}

impl ProcessControlBlock {
    pub fn print_debug_msg(&self) {
        println!("Exec path: {}", self.immu_infos.exec_path);
        println!("Pid: {}", self.pid.0);
        self.get_inner_locked().print_debug_msg_inner();
    }

    /// Create a new process from ELF.
    /// # Description
    /// Create a new process from ELF. This function will construct the memory layout from elf file, set the entry point,
    /// as well as push the inital fds (stdios).
    /// # Return
    /// Return the new process control block
    pub fn new(elf_data: &[u8], path: String) -> Self {
        let (layout, data_top, mut user_stack_top, entry, _auxv) = MemLayout::new_elf(elf_data);
        let trap_context_ppn = layout.translate(VirtAddr::from(TRAP_CONTEXT).into()).unwrap().ppn();
        let pid = alloc_pid();
        let tgid = pid.0;
        let kernel_stack = KernelStack::new(&pid);
        let kernel_stack_top = kernel_stack.top();
        let context_ptr = kernel_stack.save_to_top(ProcessContext::init()) as usize;
        let status = ProcessStatus::Ready;

        let z: usize = 0;
        user_stack_top -= core::mem::size_of::<usize>();
        layout.write_user_data(user_stack_top.into(), &z);
        let stdin = open("/dev/tty0".to_string(), OpenMode::READ).unwrap();
        let stdout = open("/dev/tty0".to_string(), OpenMode::WRITE).unwrap();
        let stderr = open("/dev/tty0".to_string(), OpenMode::WRITE).unwrap();
        verbose!("stdio fd pre-loaded.");
        let pcb = Self {
            pid,
            tgid,
            immu_infos: ImmuInfos{
                exec_path: path.clone(),
            },
            kernel_stack,
            inner: Mutex::new(ProcessControlBlockInner {
                context_ptr,
                status,
                layout,
                trap_context_ppn,
                size: data_top,
                up_since: get_time(),
                last_start: 0,
                utime: 0,
                parent: None,
                children: Vec::new(),
                files: vec![
                    Some(stdin),
                    Some(stdout),
                    Some(stderr)
                ],
                path: path[..path.rfind('/').unwrap() + 1].to_string(),
                exit_code: 0,
                pending_sig: VecDeque::new(),
                handlers: default_sig_handlers(),
                sig_mask: 0,
                last_signal: None,
                dead_children_stime: 0,
                dead_children_utime: 0,
                timer_real_int: 0,
                timer_real_next: 0,
                timer_real_start: 0,
                timer_virt_int: 0,
                timer_virt_next: 0,
                timer_prof_int: 0,
                timer_prof_next: 0,
                timer_prof_now: 0,
                signal_trap_contexts: Vec::new()
            }),
        };
        let trap_context = pcb.get_inner_locked().get_trap_context();
        *trap_context = TrapContext::init(
            entry, 
            user_stack_top, 
            KERNEL_MEM_LAYOUT.lock().get_satp(), 
            kernel_stack_top.0,
            user_trap as usize
        );
        
        trap_context.regs[10] = 0;
        trap_context.regs[11] = user_stack_top;
        trap_context.regs[12] = user_stack_top;
        return pcb;
    }

    /// Fork a new process from original.
    /// # Description
    /// Fork a process from original process, almost identical except for physical memory mapping.
    /// # Return
    /// Return the new process control block
    pub fn fork(self: &Arc<ProcessControlBlock>, clone_flags: super::CloneFlags) -> Arc<ProcessControlBlock> {
        let mut parent_arcpcb = self.get_inner_locked();
        // let layout = MemLayout::fork_from_user(&parent_arcpcb.layout);
        let layout = MemLayout::clone_from_user(&parent_arcpcb.layout, clone_flags);
        let trap_context_ppn = layout.translate(VirtAddr(TRAP_CONTEXT).into()).unwrap().ppn();
        let pid = alloc_pid();
        let kernel_stack = KernelStack::new(&pid);
        let kernel_stack_top = kernel_stack.top();
        let context_ptr = kernel_stack.save_to_top(ProcessContext::init()) as usize;
        let status = ProcessStatus::Ready;
        let immu_infos = self.immu_infos.clone();
        let tgid = if clone_flags.contains(super::CloneFlags::THREAD) {
            self.pid.0
        } else {
            pid.0
        };
        let pcb = Arc::new(ProcessControlBlock {
            pid,
            tgid,
            immu_infos,
            kernel_stack,
            inner: Mutex::new(ProcessControlBlockInner {
                context_ptr,
                status,
                layout,
                trap_context_ppn,
                size: parent_arcpcb.size,
                up_since: get_time(),
                last_start: 0,
                utime: parent_arcpcb.utime,
                parent: Some(Arc::downgrade(self)),
                children: Vec::new(),
                files: parent_arcpcb.files.clone(),
                path: parent_arcpcb.path.clone(),
                exit_code: 0,
                pending_sig: parent_arcpcb.pending_sig.clone(),
                handlers: parent_arcpcb.handlers.clone(),
                sig_mask: 0,
                last_signal: None,
                dead_children_stime: 0,
                dead_children_utime: 0,
                timer_real_int: parent_arcpcb.timer_real_int,
                timer_real_next: parent_arcpcb.timer_real_next,
                timer_real_start: parent_arcpcb.timer_real_start,
                timer_virt_int: parent_arcpcb.timer_virt_int,
                timer_virt_next: parent_arcpcb.timer_virt_next,
                timer_prof_int: parent_arcpcb.timer_prof_int,
                timer_prof_next: parent_arcpcb.timer_prof_next,
                timer_prof_now: parent_arcpcb.timer_prof_now,
                signal_trap_contexts: Vec::new()
            }),
        });

        parent_arcpcb.children.push(pcb.clone());
        let mut trap_context: &mut TrapContext = PhysAddr::from(pcb.get_inner_locked().trap_context_ppn).get_mut();
        trap_context.kernel_sp = kernel_stack_top.0;
        return pcb;
    }

    //               |========== HI ==========|
    //               |------------------------| <- original user_stack_top
    //               |            0           | 8 bytes
    //               |------------------------|
    //     ???-------- |         envp[n]        | 8 bytes
    //     |         |------------------------|
    //     |         |          ....          |
    //     |         |------------------------|
    //     | ???------ |         envp[0]        | 8 bytes    <= *const envp, envp_base
    //     | |       |------------------------|
    //     | |       |            0           | 8 bytes
    //     | |       |------------------------|
    //     | | ???---- |         argv[n]        | 8 bytes
    //     | | |     |------------------------|
    //     | | |     |          ....          |  
    //     | | |     |------------------------|
    //     | | | ???-- |         argv[0]        | 8 bytes    <= *const argv
    //     | | | |   |------------------------| <- strs_base, argv_base
    //     | | | |   |               b'/0'    |
    //     | | | |   |    str     ------------|
    //     | | | |   |    of         ....     |
    //     | | | |   |  argv[0]   ------------|
    //     | | | ???-> |             argv[0][0] |
    //     | | |     |------------------------|
    //     | | |     |          ...           |
    //     | | |     |------------------------|
    //     | | |     |               b'/0'    |
    //     | | |     |    str     ------------|
    //     | | |     |    of         ....     |
    //     | | |     |  argv[n]   ------------|
    //     | | ???---> |             argv[n][0] |
    //     | |       |------------------------|
    //     | |       |               b'/0'    |
    //     | |       |    str     ------------|
    //     | |       |    of         ....     |
    //     | |       |  envp[0]   ------------|
    //     | ???-----> |             envp[0][0] |
    //     |         |------------------------|
    //     |         |          ...           |
    //     |         |------------------------|
    //     |         |               b'/0'    |
    //     |         |    str     ------------|
    //     |         |    of         ....     |
    //     |         |  envp[n]   ------------|
    //     ???-------> |             envp[n][0] |
    //               |------------------------|
    //               |          align         |
    //               |------------------------| <- new user_stack_top
    //               |========== LO ==========|
    /// Execute certain elf file in current process
    /// # Description
    /// Execute certain elf file in current process. This will reset the whole memory layout and regs.
    /// # Return
    /// Return the argc, for this will subtitude the syscall return value.
    pub fn exec(&self, elf_data: &[u8], path: String, argv: Vec<Vec<u8>>, envp: Vec<Vec<u8>>) -> isize {
        let (layout, data_top, mut user_stack_top, entry, mut auxv) = MemLayout::new_elf(elf_data);
        let trap_context_ppn = layout.translate(VirtAddr::from(TRAP_CONTEXT).into()).unwrap().ppn();

        // // user_stack_top -= (argv.len() + 1) * core::mem::size_of::<usize>();
        // let envp_base = user_stack_top - (envp.len() + 1) * core::mem::size_of::<usize>();
        // let argv_base = envp_base - (argv.len() + 1) * core::mem::size_of::<usize>();
        // let strs_base = argv_base;
        // let mut iter: VirtAddr = strs_base.into();

        // let mut argv_ptrs: Vec<VirtAddr> = Vec::new();
        // for arg in argv {
        //     iter -= arg.len();
        //     argv_ptrs.push(iter);
        //     let mut arg_buf = layout.get_user_buffer(iter.into(), arg.len());
        //     arg_buf.write_bytes(&arg, 0);
        //     verbose!("Arg: {}, len: {}", core::str::from_utf8(&arg).unwrap(), arg.len());
        // }
        // argv_ptrs.push(0.into());

        // let mut envp_ptrs: Vec<VirtAddr> = Vec::new();
        // for env in envp {
        //     iter -= env.len();
        //     argv_ptrs.push(iter);
        //     let mut arg_buf = layout.get_user_buffer(iter.into(), env.len());
        //     arg_buf.write_bytes(&env, 0);
        //     verbose!("Env: {}, len: {}", core::str::from_utf8(&env).unwrap(), env.len());
        // }
        // envp_ptrs.push(0.into());

        // let mut envp_buf = layout.get_user_buffer(envp_base.into(), envp_ptrs.len() * core::mem::size_of::<usize>());
        // let mut offset = 0;
        // for ptr in envp_ptrs {
        //     envp_buf.write(offset, &ptr.0);
        //     offset += core::mem::size_of::<usize>();
        // }

        // let mut argv_buf = layout.get_user_buffer(argv_base.into(), argv_ptrs.len() * core::mem::size_of::<usize>());
        // let mut offset = 0;
        // for ptr in argv_ptrs.iter() {
        //     argv_buf.write(offset, &ptr.0);
        //     offset += core::mem::size_of::<usize>();
        // }

        // user_stack_top = iter.0 - iter.0 % core::mem::size_of::<usize>();

        //  ================================= file name =================================
        let name = &argv[0];
        user_stack_top -= name.len();
        let name_ptr = user_stack_top;
        let mut ptr = user_stack_top;
        for b in name {
            layout.write_user_data(ptr.into(), b);
            ptr += 1;
        }

        //  ================================= envp strs =================================
        let mut envp_ptrs: Vec<usize> = Vec::with_capacity(envp.len() + 1);
        envp_ptrs.resize(envp.len() + 1, 0);
        for (idx, bytes) in envp.iter().enumerate() {
            user_stack_top -= bytes.len();
            envp_ptrs[idx] = user_stack_top;
            let mut ptr = user_stack_top;
            for b in bytes {
                layout.write_user_data(ptr.into(), b);
                ptr += 1;
            }
            // layout.write_user_data(ptr.into(), &(0u8));
        }

        user_stack_top -= user_stack_top % size_of::<usize>();

        //  ================================= argv strs =================================
        let mut argv_ptrs: Vec<usize> = Vec::with_capacity(argv.len() + 1);
        argv_ptrs.resize(argv.len() + 1, 0);
        for (idx, bytes) in argv.iter().enumerate() {
            user_stack_top -= bytes.len();
            argv_ptrs[idx] = user_stack_top;
            let mut ptr = user_stack_top;
            for b in bytes {
                layout.write_user_data(ptr.into(), b);
                ptr += 1;
            }
            // layout.write_user_data(ptr.into(), &(0u8));
        }

        // =================================   align   =================================
        user_stack_top -= user_stack_top % size_of::<usize>();

        //  ================================= platfrom =================================
        user_stack_top -= PLATFROM.len() + 1;
        user_stack_top -= user_stack_top % size_of::<usize>();
        let mut ptr = user_stack_top;
        for b in PLATFROM {
            layout.write_user_data(ptr.into(), b);
            ptr += 1;
        }
        layout.write_user_data(ptr.into(), &0u8);

        //  ================================= rand bytes =================================
        user_stack_top -= 16;
        let mut ptr = user_stack_top;
        for i in 0u8..0xfu8 {
            layout.write_user_data(ptr.into(), &i);
            ptr += 1;
        }
        let random_ptr = user_stack_top;

        // ================================= padding =================================
        let padded_user_stack_top = user_stack_top - (16 + user_stack_top % 16);
        for i in user_stack_top..padded_user_stack_top {
            layout.write_user_data(i.into(), &(0 as u8));
        }
        user_stack_top = padded_user_stack_top;

        // ================================= auxv content =================================
        auxv.push(AuxHeader{aux_type: AuxType::RANDOM,  value: user_stack_top});
        auxv.push(AuxHeader{aux_type: AuxType::EXECFN,  value: name_ptr});
        auxv.push(AuxHeader{aux_type: AuxType::NULL,    value: 0});
        user_stack_top -= auxv.len() * size_of::<AuxHeader>();
        let auxv_base = user_stack_top;
        for (idx, header) in auxv.iter().enumerate() {
            let mut ptr = user_stack_top + size_of::<AuxHeader>() * idx;
            layout.write_user_data(ptr.into(), &(header.aux_type as usize));
            ptr += size_of::<usize>();
            layout.write_user_data(ptr.into(), &header.value);
        }

        // ================================= envp =================================
        user_stack_top -= (envp_ptrs.len()) * size_of::<usize>();
        let envp_base = user_stack_top;
        // layout.write_user_data((user_stack_top + envp.len() * size_of::<usize>()).into(), &0usize);
        // write from high to low
        for (idx, p) in envp_ptrs.iter().enumerate() {
            layout.write_user_data((user_stack_top + idx * size_of::<usize>()).into(), p);
        }

        // ================================= argv =================================
        user_stack_top -= (argv_ptrs.len()) * size_of::<usize>();
        let argv_base = user_stack_top;
        // layout.write_user_data((user_stack_top + argv.len() * size_of::<usize>()).into(), &0usize);
        // write from high to low
        for (idx, p) in argv_ptrs.iter().enumerate() {
            layout.write_user_data((user_stack_top + idx * size_of::<usize>()).into(), p);
        }

        // ================================= argc =================================
        user_stack_top -= size_of::<usize>();
        layout.write_user_data(user_stack_top.into(), &(argv.len()));

        assert!(user_stack_top % size_of::<usize>() == 0, "SP not aligned!");

        verbose!("argv.len(): {:x}", argv.len());
        verbose!("argv_base : {:x}", argv_base );
        verbose!("envp_base : {:x}", envp_base );
        verbose!("auxv_base : {:x}", auxv_base );

        // for i in 0..100 {
        //     verbose!("Stack +  {:2} ({:x}): {:16x}", i, user_stack_top + i*size_of::<usize>(), layout.read_user_data::<usize>((user_stack_top + i*size_of::<usize>()).into()));
        // }

        let mut locked_inner = self.get_inner_locked();
        locked_inner.layout = layout;     // original layout dropped, thus freed.
        locked_inner.trap_context_ppn = trap_context_ppn;
        locked_inner.utime = 0;
        locked_inner.size = data_top;
        locked_inner.utime = 0;
        locked_inner.up_since = get_time();
        locked_inner.path = path[..path.rfind('/').unwrap() + 1].to_string();
        locked_inner.pending_sig = VecDeque::new();
        locked_inner.handlers = default_sig_handlers();
        locked_inner.sig_mask = 0;
        let mut trap_context = TrapContext::init(
            entry, 
            user_stack_top, 
            KERNEL_MEM_LAYOUT.lock().get_satp(), 
            self.kernel_stack.top().0, 
            user_trap as usize
        );  
        // trap_context.regs[10] = argv.len();
        // trap_context.regs[11] = argv_base;
        // trap_context.regs[12] = envp_base;
        // trap_context.regs[13] = auxv_base;

        *locked_inner.get_trap_context() = trap_context;
        return (argv_ptrs.len() - 1) as isize;
    }

    /// Get inner mutable part of the process control block.
    /// # Description
    /// Get inner mutable part of the process control block. The returned variable will held the lock until drop.
    /// # Example
    /// ```
    /// let proc = current_process().unwrap();
    /// let locked_inner = proc.get_inner_locked();
    /// // lock is held by locked_inner
    /// do_something();
    /// drop(locked_inner);
    /// suspend_switch();
    /// ```
    pub fn get_inner_locked(&self) -> MutexGuard<ProcessControlBlockInner> {
        return self.inner.lock();
    }

    /// Get the trap context of current process.
    /// # Return
    /// A mutable reference to the trap context
    pub fn get_trap_context(&self) -> &'static mut TrapContext {
        PhysAddr::from(self.get_inner_locked().trap_context_ppn).get_mut()
    }

    /// get pid of the precess
    pub fn get_pid(&self) -> usize {
        self.tgid
    }

    /// get pid of the precess's parent.
    /// will panic if proc0 calls this.
    pub fn get_ppid(&self) -> usize {
        let locked_inner = self.get_inner_locked();
        locked_inner.parent.as_ref().unwrap().upgrade().unwrap().get_pid()
    }

    /// Alloc a file descriptor
    /// # Description
    /// Alloc a file descriptor. Note that this will require to lock the inner, might cause dead lock if the lock is already held.
    pub fn alloc_fd(&self) -> usize {
        let mut locked_inner = self.get_inner_locked();
        locked_inner.alloc_fd()
    }

    pub fn recv_signal(&self, signal: usize) -> Option<()> {
        info!("process {} received signal {}, pending handle", self.pid.0, signal);
        let mut locked_inner = self.get_inner_locked();
        locked_inner.recv_signal(signal)
    }
}
