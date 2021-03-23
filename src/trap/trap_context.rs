use riscv::register::sstatus::{Sstatus, self, SPP};

pub struct TrapContext {
    pub regs    : [usize; 32],
    pub sstatus : Sstatus,
    pub sepc    : usize,
}

impl TrapContext {
    pub fn set_sp(&mut self, sp: usize) {
        self.regs[2] = sp;  // sp = x2
    }

    // set up trap context, with stack and sepc = entry (thus we sret to entry)
    pub fn init(entry: usize, sp: usize) -> Self {
        let mut sstatus = sstatus::read();
        sstatus.set_spp(SPP::User);
        let mut context = TrapContext {
            regs: [0; 32],
            sstatus,
            sepc: entry,
        };
        context.set_sp(sp);
        return context;
    }
}