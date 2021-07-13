//! SBI function wrappers.
#![allow(unused)]

const SBI_SET_TIMER: usize = 0;
const SBI_CONSOLE_PUTCHAR: usize = 1;
const SBI_CONSOLE_GETCHAR: usize = 2;
const SBI_CLEAR_IPI: usize = 3;
const SBI_SEND_IPI: usize = 4;
const SBI_REMOTE_FENCE_I: usize = 5;
const SBI_REMOTE_SFENCE_VMA: usize = 6;
const SBI_REMOTE_SFENCE_VMA_ASID: usize = 7;
const SBI_SHUTDOWN: usize = 8;

use core::convert::TryInto;

/// Make a sbi call.
/// # Description 
/// Select a sbi call, and execute with three arguments.
/// # Examples
/// ```
/// let res: usize = res = sbi_call(SBI_CONSOLE_GETCHAR, 0, 0, 0);
/// ```
#[inline(always)]
pub fn sbi_call(which: usize, mut arg0: usize, arg1: usize, arg2: usize) -> usize{
    unsafe {
        asm!(
            "ecall",
            inout("a0") arg0,
            in("a1") arg1,
            in("a2") arg2,
            in("a7") which
        )
    }
    arg0
}

/// Make a complete sbi call, with eid and fid.
/// # Description 
/// Select a sbi call, and execute with three arguments.
/// # Returns
/// Two results, packed together.
/// # Examples
/// ```
/// let (res1, res2) = sbi_call_all(0x10, 0x4, 0, 0, 0);
/// ```
#[inline(always)]
pub fn sbi_call_all(eid: i32, fid: i32, mut arg0: usize, mut arg1: usize, arg2: usize) -> (usize, usize){
    unsafe {
        asm!(
            "ecall",
            inout("a0") arg0,
            inout("a1") arg1,
            in("a2") arg2,
            in("a6") fid,
            in("a7") eid
        )
    }
    (arg0, arg1)
}

/// Set timer interrupt
pub fn set_timer(timer: u64) {
    sbi_call(SBI_SET_TIMER, timer as usize, 0, 0);
}

/// Put a single byte to SBI I/O module
pub fn put_byte(ch: u8) {
    sbi_call(SBI_CONSOLE_PUTCHAR, ch as usize, 0, 0);
}

/// Get a single byte from SBI I/O module
pub fn get_byte() -> u8 {
    let mut res;
    loop {
        res = sbi_call(SBI_CONSOLE_GETCHAR, 0, 0, 0);
        if(res != 0xFFFFFFFFFFFFFFFF) { break; }
    }
    return res.try_into().unwrap();
}

/// Get a single byte from SBI I/O module
pub fn get_byte_non_block_with_echo() -> usize {
    let res = sbi_call(SBI_CONSOLE_GETCHAR, 0, 0, 0);
    if(res != 0xFFFFFFFFFFFFFFFF) { 
        put_byte(res.try_into().unwrap())
    }
    return res;
}

/// Shutdown the machine
pub fn shutdown() -> ! {
    sbi_call(SBI_SHUTDOWN, 0, 0, 0);
    unreachable!()
}

/// get the vendor id
pub fn get_vendor_id() -> i32 {
    sbi_call_all(0x10, 0x4, 0, 0, 0).1 as i32
}