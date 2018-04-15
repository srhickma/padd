extern crate padd;

use cli::run;
use padd::test;

mod cli;

fn main() {
    test();
    run();
}