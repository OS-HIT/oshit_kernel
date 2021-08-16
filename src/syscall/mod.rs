//! Syscall wrappers.
#![allow(dead_code)]

use crate::{memory::VirtAddr, process::CloneFlags};
use crate::utils::print_kernel_stack;

pub const SYSCALL_GETCWD            : usize = 17;
pub const SYSCALL_DUP               : usize = 23;
pub const SYSCALL_DUP3              : usize = 24;
pub const SYSCALL_IOCTL             : usize = 29;
pub const SYSCALL_MKDIRAT           : usize = 34;
pub const SYSCALL_UNLINKAT          : usize = 35;
pub const SYSCALL_LINKAT            : usize = 37;
pub const SYSCALL_UMOUNT2           : usize = 39;
pub const SYSCALL_MOUNT             : usize = 40;
pub const SYSCALL_CHDIR             : usize = 49;
pub const SYSCALL_OPENAT            : usize = 56;
pub const SYSCALL_OPEN              : usize = 56;
pub const SYSCALL_CLOSE             : usize = 57;
pub const SYSCALL_PIPE              : usize = 59;
pub const SYSCALL_PIPE2             : usize = 59;
pub const SYSCALL_GETDENTS64        : usize = 61;
pub const SYSCALL_READ              : usize = 63;
pub const SYSCALL_WRITE             : usize = 64;
pub const SYSCALL_READV             : usize = 65;
pub const SYSCALL_WRITEV            : usize = 66;
pub const SYSCALL_SENDFILE          : usize = 71;
pub const SYSCALL_PPOLL             : usize = 73;
pub const SYSCALL_READLINKAT        : usize = 78;
pub const SYSCALL_FSTATAT           : usize = 79;
pub const SYSCALL_FSTAT             : usize = 80;
pub const SYSCALL_EXIT              : usize = 93;
pub const SYSCALL_EXIT_GROUP        : usize = 94;
pub const SYSCALL_SET_TID_ADDRESS   : usize = 96;
pub const SYSCALL_NANOSLEEP         : usize = 101;
pub const SYSCALL_SCHED_YIELD       : usize = 124;
pub const SYSCALL_KILL              : usize = 129;
pub const SYSCALL_TGKILL            : usize = 131;
pub const SYSCALL_SIGACTION         : usize = 134;
pub const SYSCALL_SIGPROCMASK       : usize = 135;
pub const SYSCALL_SIGRETURN         : usize = 139;
pub const SYSCALL_TIMES             : usize = 153;
pub const SYSCALL_UNAME             : usize = 160;
pub const SYSCALL_GETTIMEOFDAY      : usize = 169;
pub const SYSCALL_GETPID            : usize = 172;
pub const SYSCALL_GETPPID           : usize = 173;
pub const SYSCALL_GETUID            : usize = 174;
pub const SYSCALL_GETEUID           : usize = 175;
pub const SYSCALL_GETGID            : usize = 176;
pub const SYSCALL_GETEGID           : usize = 177;
pub const SYSCALL_GETTID            : usize = 178;
pub const SYSCALL_SYSINFO           : usize = 179;
pub const SYSCALL_BRK               : usize = 214;
pub const SYSCALL_MUNMAP            : usize = 215;
pub const SYSCALL_CLONE             : usize = 220;  // is this sys_fork?
pub const SYSCALL_EXECVE            : usize = 221;  // is this sys_exec?
pub const SYSCALL_MMAP              : usize = 222;
pub const SYSCALL_MPROTECT          : usize = 226;
pub const SYSCALL_WAIT4             : usize = 260;  // is this sys_waitpid?
pub const SYSCALL_WAITPID           : usize = 260;

mod fs_syscall;
mod process_syscall;
mod trivial_syscall;

pub use fs_syscall::{
    sys_write, 
    sys_read,
    sys_writev,
    sys_readv,
    sys_openat,
    sys_close,
    sys_pipe,
    sys_dup,
    sys_dup3,
    sys_getdents64,
    sys_unlink,
    sys_fstatat,
    sys_fstatat_new,
    sys_fstat, 
    sys_readlinkat,
    sys_mkdirat,
    sys_ioctl,
    sys_sendfile,
    sys_ppoll,
};
pub use process_syscall::{
    sys_exit, 
    sys_exit_group,
    sys_yield,
    sys_fork,
    sys_clone,
    sys_exec,
    sys_waitpid,
    sys_getpid,
    sys_getppid,
    sys_getcwd,
    sys_chdir,
    sys_brk,
    sys_mmap,
    sys_munmap,
    sys_sigreturn,
    sys_sigaction,
    sys_sigprocmask,
    sys_kill,
    sys_mprotect,
    sys_gettid,
    sys_tgkill
};
pub use trivial_syscall::{
    sys_time, 
    sys_uname,
    sys_gettimeofday,
    sys_nanosleep,
    sys_info,
    sys_getuid,
    sys_geteuid,
    sys_getgid,
    sys_getegid,
};

use process_syscall::sys_set_tid_address;

macro_rules! CALL_SYSCALL {
    ( $syscall_name: expr ) => {
        {
            debug!("/========== SYSCALL {} CALLED BY {} ==========\\", stringify!($syscall_name), $crate::process::current_process().unwrap().pid.0);
            let ret = $syscall_name();
            debug!("\\= SYSCALL {} CALLED BY {} RESULT {:<10} =/", stringify!($syscall_name), $crate::process::current_process().unwrap().pid.0, ret);
            print_kernel_stack();
            ret
        }
    };
    ( $syscall_name: expr, $($y:expr),+ ) => {
        {
            debug!("/========== SYSCALL {} CALLED BY {} ==========\\", stringify!($syscall_name), $crate::process::current_process().unwrap().pid.0);
            $(
                verbose!("{:>25} = {:?}", stringify!{$y}, $y);
            )+
            let ret: isize = $syscall_name($($y),+);
            debug!("\\= SYSCALL {} CALLED BY {} RESULT {:<10} =/", stringify!($syscall_name), $crate::process::current_process().unwrap().pid.0, ret);
            print_kernel_stack();
            ret
        }
    };
}

/// Handle and dispatch the syscalls to corresponding module.
pub fn syscall(syscall_id: usize, args: [usize; 6]) -> isize {
    // verbose!("syscall {} received!", syscall_id);
    let res = match syscall_id {
        SYSCALL_READ            => {CALL_SYSCALL!(sys_read, args[0], VirtAddr::from(args[1]), args[2])},
        SYSCALL_WRITE           => {CALL_SYSCALL!(sys_write, args[0], VirtAddr::from(args[1]), args[2])},
        // exit is unreachable
        // SYSCALL_EXIT            => {CALL_SYSCALL!(sys_exit, args[0] as i32)},
        SYSCALL_EXIT            => sys_exit(args[0] as i32),
        SYSCALL_EXIT_GROUP      => sys_exit_group(args[0] as i32),

        SYSCALL_SCHED_YIELD     => {CALL_SYSCALL!(sys_yield)},
        SYSCALL_CLONE           => {CALL_SYSCALL!(sys_clone, CloneFlags::from_bits_truncate(args[0]), args[1], VirtAddr::from(args[2]), args[3], VirtAddr::from(args[4]))},
        SYSCALL_EXECVE          => {CALL_SYSCALL!(sys_exec, VirtAddr::from(args[0]), VirtAddr::from(args[1]), VirtAddr::from(args[2]))},
        SYSCALL_WAITPID         => {CALL_SYSCALL!(sys_waitpid, args[0] as isize, VirtAddr::from(args[1]), args[2] as isize)},
        SYSCALL_GETPID          => {CALL_SYSCALL!(sys_getpid)},
        SYSCALL_GETPPID         => {CALL_SYSCALL!(sys_getppid)},
        SYSCALL_GETCWD          => {CALL_SYSCALL!(sys_getcwd, VirtAddr::from(args[0]), args[1])},
        SYSCALL_TIMES           => {CALL_SYSCALL!(sys_time, VirtAddr::from(args[0]))},
        SYSCALL_GETTIMEOFDAY    => {CALL_SYSCALL!(sys_gettimeofday, VirtAddr::from(args[0]))},
        SYSCALL_UNAME           => {CALL_SYSCALL!(sys_uname, VirtAddr::from(args[0]))},
        SYSCALL_PIPE            => {CALL_SYSCALL!(sys_pipe, VirtAddr::from(args[0]))},
        SYSCALL_DUP             => {CALL_SYSCALL!(sys_dup, args[0])},
        SYSCALL_DUP3            => {CALL_SYSCALL!(sys_dup3, args[0], args[1], args[2])},
        SYSCALL_OPENAT          => {CALL_SYSCALL!(sys_openat, args[0] as i32, VirtAddr::from(args[1]), args[2] as u32, args[3] as u32)},
        SYSCALL_CLOSE           => {CALL_SYSCALL!(sys_close, args[0])},
        SYSCALL_CHDIR           => {CALL_SYSCALL!(sys_chdir, VirtAddr::from(args[0]))},
        SYSCALL_GETDENTS64      => {CALL_SYSCALL!(sys_getdents64, args[0], VirtAddr::from(args[1]), args[2])},
        SYSCALL_NANOSLEEP       => {CALL_SYSCALL!(sys_nanosleep, VirtAddr::from(args[0]), VirtAddr::from(args[1]))},
        SYSCALL_BRK             => {CALL_SYSCALL!(sys_brk, args[0])},
        SYSCALL_MMAP            => {CALL_SYSCALL!(sys_mmap, VirtAddr::from(args[0]), args[1], args[2], args[3], args[4], args[5])},
        SYSCALL_UNLINKAT        => {CALL_SYSCALL!(sys_unlink, args[0] as i32, VirtAddr::from(args[1]), args[2])},
        SYSCALL_MKDIRAT         => {CALL_SYSCALL!(sys_mkdirat, args[0], VirtAddr::from(args[1]), args[2])},
        SYSCALL_READLINKAT      => {CALL_SYSCALL!(sys_readlinkat, args[0], VirtAddr::from(args[1]), VirtAddr::from(args[2]), args[3])},
        SYSCALL_FSTATAT         => {CALL_SYSCALL!(sys_fstatat_new, args[0] as i32, VirtAddr::from(args[1]), VirtAddr::from(args[2]), args[3])},
        SYSCALL_FSTAT           => {CALL_SYSCALL!(sys_fstat, args[0], VirtAddr::from(args[1]))},
        SYSCALL_MUNMAP          => {CALL_SYSCALL!(sys_munmap, VirtAddr::from(args[0]), args[1])},
        SYSCALL_READV           => {CALL_SYSCALL!(sys_readv, args[0], VirtAddr::from(args[1]), args[2])},
        SYSCALL_WRITEV          => {CALL_SYSCALL!(sys_writev, args[0], VirtAddr::from(args[1]), args[2])},
        SYSCALL_SYSINFO         => {CALL_SYSCALL!(sys_info, VirtAddr::from(args[0]))},
        SYSCALL_SET_TID_ADDRESS => {CALL_SYSCALL!(sys_set_tid_address, VirtAddr::from(args[0]))},
        SYSCALL_GETUID          => {CALL_SYSCALL!(sys_getuid)},
        SYSCALL_GETEUID         => {CALL_SYSCALL!(sys_geteuid)},
        SYSCALL_GETGID          => {CALL_SYSCALL!(sys_getgid)},
        SYSCALL_GETEGID         => {CALL_SYSCALL!(sys_getegid)},
        SYSCALL_SIGRETURN       => {CALL_SYSCALL!(sys_sigreturn)},
        SYSCALL_SIGACTION       => {CALL_SYSCALL!(sys_sigaction, args[0], VirtAddr::from(args[1]), VirtAddr::from(args[2]))},
        SYSCALL_SIGPROCMASK     => {CALL_SYSCALL!(sys_sigprocmask, args[0] as isize, VirtAddr::from(args[1]), VirtAddr::from(args[2]))},
        SYSCALL_KILL            => {CALL_SYSCALL!(sys_kill, args[0] as isize, args[1])},
        SYSCALL_MPROTECT        => {CALL_SYSCALL!(sys_mprotect, VirtAddr::from(args[0]), args[1], args[2])},
        SYSCALL_GETTID          => {CALL_SYSCALL!(sys_gettid)}
        SYSCALL_IOCTL           => {CALL_SYSCALL!(sys_ioctl, args[0], args[1] as u64, VirtAddr::from(args[2]))},
        SYSCALL_SENDFILE        => {CALL_SYSCALL!(sys_sendfile, args[0], args[1], VirtAddr::from(args[2]), args[3])}
        SYSCALL_PPOLL           => {CALL_SYSCALL!(sys_ppoll)},
        SYSCALL_TGKILL          => {CALL_SYSCALL!(sys_tgkill, args[0] as isize, args[1] as isize, args[2])}
        _ => {
            CALL_SYSCALL!(sys_unknown, syscall_id, args[0], args[1], args[2], args[3], args[4], args[5])
        },
    };

    res
}

pub fn sys_unknown(syscall_id: usize, _: usize, _: usize, _: usize, _: usize, _: usize, _: usize) -> isize {
    fatal!("Unsupported syscall_id: {}", syscall_id);
    -1
}