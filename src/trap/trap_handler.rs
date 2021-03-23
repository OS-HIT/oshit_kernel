use super::TrapContext;
use crate::syscall::syscall;
use riscv::register::{
    stvec,      // s trap vector base address register
    scause::{   // s cause register
        self,
        Trap,
        Exception,
    },
    stval,      // s trap value, exception spcific.
};

global_asm!(include_str!("./trap.asm"));

// enable traps
pub fn init() {
    unsafe {
        extern "C" { fn __alltraps(); }
        // write trap handler vector, as well as trap mode
        stvec::write(__alltraps as usize, stvec::TrapMode::Direct);
    }
}

// no mangle so that call trap_handler in asm won't break
// return cx for syscall res and new sepc.
#[no_mangle]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            cx.sepc += 4;   // so that we don't stuck at one instruction
            cx.regs[10] = syscall(cx.regs[17], [cx.regs[10], cx.regs[11], cx.regs[12]]) as usize;   // exec syscall in s-mode
        },
        // TODO: Core dump and/or terminate user program and continue
        _ => {
            fatal!("Fatal error: {:?}.", scause.cause());
            fatal!("Bad addr @ 0x{:#X}, Bad Inst @ 0x{:#X}", stval, cx.sepc);
            panic!("Irrecoverable error, kernel panic.");
        }
    }
    return cx;
}