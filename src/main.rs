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

#[no_mangle]
pub extern "C" fn rust_main() -> !{
    print!("{}", config::logo);
    info!("Kernel hello world!");
    info!("Vendor id = {}", sbi::get_vendor_id());
    memory::init();
    trap::init();
        
    fs::list_tree("/", 0).unwrap();
    process::init();
    panic!("drop off from bottom!");
}
