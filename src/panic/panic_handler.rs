use core::panic::PanicInfo;
use crate::memory::KERNEL_MEM_LAYOUT;
use crate::{process::current_process, sbi::shutdown};

/// The panic handler.  
/// On panic, it will print panic information then shutdown the machine.
#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        fatal!("Panic @ {}:{} : {}", location.file(), location.line(), info.message().unwrap());
    } else {
        fatal!("Panic @ ?:? : {}", info.message().unwrap());
    }
    fatal!("KERNELMemory layout: ");
    unsafe {
        KERNEL_MEM_LAYOUT.force_unlock();
    }
    KERNEL_MEM_LAYOUT.lock().print_layout();
    shutdown();
}