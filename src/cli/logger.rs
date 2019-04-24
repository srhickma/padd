extern crate colored;

use std::process;

use self::colored::{ColoredString, Colorize};

lazy_static! {
    static ref PREFIX_ERR: ColoredString = "error".bright_red();
    static ref PREFIX_FATAL: ColoredString = "fatal".on_bright_red();
    static ref PREFIX_FMT: ColoredString = "  FMT".bright_blue();
    static ref PREFIX_FMT_OK: ColoredString = "   OK".bright_green();
    static ref PREFIX_FMT_ERR: ColoredString = "ERROR".bright_red();
}

pub fn info(string: &String) {
    println!("{}", string);
}

pub fn err(string: &String) {
    println!("{}: {}", *PREFIX_ERR, string);
}

pub fn fatal(string: &String) {
    println!("{}: {}", *PREFIX_FATAL, string);
    process::exit(1);
}

pub fn fmt(string: &String) {
    println!("{}| {}", *PREFIX_FMT, string);
}

pub fn fmt_ok(string: &String) {
    println!("{}| {}", *PREFIX_FMT_OK, string);
}

pub fn fmt_err(string: &String) {
    println!("{}| {}", *PREFIX_FMT_ERR, string);
}
