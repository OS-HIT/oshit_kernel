//! Timer related sbi calls.
use crate::config::CLOCK_FREQ;
use riscv::register::time;

// trigger per 1ms
pub const TICKS_PER_SECOND  : u64 = 100;
pub const MILLI_PER_SECOND  : u64 = 1000;

/// Get times elaped since boot, in cycles.
pub fn get_time() -> u64 {
    time::read() as u64
}

/// reset the timer trigger to next target
pub fn reset_timer_trigger() {
    super::set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SECOND);
}

// pub fn get_time_ms() -> f64 {
//     return get_time() as f64 / (CLOCK_FREQ / MILLI_PER_SECOND) as f64;
// }

/// get milisecond since boot.
pub fn get_time_ms() -> u64 {
    return get_time() as u64 / (CLOCK_FREQ / MILLI_PER_SECOND) as u64;
}
