//! This handles kernel formatted output.
#![allow(unused)]

use super::{get_byte, put_byte, get_time_ms};
use core::fmt::{self, Write};

/// Return the minimal log level of this build.
fn min_log_level() -> LogLevel {
    if cfg!(feature = "min_log_level_fatal") {
        return LogLevel::Fatal;
    } else if cfg!(feature = "min_log_level_error") {
        return LogLevel::Error;
    } else if cfg!(feature = "min_log_level_warning") {
        return LogLevel::Warning;
    } else if cfg!(feature = "min_log_level_info") {
        return LogLevel::Info;
    } else if cfg!(feature = "min_log_level_debug") {
        return LogLevel::Debug;
    } else {
        return LogLevel::Verbose;
    }
}


// ======================== color constants ========================
const FG_BLACK      :u8 = 30;
const FG_RED        :u8 = 31;
const FG_GREEN      :u8 = 32;
const FG_YELLOW     :u8 = 33;
const FG_BLUE       :u8 = 34;
const FG_MAGENTA    :u8 = 35;
const FG_CYAN       :u8 = 36;
const FG_WHITE      :u8 = 37;

const FG_B_BLACK    :u8 = 90;
const FG_B_RED      :u8 = 91;
const FG_B_GREEN    :u8 = 92;
const FG_B_YELLOW   :u8 = 93;
const FG_B_BLUE     :u8 = 94;
const FG_B_MAGENTA  :u8 = 95;
const FG_B_CYAN     :u8 = 96;
const FG_B_WHITE    :u8 = 97;

const FG_DEFAULT    :u8 = 39;

const BG_BLACK      :u8 = 40;
const BG_RED        :u8 = 41;
const BG_GREEN      :u8 = 42;
const BG_YELLOW     :u8 = 43;
const BG_BLUE       :u8 = 44;
const BG_MAGENTA    :u8 = 45;
const BG_CYAN       :u8 = 46;
const BG_WHITE      :u8 = 47;

const BG_B_BLACK    :u8 = 100;
const BG_B_RED      :u8 = 101;
const BG_B_GREEN    :u8 = 102;
const BG_B_YELLOW   :u8 = 103;
const BG_B_BLUE     :u8 = 104;
const BG_B_MAGENTA  :u8 = 105;
const BG_B_CYAN     :u8 = 106;
const BG_B_WHITE    :u8 = 107;

const BG_DEFAULT    :u8 = 49;

// ======================== utf-8 handle ========================

/// Put a single char to SBI
/// # Description
/// Put a single char to SBI. The char will be first decoded to UTF-8 byte sequence, then output to SBI.
/// # Example
/// ```
/// putc('你');
/// putc('好');
/// ```
pub fn putc(ch: char) {
    let mut buf = [0u8; 4];
    for code in ch.encode_utf8(&mut buf).as_bytes().iter() {
        put_byte(*code as u8);
    }
}

/// Get a UTF-8 char from SBI
/// # Description
/// This function will try to accept a utf-8 byte sequence, then decode it into a UTF-8 char.  
/// It will return an `'�'` when an invalid utf-8 sequence is encountered.
pub fn getc() -> char { // utf-8 to char
    let mut buf : u32;
    let init : u8 = get_byte();
    let length : u8;
    if init < 0b10000000 {
        return init as char;
    }
    else if init < 0b11100000 {length = 2;}
    else if init < 0b11110000 {length = 3;}
    else if init < 0b11111000 {length = 4;}
    else if init < 0b11111100 {length = 5;}
    else if init < 0b11111110 {length = 6;}
    else { return '�'; }     // illegal utf-8 sequence
    buf = (init & (0b01111111 >> length)) as u32;

    for _i in 1..length {
        let b = get_byte();
        if b & 0b11000000 != 0b10000000 { return '�'; }
        assert_eq!(b & 0b11000000, 0b10000000); // check utf-8 sequence
        buf <<= 6;
        buf += (b & 0b00111111) as u32;
    }
    
    match char::from_u32(buf) {
        None => '�',    // unknown sequence
        Some(res) => res
    }
}

/// print a &str to SBI
/// # Example
/// ```
/// puts("Hello world!");
/// ```
pub fn puts(s: &str) {
    for c in s.chars() {
        putc(c);
    }
}

// ======================== print! and println! support ========================

/// The zero-length struct OutputFormatter
struct OutputFormatter;

impl Write for OutputFormatter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        puts(s);
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    OutputFormatter.write_fmt(args).unwrap();
}

/// The great print! macro. Prints to the standard output.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::sbi::print(format_args!($($arg)*));
    }
}

/// The great println! macro. Prints to the standard output. Also prints a linefeed (`\\n`, or U+000A).
#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\n")
    };
    
    ($($arg:tt)*) => {
        $crate::print!("{}\n", format_args!($($arg)*))
    };
}

// ======================== log ========================

/// kernel output log level
#[derive(PartialOrd)]
#[derive(PartialEq)]
#[derive(Copy)]
#[derive(Clone)]
pub enum LogLevel {
    Verbose = 0,
    Debug   = 1,
    Info    = 2,
    Warning = 3,
    Error   = 4,
    Fatal   = 5,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::result::Result<(), core::fmt::Error> { 
        let s = match *self {
            LogLevel::Verbose   => "[ VERBOSE ]",
            LogLevel::Debug     => "[  DEBUG  ]",
            LogLevel::Info      => "[   INFO  ]",
            LogLevel::Warning   => "[ WARNING ]",
            LogLevel::Error     => "[  ERROR  ]",
            LogLevel::Fatal     => "[  FATAL  ]"
        };
        f.write_str(s)
    }
}

/// Set console color according to the log level
pub fn set_log_color(ll: LogLevel) {
    match ll {
        LogLevel::Verbose   => set_color(FG_B_BLACK,    BG_DEFAULT),
        LogLevel::Debug     => set_color(FG_DEFAULT,    BG_DEFAULT),
        LogLevel::Info      => set_color(FG_B_GREEN,    BG_DEFAULT),
        LogLevel::Warning   => set_color(FG_B_YELLOW,   BG_DEFAULT),
        LogLevel::Error     => set_color(FG_B_RED,      BG_DEFAULT),
        LogLevel::Fatal     => set_color(FG_BLACK,      BG_B_RED  )
    }
}

/// Set foreground color and background color.  
/// Foreground and background color codes are from [ANSI Escape Codes](https://en.wikipedia.org/wiki/ANSI_escape_code)
pub fn set_color(fg: u8, bg: u8) {
    print!("\x1b[{};{}m", fg, bg);
}

/// Reset console color to default.
pub fn reset_color() {
    set_color(FG_DEFAULT, BG_DEFAULT);
}

/// Print log info, alongside with log level, source file and line number.  
/// *Don't call this function. Use marcos instead.*
pub fn log(log_level: LogLevel, args: fmt::Arguments, file: &'static str, line: u32) {
    if log_level >= min_log_level() {
        set_log_color(log_level);
        print!("[{:#11.5}]{} {:>#30} @ {:<#5} : ", get_time_ms(), log_level, file, line);
        print(args);
        reset_color();
        println!();
    }
}

/// Print log info, alongside with log level, source file and line number. Will not print if the log level is lower then the min_log_level.
/// # Examples
/// ```
/// verbose!("This is a verbose message!");
/// ```
#[macro_export]
macro_rules! verbose {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::sbi::log(crate::sbi::LogLevel::Verbose, format_args!($fmt $(, $($arg)+)?), file!(), line!())
    };
}


/// Print log info, alongside with log level, source file and line number. Will not print if the log level is lower then the min_log_level.
/// # Examples
/// ```
/// debug!("This is a debug message!");
/// ```
#[macro_export]
macro_rules! debug {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::sbi::log(crate::sbi::LogLevel::Debug, format_args!($fmt $(, $($arg)+)?), file!(), line!())
    };
}


/// Print log info, alongside with log level, source file and line number. Will not print if the log level is lower then the min_log_level.
/// # Examples
/// ```
/// info!("This is an info message!");
/// ```
#[macro_export]
macro_rules! info {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::sbi::log(crate::sbi::LogLevel::Info, format_args!($fmt $(, $($arg)+)?), file!(), line!())
    };
}


/// Print log info, alongside with log level, source file and line number. Will not print if the log level is lower then the min_log_level.
/// # Examples
/// ```
/// warning!("This is an warning message!");
/// ```
#[macro_export]
macro_rules! warning {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::sbi::log(crate::sbi::LogLevel::Warning, format_args!($fmt $(, $($arg)+)?), file!(), line!())
    };
}


/// Print log info, alongside with log level, source file and line number. Will not print if the log level is lower then the min_log_level.
/// # Examples
/// ```
/// error!("This is an error message!");
/// ```
#[macro_export]
macro_rules! error {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::sbi::log(crate::sbi::LogLevel::Error, format_args!($fmt $(, $($arg)+)?), file!(), line!())
    };
}


/// Print log info, alongside with log level, source file and line number. Will not print if the log level is lower then the min_log_level.
/// # Examples
/// ```
/// fatal!("This is a fatal message!");
/// ```
#[macro_export]
macro_rules! fatal {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::sbi::log(crate::sbi::LogLevel::Fatal, format_args!($fmt $(, $($arg)+)?), file!(), line!())
    };
}