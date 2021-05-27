use core::panic::PanicInfo;
use crate::sbi::shutdown;

/// The panic handler.  
/// On panic, it will print panic information then shutdown the machine.
#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        fatal!("Panic @ {}:{} : {}", location.file(), location.line(), info.message().unwrap());
    } else {
        fatal!("Panic @ ?:? : {}", info.message().unwrap());
    }
    shutdown();
}