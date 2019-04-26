extern crate clap;
extern crate colored;
extern crate regex;
extern crate stopwatch;

use {
    cli::{
        formatter::{self, FormatCommand, FormatMetrics},
        logger, server, tracker,
    },
    std::{env, path::Path, process::Command},
};

use self::{
    clap::ArgMatches,
    colored::{ColoredString, Colorize},
    regex::Regex,
    stopwatch::Stopwatch,
};

pub fn fmt(matches: &ArgMatches) {
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

pub fn print_final_status(elapsed_ms: i64, metrics: FormatMetrics) {
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

pub fn forget(matches: &ArgMatches) {
    let target: &Path = Path::new(matches.value_of("target").unwrap());
    tracker::clear_tracking(target);
}

pub fn daemon(matches: &ArgMatches) {
    if matches.subcommand_matches("start").is_some() {
        if server::running() {
            logger::info(&"Daemon already running".to_string());
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
