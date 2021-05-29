//! Trap handler of oshit kernel
use super::TrapContext;
use crate::{process::current_process, syscall::syscall};
use riscv::register::{
    stvec,      // s trap vector base address register
    scause::{   // s cause register
        self,
        Trap,
        Exception,
        Interrupt,
    },
    stval,      // s trap value, exception spcific.
    sie,        // s interrupt enable.
};
use crate::sbi::{
    reset_timer_trigger,
};
use crate::process::{suspend_switch, exit_switch};
use crate::config::*;
use crate::process::{current_trap_context, current_satp};
use crate::memory::VMAFlags;

global_asm!(include_str!("./trap.asm"));

/// enable traps handling unit, by writing stvec and enable the timer interrupt
pub fn init() {
    debug!("Initilizing traps...");
    unsafe {
        extern "C" { fn __alltraps(); }
        verbose!("Enabling interrupts...");
        // write trap handler vector, as well as trap mode
        stvec::write(__alltraps as usize, stvec::TrapMode::Direct);
        // enable timer interrupt
        verbose!("Enabling Supervisor Timer Interrupt...");
        sie::set_stimer();
        reset_timer_trigger();
    }
    info!("Traps initialized.");
}

/// Set trap entry to kernel trap handling function.
fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(kernel_trap as usize, stvec::TrapMode::Direct);
    }
}

/// Set trap entry to user trap handling function.
fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE as usize, stvec::TrapMode::Direct);
    }
}

/// Kernel trap handling function
/// Currently, kernel trap only happen if severe problem has emerged.
#[no_mangle]
pub fn kernel_trap() -> ! {
    fatal!("unhandled trap {:?}.", scause::read().cause());
    panic!("Kernel trap not supported yet!");
}

/// User trap handling function
/// # Description
/// After trampoline has successfully 
/// no mangle so that call user_trap in asm won't break
/// # Return 
/// Do not return, for trap_return calls __restore, then it SRET to user.
#[no_mangle]
pub fn user_trap(_cx: &mut TrapContext) -> ! {
    set_kernel_trap_entry();
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            let mut cx = current_trap_context();
            cx.sepc += 4;   // so that we don't stuck at one instruction
            let result = syscall(cx.regs[17], [
                cx.regs[10], 
                cx.regs[11], 
                cx.regs[12],
                cx.regs[13],
                cx.regs[14],
                cx.regs[15],
            ]) as usize;   // exec syscall in s-mode
            cx =  current_trap_context();
            cx.regs[10] = result as usize;
        },
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            reset_timer_trigger();
            suspend_switch();
        },
        // Store page fault, check vma
        Trap::Exception(Exception::StorePageFault) => {
            verbose!("Store Page Fault!");
            let proc = current_process().unwrap();
            let mut arcpcb = proc.get_inner_locked();
            if let Err(msg) = arcpcb.layout.lazy_copy_vma(stval.into(), VMAFlags::W) {
                error!(
                    "{:?} in application, bad addr = {:#x}, bad instruction = {:#x}, {}",
                    scause.cause(),
                    stval,
                    arcpcb.get_trap_context().sepc,
                    msg
                );
                exit_switch(-2);
            }
        },
        Trap::Exception(Exception::LoadPageFault) => {
            verbose!("Load Page Fault");
            let proc = current_process().unwrap();
            let mut arcpcb = proc.get_inner_locked();
            if let Err(msg) = arcpcb.layout.lazy_copy_vma(stval.into(), VMAFlags::R) {
                error!(
                    "{:?} in application, bad addr = {:#x}, bad instruction = {:#x}, {}",
                    scause.cause(),
                    stval,
                    arcpcb.get_trap_context().sepc,
                    msg
                );
                exit_switch(-2);
            }
        },
        // TODO: Core dump and/or terminate user program and continue
        Trap::Exception(Exception::StoreFault) |
        Trap::Exception(Exception::InstructionFault) |
        Trap::Exception(Exception::InstructionPageFault) |
        Trap::Exception(Exception::LoadFault) => {
            error!(
                "{:?} in application, bad addr = {:#x}, bad instruction = {:#x}, core dumped.",
                scause.cause(),
                stval,
                current_trap_context().sepc,
            );
            exit_switch(-2);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            error!("IllegalInstruction in application, core dumped.");
            exit_switch(-3);
        }
        _ => {
            let cx = current_trap_context();
            error!("Unhandled trap {:?}.", scause.cause());
            error!("Bad addr @ 0x{:#X}, Bad Inst @ 0x{:#X}", stval, cx.sepc);
            exit_switch(-1);
        }
    }
    trap_return();
}

/// Trap return function
/// # Description
/// Trap return funciton. Use jr for trampoline functions.
#[no_mangle]
pub fn trap_return() -> ! {
    set_user_trap_entry();
    let trap_cx_ptr = TRAP_CONTEXT;
    let user_satp = current_satp();
    extern "C" {
        fn __alltraps();
        fn __restore();
    }
    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;
    unsafe {
        llvm_asm!("fence.i" :::: "volatile");
        llvm_asm!("jr $0" :: "r"(restore_va), "{a0}"(trap_cx_ptr), "{a1}"(user_satp) :: "volatile");
    }
    unreachable!("Unreachable in trap_return!");
}