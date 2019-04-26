#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate clap;
extern crate padd;

mod cli;

fn main() {
    cli::run();
}
