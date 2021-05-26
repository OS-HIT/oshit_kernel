use crate::{process::current_process, sbi::get_time};
use crate::process::{current_up_since, current_utime, current_satp};
use crate::memory::{VirtAddr, translate_user_va};
use crate::config::*;
use crate::utils::strcpy;
use crate::version::*;
use core::{borrow::Borrow, convert::TryInto};

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct TMS {
    tms_utime: u64,
    tms_stime: u64,
    tms_cutime: u64,
    tms_cstime: u64,
}

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

pub fn sys_uname(uts_va: VirtAddr) -> isize {
    // let uts_ptr: *mut UTSName = translate_user_va(current_satp(), uts_va);
    // if let Some(uts) = unsafe{ uts_ptr.as_mut() } {

    //     strcpy(SYSNAME.as_ptr(),    translate_user_va(current_satp(), VirtAddr(uts.sysname   )));
    //     strcpy(NODENAME.as_ptr(),   translate_user_va(current_satp(), VirtAddr(uts.nodename  )));
    //     strcpy(RELEASE.as_ptr(),    translate_user_va(current_satp(), VirtAddr(uts.release   )));
    //     strcpy(VERSION.as_ptr(),    translate_user_va(current_satp(), VirtAddr(uts.version   )));
    //     strcpy(MACHINE.as_ptr(),    translate_user_va(current_satp(), VirtAddr(uts.machine   )));
    //     strcpy(DOMAINNAME.as_ptr(), translate_user_va(current_satp(), VirtAddr(uts.domainname)));
    //     return 0;
    // } else {
    //     return -1;
    // }

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