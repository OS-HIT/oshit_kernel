//! # OSHIT-Kernel
//! This is OSHIT Kernel, a RISC-V rust based operating system kernel.
#![no_std]
#![no_main]
#![feature(global_asm)]
#![feature(asm)]
#![feature(llvm_asm)]
#![feature(assoc_char_funcs)]
#![feature(panic_info_message)]
#![feature(const_in_array_repeat_expressions)]
#![feature(alloc_error_handler)]

use lazy_static::lazy_static;

use crate::{fs::fat::print_vec, memory::{FrameTracker, alloc_frame}};
use alloc::vec::Vec;

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
    print!("{}", config::logo);
    info!("Kernel hello world!");
    info!("Vendor id = {}", sbi::get_vendor_id());
    memory::init();
    
    trap::init();

    fs::fat::mbr::MBR_INST.print();
    fs::fat::print_dbr();
        
    fs::list_tree("/", 0).unwrap();
    // match fs::FILE::open_file("/clone", fs::FILE::FMOD_READ) {
    //     Ok(mut file) => {
    //         error!("FILE clone chain len {} ({:?}) size {}", file.fchain.len(), file.fchain, file.fsize);
    //     }
    //     Err(msg) => {
    //         error!("Create file failed: {}", msg);
    //     }
    // }
    // fs::list_tree("/", 0).unwrap();
    // fs::fs_test();
    // let mut a = fs::FILE::open_file("/test1", fs::FILE::FMOD_READ | fs::FILE::FMOD_WRITE | fs::FILE::FMOD_CREATE).unwrap();
    // a.write_file(&[1u8; 64]).unwrap();
    // let mut a = fs::FILE::open_file("/test2", fs::FILE::FMOD_READ | fs::FILE::FMOD_WRITE | fs::FILE::FMOD_CREATE).unwrap();
    // a.write_file(&[1u8; 64]).unwrap();
    // let mut a = fs::FILE::open_file("/test3", fs::FILE::FMOD_READ | fs::FILE::FMOD_WRITE | fs::FILE::FMOD_CREATE).unwrap();
    // a.write_file(&[1u8; 64]).unwrap();
    // let mut a = fs::FILE::open_file("/test4", fs::FILE::FMOD_READ | fs::FILE::FMOD_WRITE | fs::FILE::FMOD_CREATE).unwrap();
    // a.write_file(&[1u8; 64]).unwrap();
    // let mut a = fs::FILE::open_file("/test5", fs::FILE::FMOD_READ | fs::FILE::FMOD_WRITE | fs::FILE::FMOD_CREATE).unwrap();
    // a.write_file(&[1u8; 64]).unwrap();
    // let mut a = fs::FILE::open_file("/test6", fs::FILE::FMOD_READ | fs::FILE::FMOD_WRITE | fs::FILE::FMOD_CREATE).unwrap();
    // a.write_file(&[1u8; 64]).unwrap();

    // let a = fs::FILE::open_file("/clone", fs::FILE::FMOD_READ).unwrap();
    // let a = fs::FILE::open_file("/brk", fs::FILE::FMOD_READ).unwrap();
    // let a = fs::FILE::open_file("/dup", fs::FILE::FMOD_READ).unwrap();
    // let a = fs::FILE::open_file("/dup2", fs::FILE::FMOD_READ).unwrap();
    // let a = fs::FILE::open_file("/execve", fs::FILE::FMOD_READ).unwrap();
    // let a = fs::FILE::open_file("/fork", fs::FILE::FMOD_READ).unwrap();
    // let a = fs::FILE::open_file("/unlink", fs::FILE::FMOD_READ).unwrap();

    // let mut a = fs::FILE::open_file("/test1", fs::FILE::FMOD_READ | fs::FILE::FMOD_WRITE | fs::FILE::FMOD_CREATE).unwrap();
    // a.write_file(&[1u8; 64]).unwrap();
    // let mut a = fs::FILE::open_file("/test2", fs::FILE::FMOD_READ | fs::FILE::FMOD_WRITE | fs::FILE::FMOD_CREATE).unwrap();
    // a.write_file(&[1u8; 64]).unwrap();
    // let mut a = fs::FILE::open_file("/test3", fs::FILE::FMOD_READ | fs::FILE::FMOD_WRITE | fs::FILE::FMOD_CREATE).unwrap();
    // a.write_file(&[1u8; 64]).unwrap();
    // let mut a = fs::FILE::open_file("/test4", fs::FILE::FMOD_READ | fs::FILE::FMOD_WRITE | fs::FILE::FMOD_CREATE).unwrap();
    // a.write_file(&[1u8; 64]).unwrap();
    // let mut a = fs::FILE::open_file("/test5", fs::FILE::FMOD_READ | fs::FILE::FMOD_WRITE | fs::FILE::FMOD_CREATE).unwrap();
    // a.write_file(&[1u8; 64]).unwrap();
    // let mut a = fs::FILE::open_file("/test6", fs::FILE::FMOD_READ | fs::FILE::FMOD_WRITE | fs::FILE::FMOD_CREATE).unwrap();
    // a.write_file(&[1u8; 64]).unwrap();

    // let a = fs::FILE::open_file("/clone", fs::FILE::FMOD_READ).unwrap();
    // let a = fs::FILE::open_file("/brk", fs::FILE::FMOD_READ).unwrap();
    // let a = fs::FILE::open_file("/dup", fs::FILE::FMOD_READ).unwrap();
    // let a = fs::FILE::open_file("/dup2", fs::FILE::FMOD_READ).unwrap();
    // let a = fs::FILE::open_file("/execve", fs::FILE::FMOD_READ).unwrap();
    // let a = fs::FILE::open_file("/fork", fs::FILE::FMOD_READ).unwrap();
    // let a = fs::FILE::open_file("/unlink", fs::FILE::FMOD_READ).unwrap();

    process::init();
    panic!("drop off from bottom!");
}
