#![no_std]
#![no_main]
#![feature(global_asm)]
#![feature(asm)]
#![feature(llvm_asm)]
#![feature(assoc_char_funcs)]
#![feature(panic_info_message)]
#![feature(const_in_array_repeat_expressions)]
#![feature(alloc_error_handler)]

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.asm"));

#[macro_use]    // so that we get vec![] macro
extern crate alloc;
extern crate xmas_elf;

#[macro_use]
mod sbi;
mod panic;
mod fs;
mod syscall;
mod trap;
mod process;
mod memory;
pub mod config;
mod utils;

#[cfg(not(any(feature="board_qemu", feature="board_k210")))]
compile_error!("At least one of the board_* feature should be active!");

#[no_mangle]
pub extern "C" fn rust_main() -> !{
    info!("Kernel hello world!");
    info!("Vendor id = {}", sbi::get_vendor_id());
    memory::init();
    trap::init();
    process::run_first_app();
    panic!("drop off from bottom!");
}
