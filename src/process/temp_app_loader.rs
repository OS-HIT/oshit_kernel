use crate::trap::TrapContext;
use crate::process::ProcessContext;
use crate::config::*;

// align to 4k page
// will push in trap context, then process context, then load to sp.
#[repr(align(4096))]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

// fixed user stack in real mem area. will change to vir in future.
#[repr(align(4096))]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

static KERNEL_STACK: [KernelStack; MAX_APP_NUM] = [
    KernelStack {data: [0; KERNEL_STACK_SIZE]};
    MAX_APP_NUM
];

static USER_STACK: [UserStack; MAX_APP_NUM] = [
    UserStack {data: [0; USER_STACK_SIZE]};
    MAX_APP_NUM
];

impl KernelStack {
    fn get_stack_bottom(&self) -> usize {
        return self.data.as_ptr() as usize + KERNEL_STACK_SIZE;
    }

    pub fn push_context(&self, trap_context: TrapContext, process_context: ProcessContext) -> *const ProcessContext {
        unsafe {
            let trap_context_p = (
                self.get_stack_bottom()
                - core::mem::size_of::<ProcessContext>()
            ) as *mut TrapContext;

            let process_context_p = (
                trap_context_p as usize 
                - core::mem::size_of::<ProcessContext>()
            ) as *mut ProcessContext;

            *trap_context_p = trap_context;
            *process_context_p = process_context;

            return process_context_p;
        }
    }
}

impl UserStack {
    fn get_stack_bottom(&self) -> usize {
        return self.data.as_ptr() as usize + USER_STACK_SIZE;
    }
}


// temp func: read info from link_app.asm. will deprecate in future.
pub fn get_app_count() -> usize {
    extern "C" {fn _num_app();};
    unsafe{ (_num_app as usize as *const usize).read_volatile() }
}

pub fn get_base(app_id: usize) -> usize {
    APP_BASE_ADDRESS + app_id * APP_SIZE_LIMIT
}

pub fn load_apps() {
    verbose!("Loading apps from asm...");
    extern "C" {fn _num_app();};
    let num_app_p = _num_app as usize as *const usize;
    let num_app = get_app_count();
    // read array from .asm
    let app_start = unsafe {
        core::slice::from_raw_parts(num_app_p.add(1), num_app + 1)
    };

    // clear i-cache, we are loading data into exec area!
    unsafe { asm!("fence.i"); }

    for i in 0..num_app {
        let base_i = get_base(i);
        // clear memory region
        for i in base_i..(base_i+APP_SIZE_LIMIT) {
            unsafe {
                (i as *mut u8).write_volatile(0);
            }
        }

        // load app from data section to memory
        let src = unsafe {
            core::slice::from_raw_parts(app_start[i] as *const u8, app_start[i + 1] - app_start[i])
        };
        let dst = unsafe {
            core::slice::from_raw_parts_mut(base_i as *mut u8, src.len())
        };
        dst.copy_from_slice(src);
    }
    info!("Apps loaded.");
}

pub fn init_app_context(app_id: usize) -> *const ProcessContext {
    KERNEL_STACK[app_id].push_context(
        TrapContext::init(get_base(app_id), USER_STACK[app_id].get_stack_bottom()),
        ProcessContext::init()
    )
}
