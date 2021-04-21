use crate::sbi::get_time;
use crate::process::{get_current_up_since, get_current_utime, get_current_satp};
use crate::memory::{VirtAddr, translate_user_va};
use crate::config::*;
use crate::utils::strcpy;
use core::convert::TryInto;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct TMS {
    tms_utime: u64,
    tms_stime: u64,
    tms_cutime: u64,
    tms_cstime: u64,
}

pub fn sys_time(tms_va: VirtAddr) -> isize {
    let tms_ptr: *mut TMS = translate_user_va(get_current_satp(), tms_va);
    unsafe {
        if let Some(tms) = tms_ptr.as_mut() {
            tms.tms_stime = (get_time() - get_current_up_since()) as u64;
            tms.tms_utime = get_current_utime() as u64;
            tms.tms_cstime = 0; // TODO: add these after we implemented fork
            tms.tms_cutime = 0;
        }
    }
    return get_time().try_into().unwrap();
}


#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct UTSName {
    sysname     : usize,
    nodename    : usize,
    release     : usize,
    version     : usize,
    machine     : usize,
    domainname  : usize,
}

pub fn sys_uname(uts_va: VirtAddr) -> isize {
    let uts_ptr: *mut UTSName = translate_user_va(get_current_satp(), uts_va);
    if let Some(uts) = unsafe{ uts_ptr.as_mut() } {
        strcpy(SYSNAME.as_ptr(),    translate_user_va(get_current_satp(), VirtAddr(uts.sysname   )));
        strcpy(NODENAME.as_ptr(),   translate_user_va(get_current_satp(), VirtAddr(uts.nodename  )));
        strcpy(RELEASE.as_ptr(),    translate_user_va(get_current_satp(), VirtAddr(uts.release   )));
        strcpy(VERSION.as_ptr(),    translate_user_va(get_current_satp(), VirtAddr(uts.version   )));
        strcpy(MACHINE.as_ptr(),    translate_user_va(get_current_satp(), VirtAddr(uts.machine   )));
        strcpy(DOMAINNAME.as_ptr(), translate_user_va(get_current_satp(), VirtAddr(uts.domainname)));
        return 0;
    } else {
        return -1;
    }
}