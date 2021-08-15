//! Trap handler of oshit kernel
use super::TrapContext;
use crate::{memory::{VirtAddr}, process::{current_process, default_sig_handlers}, syscall::syscall};
use alloc::sync::Arc;
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
use crate::process::{current_trap_context, current_satp, SignalFlags};
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
                    "{:?} in application {}, bad addr = {:#x}, bad instruction = {:#x}, {}",
                    scause.cause(),
                    proc.pid.0,
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
                if let Some(pte) = arcpcb.layout.pagetable.walk(VirtAddr::from(stval).into()) {
                    error!("Pagetable entry flags: {:?}", pte.flags());
                } else {
                    error!("No such pagetable entry");
                }
                arcpcb.layout.print_layout();
                // arcpcb.recv_signal(crate::process::default_handlers::SIGSEGV);
                exit_switch(-2);
            }
        },
        // TODO: Core dump and/or terminate user program and continue
        Trap::Exception(Exception::StoreFault) |
        Trap::Exception(Exception::InstructionFault) |
        Trap::Exception(Exception::InstructionPageFault) |
        Trap::Exception(Exception::LoadFault) => {
            let proc = current_process().unwrap();
            let arcpcb = proc.get_inner_locked();
            error!(
                "{:?} in application {}, bad addr = {:#x}, bad instruction @ {:#x}",
                scause.cause(),
                proc.pid.0,
                stval,
                current_trap_context().sepc,
            );
            if let Some(pte) = arcpcb.layout.pagetable.walk(VirtAddr::from(stval).into()) {
                error!("Pagetable entry flags: {:?}", pte.flags());
            } else {
                error!("No such pagetable entry");
            }
            exit_switch(-2);
            // current_process().unwrap().recv_signal(crate::process::default_handlers::SIGSEGV);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            error!(
                "{:?} in application {}, bad inst = {:#x} @ {:#x}",
                scause.cause(),
                current_process().unwrap().pid.0,
                stval,
                current_trap_context().sepc,
            );
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

#[allow(unused)]
struct SigInfo {
    si_signo     :i32,	/* Signal number */
    si_errno     :i32,	/* An errno value */
    si_code      :i32,	/* Signal code */
    si_trapno    :i32,	/* Trap number that caused hardware-generated signal (unused on most architectures) */
    si_pid       :u32,	/* Sending process ID */
    si_uid       :u32,	/* Real user ID of sending process */
    si_status    :i32,	/* Exit value or signal */
    si_utime     :i32,	/* User time consumed */
    si_stime     :i32,	/* System time consumed */
}


pub const SIG_DFL: usize = 0;
pub const SIG_IGN: usize = 1;
pub const SIG_ERR: usize = -1isize as usize;

/// Trap return function
/// # Description
/// Trap return funciton. Use jr for trampoline functions.
#[no_mangle]
pub fn trap_return() -> ! {
    set_user_trap_entry();

    let current = current_process().unwrap();
    let mut arcpcb = current.get_inner_locked();    
    let mut to_process: Option<(usize, usize)> = None;

    for sig in arcpcb.pending_sig.iter().enumerate() {
        if 1u64 << sig.1 & &arcpcb.sig_mask != 0 {
            to_process = Some((sig.0, *sig.1));
            break;
        }
    }

    let mut restore_vec = 0;
    let mut arg0 = 0;
    let mut arg1 = 0;
    let mut arg2 = 0;
    let mut arg3 = 0;
    let mut arg4 = 0;

    if let Some((idx, signal)) = to_process {
        let trap_cx_ptr = TRAP_CONTEXT;
        let user_satp = current_satp();
        extern "C" {
            fn strampoline();
            fn __restore();
            fn __restore_to_signal_handler();
            fn __siginfo();
        }
        let restore_to_signal_handler_va = __restore_to_signal_handler as usize - strampoline as usize + TRAMPOLINE;

        arcpcb.pending_sig.remove(idx);
        let terminate_self_va = crate::process::default_handlers::def_terminate_self as usize - strampoline as usize + TRAMPOLINE;
        let ignore_va = crate::process::default_handlers::def_ignore as usize - strampoline as usize + TRAMPOLINE;
        let handler_va = if let Some(act) = arcpcb.handlers.get(&signal) {
            if act.flags.contains(SignalFlags::SIGINFO) {
                act.sigaction.0
            } else if act.sighandler.0 == SIG_DFL {
                default_sig_handlers()[&signal].sighandler.0 as usize - strampoline as usize + TRAMPOLINE
            } else if act.sighandler.0 == SIG_IGN {
                ignore_va
            } else if act.sighandler.0 == SIG_ERR{
                terminate_self_va
            } else {
                act.sighandler.0
            }
        } else {
            terminate_self_va
        };
        let sig_info = SigInfo {
            si_signo:   signal as i32,
            si_errno:   0,
            si_code:    32767,     // SI_NOINFO
            si_trapno:  0,
            si_pid:     0,
            si_uid:     0,
            si_status:  0,
            si_utime:   0,
            si_stime:   0,
        };
        arcpcb.layout.write_user_data(VirtAddr::from(__siginfo as usize), &sig_info);
        
        if arcpcb.handlers.get(&signal).unwrap().flags.contains(SignalFlags::RESETHAND) {
            arcpcb.handlers.insert(signal, crate::process::default_sig_handlers()[&signal]);
        }
        
        // mask itself
        arcpcb.sig_mask |= 1u64 << signal;
        arcpcb.last_signal = Some(signal);

        drop(arcpcb);
        drop(current);
        drop(to_process);

        restore_vec = restore_to_signal_handler_va;
        arg0 = trap_cx_ptr;
        arg1 = user_satp;
        arg2 = handler_va;
        arg3 = signal;
        arg4 = __siginfo as usize;
    } else {
        drop(arcpcb);
        drop(current);
        drop(to_process);

        let trap_cx_ptr = TRAP_CONTEXT;
        let user_satp = current_satp();
        extern "C" {
            fn strampoline();
            fn __restore();
        }

        restore_vec = __restore as usize - strampoline as usize + TRAMPOLINE;
        arg0 = trap_cx_ptr;
        arg1 = user_satp;
    }
    
    unsafe {
        llvm_asm!("fence.i" :::: "volatile");
        llvm_asm!(
            "jr $0" :: 
            "r"(restore_vec), 
            "{a0}"(arg0), 
            "{a1}"(arg1), 
            "{a2}"(arg2),
            "{a3}"(arg3),
            "{a4}"(arg4) :: 
            "volatile"
        );  
    }

    unreachable!("Unreachable in trap_return!");
}