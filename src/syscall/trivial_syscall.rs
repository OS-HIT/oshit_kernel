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
    sysname     : *mut u8,
    nodename    : *mut u8,
    release     : *mut u8,
    version     : *mut u8,
    machine     : *mut u8,
    domainname  : *mut u8,
}

pub fn sys_uname(uts_va: VirtAddr) -> isize {
    verbose!("loaded uts_va: 0x{:X}", uts_va.0);
    let uts_ptr: *mut UTSName = translate_user_va(get_current_satp(), uts_va);
    verbose!("translated to: 0x{:X}", uts_ptr as usize);
    unsafe {
        if let Some(uts) = uts_ptr.as_mut() {
            strcpy(SYSNAME.as_ptr(),    uts.sysname);
            strcpy(NODENAME.as_ptr(),   uts.nodename);
            strcpy(RELEASE.as_ptr(),    uts.release);
            strcpy(VERSION.as_ptr(),    uts.version);
            strcpy(MACHINE.as_ptr(),    uts.machine);
            strcpy(DOMAINNAME.as_ptr(), uts.domainname);
            return 0;
        } else {
            return -1;
        }
    }
}