use riscv::register::sstatus::{Sstatus, self, SPP};

pub struct TrapContext {
    pub regs            : [usize; 32],
    pub sstatus         : Sstatus,
    pub sepc            : usize,
    pub kernel_satp     : usize,
    pub kernel_sp       : usize,
    pub user_trap       : usize,
}

impl TrapContext {
    pub fn set_sp(&mut self, sp: usize) {
        info!("Setting TrapContext user sp to {:0x}", sp);
        self.regs[2] = sp;  // sp = x2
    }

    // set up trap context, with stack and sepc = entry (thus we sret to entry)
    pub fn init(
        entry: usize, 
        sp: usize,
        kernel_satp: usize,
        kernel_sp: usize,
        user_trap: usize
    ) -> Self {
        info!("init TrapContext kernel_sp to {:0x}", kernel_sp);
        let mut sstatus = sstatus::read();
        sstatus.set_spp(SPP::User);
        let mut context = TrapContext {
            regs: [0; 32],
            sstatus,
            sepc: entry,
            kernel_satp,
            kernel_sp,
            user_trap,
        };
        context.set_sp(sp);
        return context;
    }
}