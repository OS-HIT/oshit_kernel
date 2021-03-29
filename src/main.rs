#![no_std]
#![no_main]
#![feature(global_asm)]
#![feature(asm)]
#![feature(llvm_asm)]
#![feature(assoc_char_funcs)]
#![feature(panic_info_message)]
#![feature(const_in_array_repeat_expressions)]

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.asm"));

#[macro_use]
mod sbi;
mod panic;
mod fs;
mod syscall;
mod trap;
mod process;
mod memory;
pub mod config;

#[cfg(not(any(feature="board_qemu", feature="board_k210")))]
compile_error!("At least one of the board_* feature should be active!");

#[no_mangle]
pub extern "C" fn rust_main() -> !{
    info!("Kernel hello world!");
    trap::init();
    process::load_apps();
    process::run_first_app();
    panic!("drop off from bottom!");
}
