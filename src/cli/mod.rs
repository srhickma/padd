extern crate clap;
extern crate yaml_rust;

use self::{clap::App, yaml_rust::yaml::Yaml};

mod cmd;
mod formatter;
mod logger;
#[cfg(test)]
pub mod server;
//#[cfg(ccstop)]
#[cfg(not(test))]
mod server;
mod thread_pool;
mod tracker;

lazy_static! {
    static ref CLAP_CONFIG: Yaml = load_yaml!("../../res/clap_config.yml").clone();
}

pub fn run(args: Vec<&str>) {
    let matches = App::from_yaml(&CLAP_CONFIG).get_matches_from(args.clone());
    let command = args.join(" ");

    logger::init(&matches);

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
        cmd::daemon(&matches, &args);
    }

    if matches.subcommand_matches("start-server").is_some() {
        server::start(&CLAP_CONFIG);
    }
}
