//! # OSHIT-Kernel
//! This is OSHIT Kernel, a RISC-V rust based operating system kernel.
#![no_std]
#![no_main]
#![feature(global_asm)]
#![feature(asm)]
#![feature(llvm_asm)]
#![feature(panic_info_message)]
// #![feature(const_in_array_repeat_expressions)]
#![feature(alloc_error_handler)]
#![feature(map_try_insert)]

use alloc::string::ToString;

use crate::{config::{U_TRAMPOLINE, TRAMPOLINE}, process::default_handlers::{def_dump_core, def_ignore, def_terminate_self}};

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.asm"));

#[macro_use]    // so that we get vec![] macro
extern crate alloc;
extern crate xmas_elf;
extern crate k210_pac;
extern crate k210_soc;
extern crate k210_hal;

#[macro_use]
mod sbi;
mod panic;
mod fs;
mod syscall;
mod trap;
mod process;
mod memory;
pub mod config;
pub mod version;
mod utils;
mod drivers;

#[cfg(not(any(feature="board_qemu", feature="board_k210")))]
compile_error!("At least one of the board_* feature should be active!");

/// Main function for boot sequence
/// # Description
/// This is the main function, which is used during the boot sequence.
/// Will be called by `__start()` in entry.asm, after CRT setup.
/// # Examples
/// **DO NOT CALL THIS FUNCTION!**
/// # Returns
/// never returns.
#[no_mangle]
pub extern "C" fn rust_main() -> !{
    print!("{}", config::LOGO);
    info!("Kernel hello world!");
    info!("Vendor id = {}", sbi::get_vendor_id());

    extern "C" {
        fn strampoline();
        fn sutrampoline();
        fn __alltraps();
        fn __restore();
        fn __user_call_sigreturn();
        fn __user_restore_from_handler();
        fn __siginfo();
    }
    debug!("========== mapped funcs ==========");
    debug!("strampoline: {:x}", strampoline as usize);
    debug!("strampoline: {:x}", sutrampoline as usize);
    debug!("__alltraps: {:x}", __alltraps as usize);
    debug!("__restore: {:x}", __restore as usize);
    // debug!("__restore_to_signal_handler: {:x}", __restore_to_signal_handler as usize);
    debug!("trampoline: {:x}", TRAMPOLINE);
    debug!("utrampoline: {:x}", U_TRAMPOLINE);
    debug!("phys strampoline: {:x}", strampoline as usize);
    debug!("phys sutrampoline: {:x}", sutrampoline as usize);
    debug!("__user_call_sigreturn: {:x}", __user_call_sigreturn as usize);
    debug!("__siginfo: {:x}", __siginfo as usize);
    debug!("def_terminate_self: {:x}", def_terminate_self as usize - sutrampoline as usize + U_TRAMPOLINE);
    debug!("def_dump_core: {:x}", def_dump_core as usize - sutrampoline as usize + U_TRAMPOLINE);
    debug!("def_ignore: {:x}", def_ignore as usize - sutrampoline as usize + U_TRAMPOLINE);
    // debug!("trampoline: {:x}", TRAMPOLINE);
    debug!("==================================");

    memory::init();
    trap::init();

    fs::mount_fs("/dev".to_string(), fs::DEV_FS.clone()).unwrap();
    fs::mount_fs("/".to_string(), alloc::sync::Arc::new(fs::fs_impl::Fat32W::new(fs::open("/dev/block/sda".to_string(), fs::OpenMode::SYS).unwrap()).unwrap())).unwrap();
    fs::mount_fs("/proc".to_string(), fs::PROC_FS.clone()).unwrap();

    process::init();
    panic!("drop off from bottom!");
}
