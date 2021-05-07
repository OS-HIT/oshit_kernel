use super::TrapContext;
use crate::syscall::syscall;
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

global_asm!(include_str!("./trap.asm"));

// enable traps
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

fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(kernel_trap as usize, stvec::TrapMode::Direct);
    }
}

fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE as usize, stvec::TrapMode::Direct);
    }
}

#[no_mangle]
pub fn kernel_trap() -> ! {
    fatal!("Fatal error: unhandled trap {:?}.", scause::read().cause());
    panic!("Kernel trap not supported yet!");
}

// no mangle so that call user_trap in asm won't break
// return cx for syscall res and new sepc.
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
        // TODO: Core dump and/or terminate user program and continue
        
        Trap::Exception(Exception::StoreFault) |
        Trap::Exception(Exception::StorePageFault) |
        Trap::Exception(Exception::InstructionFault) |
        Trap::Exception(Exception::InstructionPageFault) |
        Trap::Exception(Exception::LoadFault) |
        Trap::Exception(Exception::LoadPageFault) => {
            println!(
                "[kernel] {:?} in application, bad addr = {:#x}, bad instruction = {:#x}, core dumped.",
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