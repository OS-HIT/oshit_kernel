//! Syscall wrappers.

#![allow(dead_code)]
pub const SYSCALL_GETCWD        : usize = 17;
pub const SYSCALL_DUP           : usize = 23;
pub const SYSCALL_DUP3          : usize = 24;
pub const SYSCALL_MKDIRAT       : usize = 34;
pub const SYSCALL_UNLINKAT      : usize = 35;
pub const SYSCALL_LINKAT        : usize = 37;
pub const SYSCALL_UMOUNT2       : usize = 39;
pub const SYSCALL_MOUNT         : usize = 40;
pub const SYSCALL_CHDIR         : usize = 49;
pub const SYSCALL_OPENAT        : usize = 56;
pub const SYSCALL_OPEN          : usize = 56;
pub const SYSCALL_CLOSE         : usize = 57;
pub const SYSCALL_PIPE          : usize = 59;
pub const SYSCALL_PIPE2         : usize = 59;
pub const SYSCALL_GETDENTS64    : usize = 61;
pub const SYSCALL_READ          : usize = 63;
pub const SYSCALL_WRITE         : usize = 64;
pub const SYSCALL_FSTAT         : usize = 80;
pub const SYSCALL_EXIT          : usize = 93;
pub const SYSCALL_NANOSLEEP     : usize = 101;
pub const SYSCALL_SCHED_YIELD   : usize = 124;
pub const SYSCALL_TIMES         : usize = 153;
pub const SYSCALL_UNAME         : usize = 160;
pub const SYSCALL_GETTIMEOFDAY  : usize = 169;
pub const SYSCALL_GETPID        : usize = 172;
pub const SYSCALL_GETPPID       : usize = 173;
pub const SYSCALL_BRK           : usize = 214;
pub const SYSCALL_MUNMAP        : usize = 215;
pub const SYSCALL_CLONE         : usize = 220;  // is this sys_fork?
pub const SYSCALL_FORK          : usize = 220;
pub const SYSCALL_EXECVE        : usize = 221;  // is this sys_exec?
pub const SYSCALL_EXEC          : usize = 221;
pub const SYSCALL_MMAP          : usize = 222;
pub const SYSCALL_WAIT4         : usize = 260;  // is this sys_waitpid?
pub const SYSCALL_WAITPID       : usize = 260;

mod fs_syscall;
mod process_syscall;
mod trivial_syscall;

use core::convert::TryInto;

pub use fs_syscall::{
    sys_write, 
    sys_read,
    sys_openat,
    sys_close,
    sys_pipe,
    sys_dup,
    sys_dup3,
    sys_getdents64,
};
pub use process_syscall::{
    sys_exit, 
    sys_yield,
    sys_fork,
    sys_exec,
    sys_waitpid,
    sys_getpid,
    sys_getppid,
    sys_getcwd,
    sys_chdir,
    sys_sbrk,
};
pub use trivial_syscall::{
    sys_time, 
    sys_uname,
    sys_gettimeofday,
    sys_nanosleep,
};

use crate::memory::VirtAddr;

/// Handle and dispatch the syscalls to corresponding module.
pub fn syscall(syscall_id: usize, args: [usize; 6]) -> isize {
    match syscall_id {
        SYSCALL_READ        => sys_read(args[0], args[1].into(), args[2]),
        SYSCALL_WRITE       => sys_write(args[0], VirtAddr(args[1]), args[2]),
        SYSCALL_EXIT        => sys_exit(args[0] as i32),
        SYSCALL_SCHED_YIELD => sys_yield(),
        SYSCALL_FORK        => sys_fork(),
        SYSCALL_EXEC        => sys_exec(args[0].into(), args[1].into(), args[2].into()),
        SYSCALL_WAITPID     => sys_waitpid(args[0] as isize, args[1].into(), args[2] as isize),
        SYSCALL_GETPID      => sys_getpid(),
        SYSCALL_GETPPID     => sys_getppid(),
        SYSCALL_GETCWD      => sys_getcwd(args[0].into(), args[1]),
        SYSCALL_TIMES       => sys_time(VirtAddr(args[0])),
        SYSCALL_GETTIMEOFDAY=> sys_gettimeofday(args[0].into()),
        SYSCALL_UNAME       => sys_uname(VirtAddr(args[0])),
        SYSCALL_PIPE        => sys_pipe(VirtAddr(args[0])),
        SYSCALL_DUP         => sys_dup(args[0]),
        SYSCALL_DUP3        => sys_dup3(args[0], args[1], args[2]),
        SYSCALL_OPENAT      => sys_openat(args[0] as i32, args[1].into(), args[2] as u32, args[3] as u32),
        SYSCALL_CLOSE       => sys_close(args[0]),
        SYSCALL_CHDIR       => sys_chdir(args[0].into()),
        SYSCALL_GETDENTS64  => sys_getdents64(args[0], args[1].into(), args[2]),
        SYSCALL_NANOSLEEP   => sys_nanosleep(args[0].into(), args[1].into()),
        SYSCALL_BRK         => sys_sbrk(args[0]),
        _ => {
            fatal!("Unsupported syscall_id: {}", syscall_id);
            -1
        },
    }
}