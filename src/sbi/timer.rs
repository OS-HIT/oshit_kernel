// warning is emmited due to missing features, ignore it.
use crate::config::CLOCK_FREQ;
use riscv::register::time;

// trigger per 1ms
pub const TICKS_PER_SECOND  : usize = 1000;
pub const MILLI_PER_SECOND  : usize = 1000;

pub fn get_time() -> usize {
    return time::read();
}

pub fn reset_timer_trigger() {
    super::set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SECOND);
}

pub fn get_time_ms() -> u64 {
    return get_time() as u64 / (CLOCK_FREQ / MILLI_PER_SECOND) as u64;
}
