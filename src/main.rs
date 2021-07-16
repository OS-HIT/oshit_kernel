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
    memory::init();
    
    trap::init();

    

    process::init();
    panic!("drop off from bottom!");
}
