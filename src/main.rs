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

use lazy_static::*;
use core::cell::RefCell;
pub struct test_struct {
    inner: RefCell<test_struct_inner>,
}
struct test_struct_inner {
    val: usize
}

unsafe impl Sync for test_struct {}

lazy_static! {
    pub static ref test_var : test_struct = test_struct{
        inner: RefCell::new(
            test_struct_inner{
                val: 0,
            }
        ),
    };
}

#[no_mangle]
pub extern "C" fn rust_main() -> !{
    for _i in 0..50000000 {unsafe{asm!("nop");}}
    info!("test value = {:?}", test_var.inner.borrow().val);
    test_var.inner.borrow_mut().val = 1;
    info!("test value = {:?}", test_var.inner.borrow().val);

    info!("Kernel hello world!");
    memory::init();
    trap::init();
    // process::load_apps();
    process::run_first_app();
    panic!("drop off from bottom!");
}
