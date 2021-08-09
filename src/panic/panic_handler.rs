use core::panic::PanicInfo;
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
    fatal!("Memory layout: ");
    unsafe {
        current_process().unwrap().inner.force_unlock();
    }
    current_process().unwrap().get_inner_locked().layout.print_layout();
    shutdown();
}