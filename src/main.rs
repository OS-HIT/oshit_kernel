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

use fs::file::FILE;

#[no_mangle]
pub extern "C" fn rust_main() -> !{
    info!("Kernel hello world!");
    info!("Vendor id = {}", sbi::get_vendor_id());
    memory::init();
    trap::init();
    // drivers::sdcard_test();
    let mut file = FILE::open_file("/test.txt", FILE::FMOD_READ).unwrap();
    let mut buf = [0u8; 512];
    let len = file.read_file(&mut buf).unwrap();
    let buf = &buf[0..len as usize];
    println!("{}", core::str::from_utf8(buf).unwrap());
    if let Err((_, msg)) = file.close_file() {
        error!("{}", msg);
    }

    let mut file = FILE::open_file("/test.txt", FILE::FMOD_WRITE).unwrap();
    let mut buf = [0u8; 512];
    match file.read_file(&mut buf) {
        Ok(len) => {
            error!("我们太弱小了，没有力量（哭腔");
            debug!("len: {}", len);
        },
        Err(msg) => {
            info!("{}", msg);
        }
    };

    let buf = "Goodbye".as_bytes();
    assert!(file.write_file(buf).unwrap() == buf.len() as u32);
    if let Err((_, msg)) = file.close_file() {
        error!("{}", msg);
    }
    
    // process::run_first_app();
    process::init();
    panic!("drop off from bottom!");
}
