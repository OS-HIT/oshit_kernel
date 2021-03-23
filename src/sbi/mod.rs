mod sbi_funcs;
#[macro_use]
mod primitive_io;

pub use sbi_funcs::{
    shutdown,
    get_byte,
    put_byte,
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