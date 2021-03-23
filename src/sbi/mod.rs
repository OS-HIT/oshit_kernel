#[macro_use]
mod primitive_io;
mod sbi_funcs;
mod timer;

pub use sbi_funcs::{
    set_timer,
    get_byte,
    put_byte,
    shutdown,
    sbi_call,
};

pub use primitive_io::{
    putc,
    getc,
    puts,
    print,
    set_color,
    set_log_color,
    reset_color,
    log,
    LogLevel,
};

pub use timer::{
    TICKS_PER_SECOND,
    get_time,
    get_time_ms,
    reset_timer_trigger,
};