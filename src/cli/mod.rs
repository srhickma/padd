extern crate clap;
extern crate colored;
extern crate crypto;
extern crate regex;
extern crate stopwatch;

use {
    cli::formatter::{FormatCommand, FormatMetrics},
    std::{env, path::Path, process::Command},
};

use self::{
    clap::{App, AppSettings, Arg, ArgMatches, SubCommand},
    colored::{ColoredString, Colorize},
    regex::Regex,
    stopwatch::Stopwatch,
};

mod formatter;
mod logger;
mod server;
mod thread_pool;
mod tracker;

pub fn run() {
    let matches = build_app();

    if let Some(matches) = matches.subcommand_matches("fmt") {
        if server::running() {
            let mut command = String::new();
            env::args()
                .skip(1)
                .for_each(|arg| command = format!("{} {}", command, arg));

            server::send_command(command);
        } else {
            fmt(&matches);
        }
    }

    if let Some(matches) = matches.subcommand_matches("forget") {
        forget(&matches);
    }

    if let Some(matches) = matches.subcommand_matches("daemon") {
        daemon(&matches);
    }

    if matches.subcommand_matches("start-server").is_some() {
        server::start();
    }
}

fn build_app<'a>() -> ArgMatches<'a> {
    App::new("padd")
        .version("0.1.0")
        .author("Shane Hickman <srhickma@edu.uwaterloo.ca>")
        .about("Text formatter for context-free languages")
        .setting(AppSettings::SubcommandRequired)
        .setting(AppSettings::VersionlessSubcommands)
        .subcommand(
            SubCommand::with_name("fmt")
                .about("Formatter")
                .arg(
                    Arg::with_name("spec")
                        .help("Specification file path")
                        .takes_value(true)
                        .value_name("SPECIFICATION")
                        .required(true),
                )
                .arg(
                    Arg::with_name("target")
                        .short("t")
                        .long("target")
                        .help("Sets a the path to format files under")
                        .takes_value(true)
                        .value_name("PATH")
                        .required(true),
                )
                .arg(
                    Arg::with_name("matching")
                        .short("m")
                        .long("matching")
                        .help("Sets the regex for file names to format")
                        .takes_value(true)
                        .value_name("REGEX")
                        .requires("target"),
                )
                .arg(
                    Arg::with_name("threads")
                        .long("threads")
                        .help("Sets the number of worker threads")
                        .takes_value(true)
                        .value_name("NUM"),
                )
                .arg(
                    Arg::with_name("no-skip").long("no-skip").help(
                        "Do not skip files which haven't changed since they were last formatted",
                    ),
                )
                .arg(
                    Arg::with_name("no-track")
                        .long("no-track")
                        .help("Do not track file changes"),
                )
                .arg(
                    Arg::with_name("no-write")
                        .long("no-write")
                        .help("Do not write changes to files"),
                ),
        )
        .subcommand(
            SubCommand::with_name("forget")
                .about("Clears all file tracking data")
                .arg(
                    Arg::with_name("target")
                        .help("Sets the target directory to clear tracking data under")
                        .takes_value(true)
                        .value_name("PATH")
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("daemon")
                .about("Daemon specific commands")
                .subcommand(
                    SubCommand::with_name("start").about("Start padd server in daemon mode"),
                )
                .subcommand(SubCommand::with_name("kill").about("Stop the padd daemon")),
        )
        .subcommand(SubCommand::with_name("start-server").about("Start a padd server"))
        .get_matches()
}

fn fmt(matches: &ArgMatches) {
    let mut sw = Stopwatch::new();
    sw.start();

    let spec_path = matches.value_of("spec").unwrap();

    let formatter = match formatter::generate_formatter(&spec_path) {
        Err(err) => {
            logger::fatal(&format!(
                "Error loading specification {}: {}",
                &spec_path, err
            ));
            return;
        }
        Ok(formatter) => formatter,
    };

    let target_path = Path::new(matches.value_of("target").unwrap());

    let file_regex: Option<Regex> = match matches.value_of("matching") {
        None => None,
        Some(regex) => match Regex::new(regex) {
            Ok(fn_regex) => Some(fn_regex),
            Err(err) => {
                logger::fatal(&format!("Failed to build file name regex: {}", err));
                None
            }
        },
    };

    let thread_count: usize = match matches.value_of("threads") {
        None => 1,
        Some(threads) => match str::parse::<usize>(threads) {
            Err(_) => {
                logger::err(&format!(
                    "Invalid number of threads: '{}'. Falling back to one thread",
                    threads
                ));
                1
            }
            Ok(threads) => {
                if threads == 0 {
                    logger::err(&format!(
                        "Invalid number of threads: '{}'. Falling back to one thread",
                        threads
                    ));
                    1
                } else {
                    threads
                }
            }
        },
    };

    let no_skip = matches.is_present("no-skip");
    let no_track = matches.is_present("no-track");
    let no_write = matches.is_present("no-write");

    println!();

    let metrics = formatter::format(FormatCommand {
        formatter,
        target_path,
        file_regex,
        thread_count,
        no_skip,
        no_track,
        no_write,
    });

    sw.stop();
    print_final_status(sw.elapsed_ms(), metrics);
}

fn print_final_status(elapsed_ms: i64, metrics: FormatMetrics) {
    let unchanged = metrics.total - metrics.failed - metrics.formatted;

    let mut unchanged_msg = format!("{} unchanged", unchanged).normal();
    if unchanged > 0 {
        unchanged_msg = unchanged_msg.yellow()
    }

    let mut formatted_msg: ColoredString = format!("{} formatted", metrics.formatted).normal();
    if metrics.formatted > 0 {
        formatted_msg = formatted_msg.bright_green()
    }

    let mut failed_msg = format!("{} failed", metrics.failed).normal();
    if metrics.failed > 0 {
        failed_msg = failed_msg.bright_red()
    }

    println!();
    logger::info(&format!(
        "COMPLETE: {}ms : {} processed, {}, {}, {}",
        elapsed_ms, metrics.total, unchanged_msg, formatted_msg, failed_msg
    ));
}

fn forget(matches: &ArgMatches) {
    let target: &Path = Path::new(matches.value_of("target").unwrap());
    tracker::clear_tracking(target);
}

fn daemon(matches: &ArgMatches) {
    if matches.subcommand_matches("start").is_some() {
        if server::running() {
            logger::info(&format!("Daemon already running"));
        } else {
            let child = Command::new(&env::args().next().unwrap()[..])
                .arg("start-server")
                .spawn()
                .unwrap();

            logger::info(&format!("Starting padd daemon with pid {}", child.id()));
        }
    } else if matches.subcommand_matches("kill").is_some() {
        server::kill();
    }
}
