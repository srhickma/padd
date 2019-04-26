extern crate clap;
extern crate yaml_rust;

use std::env;

use self::{clap::App, yaml_rust::yaml::Yaml};

mod cmd;
mod formatter;
mod logger;
mod server;
mod thread_pool;
mod tracker;

lazy_static! {
    static ref CLAP_CONFIG: Yaml = load_yaml!("../../res/clap_config.yml").clone();
}

pub fn run() {
    let matches = App::from_yaml(&CLAP_CONFIG).get_matches();
    let args: Vec<String> = env::args().collect();
    let command = args.join(" ");

    if let Some(matches) = matches.subcommand_matches("fmt") {
        if server::running() {
            server::send_command(command);
        } else {
            cmd::fmt(&matches);
        }
    }

    if let Some(matches) = matches.subcommand_matches("forget") {
        cmd::forget(&matches);
    }

    if let Some(matches) = matches.subcommand_matches("daemon") {
        cmd::daemon(&matches);
    }

    if matches.subcommand_matches("start-server").is_some() {
        server::start(&CLAP_CONFIG);
    }
}
