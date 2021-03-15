use core::fmt::{self, Write};
use crate::sbi::console_putchar;

struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            console_putchar(c as usize);
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}

pub fn set_log_status() -> i32 {
    if cfg!(LOG = "TRACE") {1} 
    else if cfg!(LOG = "DEBUG") {2}
    else if cfg!(LOG = "INFO") {3}
    else if cfg!(LOG = "WARN") {4}
    else if cfg!(LOG = "ERROR") {5}
    else {3}
}

macro_rules! abstract_log {
    ($name : expr, $level : expr, $color : expr, $fmt: literal $(, $($arg: tt)+)?) => {
        if (crate::console::set_log_status() <= $level) {
            $crate::console::print(format_args!(concat!("\x1b[{}m[{}][0] ", $fmt, "\x1b[0m\n"), $color, $name $(, $($arg)+)?));
        }
    }
}

#[macro_export]
macro_rules! error {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        abstract_log!("ERROR", 5, 31, $fmt $(, $($arg)+)?);
    }
}

#[macro_export]
macro_rules! warn {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        abstract_log!("WARN", 4, 93, $fmt $(, $($arg)+)?);
    }
}

#[macro_export]
macro_rules! info {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        abstract_log!("INFO", 3, 34, $fmt $(, $($arg)+)?);
    }
}

#[macro_export]
macro_rules! debug {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        abstract_log!("DEBUG", 2, 32, $fmt $(, $($arg)+)?);
    }
}

#[macro_export]
macro_rules! trace {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        abstract_log!("TRACE", 1, 90, $fmt $(, $($arg)+)?);
    }
}