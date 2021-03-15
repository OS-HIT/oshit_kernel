#![no_std]
#![no_main]
#![feature(global_asm)]
#![feature(asm)]
#![feature(llvm_asm)]
#![feature(assoc_char_funcs)]
#![feature(panic_info_message)]
#![feature(const_in_array_repeat_expressions)]

global_asm!(include_str!("entry.asm"));

#[macro_use]
mod sbi;
mod panic;

#[no_mangle]
pub extern "C" fn rust_main() -> !{
    println!("Hello, world!");
    panic!("drop off from bottom!");
}
