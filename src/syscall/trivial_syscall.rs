//! Trivial system calls.
use crate::{process::{current_process, suspend_switch}, sbi::get_time};
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
        tms.tms_cstime += get_time() - child_proc.get_inner_locked().up_since;
        tms.tms_cutime += child_proc.get_inner_locked().utime;
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