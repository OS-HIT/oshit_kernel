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
pub const SYSCALL_CLOSE         : usize = 57;
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
pub const SYSCALL_CLONE         : usize = 220;
pub const SYSCALL_EXECVE        : usize = 221;
pub const SYSCALL_MMAP          : usize = 222;
pub const SYSCALL_WAIT4         : usize = 260;

mod fs_syscall;
mod process_syscall;
mod trivial_syscall;

pub use fs_syscall::{sys_write};
pub use process_syscall::{sys_exit, sys_yield};
pub use trivial_syscall::{sys_time, sys_uname};

use crate::memory::VirtAddr;

pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    match syscall_id {
        SYSCALL_WRITE       => sys_write(args[0], VirtAddr(args[1]), args[2]),
        SYSCALL_EXIT        => sys_exit(args[0] as i32),
        SYSCALL_SCHED_YIELD => sys_yield(),
        SYSCALL_TIMES       => sys_time(VirtAddr(args[0])),
        SYSCALL_UNAME       => sys_uname(VirtAddr(args[0])),

        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}