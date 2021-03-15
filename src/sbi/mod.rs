mod sbi_funcs;
#[macro_use]
mod primitive_io;

pub use sbi_funcs::shutdown;
pub use sbi_funcs::get_byte;
pub use sbi_funcs::put_byte;
pub use sbi_funcs::sbi_call;

pub use primitive_io::putc;
pub use primitive_io::getc;
pub use primitive_io::puts;

pub use primitive_io::print;
pub use primitive_io::set_color;
pub use primitive_io::set_log_color;
pub use primitive_io::reset_color;
pub use primitive_io::log;

pub use primitive_io::LogLevel;
