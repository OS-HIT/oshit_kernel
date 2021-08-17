//! Trivial system calls.
use crate::{process::{ProcessStatus, current_process, suspend_switch}, sbi::{TICKS_PER_SECOND, get_time}};
use crate::memory::{VirtAddr};
use crate::config::*;
use crate::version::*;
use core::{convert::TryInto};

/// Linux style tms
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct TMS {
    tms_utime: u64,
    tms_stime: u64,
    tms_cutime: u64,
    tms_cstime: u64,
}

/// Return execution time of current process and it's children
pub fn sys_time(tms_va: VirtAddr) -> isize {
    let process = current_process().unwrap();
    let arcpcb = process.get_inner_locked();

    let mut tms = TMS {
        tms_stime  : (get_time() - arcpcb.up_since) as u64,
        tms_utime  : arcpcb.utime,
        tms_cstime : 0,
        tms_cutime : 0,
    };
    for child_proc in arcpcb.children.iter() {
        if child_proc.get_inner_locked().status == ProcessStatus::Zombie {
            tms.tms_cstime += get_time() - child_proc.get_inner_locked().up_since;
            tms.tms_cutime += child_proc.get_inner_locked().utime;
        }
    }

    arcpcb.layout.write_user_data(tms_va, &tms);

    return get_time().try_into().unwrap();
}

/// Linux Style timespec
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct TimeSPEC {
    pub tvsec: u64,
    pub tvnsec: u32,
}

/// Since we don't have RTC, we return seconds and nanoseconds since boot.
pub fn sys_gettimeofday(ts: VirtAddr) -> isize {
    let time = TimeSPEC {
        tvsec: crate::sbi::get_time_ms()/1000,
        tvnsec: (crate::sbi::get_time() * (1000000000 / CLOCK_FREQ) % 1000000000) as u32 ,
    };
    current_process().unwrap().get_inner_locked().layout.write_user_data(ts, &time);
    0
}


/// Linux style uts_name
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct UTSName {
    sysname     : [u8; UTSNAME_LEN],
    nodename    : [u8; UTSNAME_LEN],
    release     : [u8; UTSNAME_LEN],
    version     : [u8; UTSNAME_LEN],
    machine     : [u8; UTSNAME_LEN],
    domainname  : [u8; UTSNAME_LEN],
}

/// Rsturn system informations.
pub fn sys_uname(uts_va: VirtAddr) -> isize {
    let mut uts: UTSName = UTSName {
        sysname    : [0u8; UTSNAME_LEN] ,
        nodename   : [0u8; UTSNAME_LEN] ,
        release    : [0u8; UTSNAME_LEN] ,
        version    : [0u8; UTSNAME_LEN] ,
        machine    : [0u8; UTSNAME_LEN] ,
        domainname : [0u8; UTSNAME_LEN] ,
    };
    uts.sysname   [0..SYSNAME   .len()].clone_from_slice(SYSNAME      );
    uts.nodename  [0..NODENAME  .len()].clone_from_slice(NODENAME     );
    uts.release   [0..RELEASE   .len()].clone_from_slice(RELEASE      );
    uts.version   [0..VERSION   .len()].clone_from_slice(VERSION      );
    uts.machine   [0..MACHINE   .len()].clone_from_slice(MACHINE      );
    uts.domainname[0..DOMAINNAME.len()].clone_from_slice(DOMAINNAME   );

    current_process().unwrap().get_inner_locked().layout.write_user_data(uts_va, &uts);
    0
}

/// Sleep for a specified time.
pub fn sys_nanosleep(req: VirtAddr, _: VirtAddr) -> isize{
    let req: TimeSPEC = current_process().unwrap().get_inner_locked().layout.read_user_data(req);
    while get_time() / CLOCK_FREQ < req.tvsec {
        suspend_switch();
    }
    while (get_time() * (1000000000 / CLOCK_FREQ)) % 1000000000 < req.tvnsec as u64 {
        suspend_switch();
    }

    0
}

pub fn sys_info(sysinfo: VirtAddr) -> isize {
    // TODO
    return 0;
}

pub fn sys_getuid() -> isize {
    return 0;
}
pub fn sys_geteuid() -> isize {
    return 0;
}
pub fn sys_getgid() -> isize {
    return 0;
}
pub fn sys_getegid() -> isize {
    return 0;
}


const RUSAGE_SELF     : i32 = 0;
const RUSAGE_CHILDREN : i32 = -1;
const RUSAGE_BOTH     : i32 = -2;
const RUSAGE_THREAD   : i32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct OldTimeVal {
    pub tvsec: u32,
    pub tvnsec: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RUSage {
    pub utime: OldTimeVal,
    pub stime: OldTimeVal,
    pub maxrss: u32,        /* maximum resident set size */
    pub ixrss: u32,         /* integral shared memory size */
    pub idrss: u32,         /* integral unshared data size */
    pub isrss: u32,         /* integral unshared stack size */
    pub minflt: u32,        /* page reclaims (soft page faults) */
    pub majflt: u32,        /* page faults (hard page faults) */
    pub nswap: u32,         /* swaps */
    pub inblock: u32,       /* block input operations */
    pub oublock: u32,       /* block output operations */
    pub msgsnd: u32,        /* IPC messages sent */
    pub msgrcv: u32,        /* IPC messages received */
    pub nsignals: u32,      /* signals received */
    pub nvcsw: u32,         /* voluntary context switches */
    pub nivcsw: u32,        /* involuntary context switches */
}

pub fn sys_getrusage(who: i32, rusage_ptr: VirtAddr) -> isize {
    let process = current_process().unwrap();
    let arcpcb = process.get_inner_locked();

    let rusage = match who {
        RUSAGE_SELF | RUSAGE_CHILDREN | RUSAGE_BOTH => {
            let s_time = get_time() - arcpcb.up_since;
            let u_time = arcpcb.utime;
            
            // for child_proc in arcpcb.children.iter() {
            //     s_time += get_time() - child_proc.get_inner_locked().up_since;
            //     u_time += child_proc.get_inner_locked().utime;
            // }

            RUSage {
                utime: OldTimeVal {
                    tvsec: (s_time / CLOCK_FREQ) as u32,
                    tvnsec: (s_time % CLOCK_FREQ * 1000000) as u32,
                },
                stime: OldTimeVal{
                    tvsec: (u_time / CLOCK_FREQ) as u32,
                    tvnsec: (u_time % CLOCK_FREQ * 1000000) as u32,
                },
                maxrss:     0,
                ixrss:      0,
                idrss:      arcpcb.size as u32,
                isrss:      USER_STACK_SIZE  as u32,
                minflt:     0,
                majflt:     0,
                nswap:      0,
                inblock:    0,
                oublock:    0,
                msgsnd:     0,
                msgrcv:     0,
                nsignals:   0,
                nvcsw:      0,
                nivcsw:     0,
            }
        },
        _ => {
            return -1;
        }
    };

    arcpcb.layout.write_user_data(rusage_ptr, &rusage);

    return 0;
}