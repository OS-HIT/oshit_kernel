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
use crate::process::suspend_switch;
use crate::config::*;
use crate::process::{get_current_trap_context, get_current_satp};

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
    let cx = get_current_trap_context();
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;   // so that we don't stuck at one instruction
            cx.regs[10] = syscall(cx.regs[17], [cx.regs[10], cx.regs[11], cx.regs[12]]) as usize;   // exec syscall in s-mode
        },
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            reset_timer_trigger();
            suspend_switch();
        },
        // TODO: Core dump and/or terminate user program and continue
        _ => {
            fatal!("Fatal error: unhandled trap {:?}.", scause.cause());
            fatal!("Bad addr @ 0x{:#X}, Bad Inst @ 0x{:#X}", stval, cx.sepc);
            panic!("Irrecoverable error, kernel panic.");
        }
    }
    trap_return();
}

#[no_mangle]
pub fn trap_return() -> ! {
    set_user_trap_entry();
    let trap_cx_ptr = TRAP_CONTEXT;
    let user_satp = get_current_satp();
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