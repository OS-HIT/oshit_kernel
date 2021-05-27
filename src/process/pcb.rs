use crate::fs::{
    FILE,
    FTYPE,
    File,
    STDIN,
    STDOUT,
    STDERR,
};

use crate::memory::{
    MemLayout,
    PhysAddr,
    PhysPageNum,
    VirtAddr,
    Segment,
    MapType,
    SegmentFlags,
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
    kernel_stack_pos
};
use spin::{
    Mutex,
    MutexGuard
};
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use alloc::string::{String, ToString};

// saved on top of the kernel stack of corresponding process.
#[repr(C)]
pub struct ProcessContext {
    ra  : usize,
    s   : [usize; 12],
}

impl ProcessContext {
    // constructor (?)
    // load in __restore as ra
    pub fn init() -> Self {
        extern "C" { fn __restore(); }
        return Self {
            ra  : trap_return as usize,
            s   : [0; 12],
        };
    }
}

#[allow(dead_code)]
#[derive(Copy, Clone, PartialEq)]
pub enum ProcessStatus {
    New,
    Ready,
    Running,
    Zombie
}

pub struct ProcessControlBlock {
    pub pid:            Pid,
    pub kernel_stack:   KernelStack,
    inner:              Mutex<ProcessControlBlockInner>,
}

pub struct ProcessControlBlockInner {
    pub context_ptr: usize,
    pub status: ProcessStatus,
    pub layout: MemLayout,
    pub trap_context_ppn: PhysPageNum,
    pub size: usize,
    pub up_since: u64,
    pub last_start: u64,
    pub utime: u64,
    pub parent: Option<Weak<ProcessControlBlock>>,
    pub children: Vec<Arc<ProcessControlBlock>>,
    pub files: Vec<Option<Arc<dyn File + Send+ Sync>>>,
    pub path: String,
    pub exit_code: i32,
}

impl ProcessControlBlockInner {
    pub fn get_trap_context(&self) -> &'static mut TrapContext {
        unsafe {
            (PhysAddr::from(self.trap_context_ppn.clone()).0 as *mut TrapContext).as_mut().unwrap()
        }
    }

    pub fn get_satp(&self) -> usize {
        return self.layout.get_satp();
    }
    
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
}

impl ProcessControlBlock {
    pub fn new(elf_data: &[u8], path: String) -> Self {
        let (layout, data_top, mut user_stack_top, entry) = MemLayout::new_elf(elf_data);
        let trap_context_ppn = layout.translate(VirtAddr::from(TRAP_CONTEXT).into()).unwrap().ppn();
        let pid = alloc_pid();
        let kernel_stack = KernelStack::new(&pid);
        let kernel_stack_top = kernel_stack.top();
        let context_ptr = kernel_stack.save_to_top(ProcessContext::init()) as usize;
        let status = ProcessStatus::Ready;

        let z: usize = 0;
        user_stack_top -= core::mem::size_of::<usize>();
        layout.write_user_data(user_stack_top.into(), &z);

        let pcb = Self {
            pid,
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
                    Some(Arc::new(crate::fs::Stdin)), 
                    Some(Arc::new(crate::fs::Stdout)), 
                    Some(Arc::new(crate::fs::Stderr))
                ],
                path: path[..path.rfind('/').unwrap() + 1].to_string(),
                exit_code: 0
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

    pub fn fork(self: &Arc<ProcessControlBlock>) -> Arc<ProcessControlBlock> {
        let mut parent_arcpcb = self.get_inner_locked();
        let layout = MemLayout::fork_from_user(&parent_arcpcb.layout);
        let trap_context_ppn = layout.translate(VirtAddr(TRAP_CONTEXT).into()).unwrap().ppn();
        let pid = alloc_pid();
        let kernel_stack = KernelStack::new(&pid);
        let kernel_stack_top = kernel_stack.top();
        let context_ptr = kernel_stack.save_to_top(ProcessContext::init()) as usize;
        let status = ProcessStatus::Ready;
        
        let pcb = Arc::new(ProcessControlBlock {
            pid,
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
                exit_code: 0
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
    //     ┌-------- |         envp[n]        | 8 bytes
    //     |         |------------------------|
    //     |         |          ....          |
    //     |         |------------------------|
    //     | ┌------ |         envp[0]        | 8 bytes    <= *const envp, envp_base
    //     | |       |------------------------|
    //     | |       |            0           | 8 bytes
    //     | |       |------------------------|
    //     | | ┌---- |         argv[n]        | 8 bytes
    //     | | |     |------------------------|
    //     | | |     |          ....          |  
    //     | | |     |------------------------|
    //     | | | ┌-- |         argv[0]        | 8 bytes    <= *const argv
    //     | | | |   |------------------------| <- strs_base, argv_base
    //     | | | |   |               b'/0'    |
    //     | | | |   |    str     ------------|
    //     | | | |   |    of         ....     |
    //     | | | |   |  argv[0]   ------------|
    //     | | | └-> |             argv[0][0] |
    //     | | |     |------------------------|
    //     | | |     |          ...           |
    //     | | |     |------------------------|
    //     | | |     |               b'/0'    |
    //     | | |     |    str     ------------|
    //     | | |     |    of         ....     |
    //     | | |     |  argv[n]   ------------|
    //     | | └---> |             argv[n][0] |
    //     | |       |------------------------|
    //     | |       |               b'/0'    |
    //     | |       |    str     ------------|
    //     | |       |    of         ....     |
    //     | |       |  envp[0]   ------------|
    //     | └-----> |             envp[0][0] |
    //     |         |------------------------|
    //     |         |          ...           |
    //     |         |------------------------|
    //     |         |               b'/0'    |
    //     |         |    str     ------------|
    //     |         |    of         ....     |
    //     |         |  envp[n]   ------------|
    //     └-------> |             envp[n][0] |
    //               |------------------------|
    //               |          align         |
    //               |------------------------| <- new user_stack_top
    //               |========== LO ==========|
    pub fn exec(&self, elf_data: &[u8], path: String, argv: Vec<Vec<u8>>, envp: Vec<Vec<u8>>) -> isize {
        let (layout, data_top, mut user_stack_top, entry) = MemLayout::new_elf(elf_data);
        let trap_context_ppn = layout.translate(VirtAddr::from(TRAP_CONTEXT).into()).unwrap().ppn();

        // user_stack_top -= (argv.len() + 1) * core::mem::size_of::<usize>();
        let envp_base = user_stack_top - (envp.len() + 1) * core::mem::size_of::<usize>();
        let argv_base = envp_base - (argv.len() + 1) * core::mem::size_of::<usize>();
        let strs_base = argv_base;
        let mut iter: VirtAddr = strs_base.into();

        let mut argv_ptrs: Vec<VirtAddr> = Vec::new();
        for arg in argv {
            iter -= arg.len();
            argv_ptrs.push(iter);
            let mut arg_buf = layout.get_user_buffer(iter.into(), arg.len());
            arg_buf.write_bytes(&arg, 0);
            verbose!("Arg: {}, len: {}", core::str::from_utf8(&arg).unwrap(), arg.len());
        }
        argv_ptrs.push(0.into());

        let mut envp_ptrs: Vec<VirtAddr> = Vec::new();
        for env in envp {
            iter -= env.len();
            argv_ptrs.push(iter);
            let mut arg_buf = layout.get_user_buffer(iter.into(), env.len());
            arg_buf.write_bytes(&env, 0);
            verbose!("Env: {}, len: {}", core::str::from_utf8(&env).unwrap(), env.len());
        }
        envp_ptrs.push(0.into());

        let mut envp_buf = layout.get_user_buffer(envp_base.into(), envp_ptrs.len() * core::mem::size_of::<usize>());
        let mut offset = 0;
        for ptr in envp_ptrs {
            envp_buf.write(offset, &ptr.0);
            offset += core::mem::size_of::<usize>();
        }

        let mut argv_buf = layout.get_user_buffer(argv_base.into(), argv_ptrs.len() * core::mem::size_of::<usize>());
        let mut offset = 0;
        for ptr in argv_ptrs.iter() {
            argv_buf.write(offset, &ptr.0);
            offset += core::mem::size_of::<usize>();
        }

        user_stack_top = iter.0 - iter.0 % core::mem::size_of::<usize>();

        let mut arcpcb = self.get_inner_locked();
        arcpcb.layout = layout;     // original layout dropped, thus freed.
        arcpcb.trap_context_ppn = trap_context_ppn;
        arcpcb.size = data_top;
        arcpcb.utime = 0;
        arcpcb.up_since = get_time();
        arcpcb.path = path[..path.rfind('/').unwrap() + 1].to_string();
        let mut trap_context = TrapContext::init(
            entry, 
            user_stack_top, 
            KERNEL_MEM_LAYOUT.lock().get_satp(), 
            self.kernel_stack.top().0, 
            user_trap as usize
        );
        trap_context.regs[10] = argv_ptrs.len() - 1;
        trap_context.regs[11] = argv_base;
        trap_context.regs[12] = envp_base;
        verbose!("fork argc: {}", trap_context.regs[10]);
        *arcpcb.get_trap_context() = trap_context;
        return (argv_ptrs.len() - 1) as isize;
    }

    pub fn get_inner_locked(&self) -> MutexGuard<ProcessControlBlockInner> {
        return self.inner.lock();
    }

    pub fn get_trap_context(&self) -> &'static mut TrapContext {
        PhysAddr::from(self.get_inner_locked().trap_context_ppn).get_mut()
    }

    pub fn get_pid(&self) -> usize {
        self.pid.0
    }

    pub fn get_ppid(&self) -> usize {
        let arcpcb = self.get_inner_locked();
        arcpcb.parent.as_ref().unwrap().upgrade().unwrap().get_pid()
    }

    pub fn alloc_fd(&mut self) -> usize {
        let mut arcpcb = self.get_inner_locked();
        arcpcb.alloc_fd()
    }
}
