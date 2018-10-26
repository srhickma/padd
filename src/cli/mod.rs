extern crate regex;
extern crate stopwatch;
extern crate clap;
extern crate colored;

use self::regex::Regex;
use self::stopwatch::Stopwatch;
use self::clap::{Arg, ArgMatches, App};
use self::colored::{Colorize, ColoredString};

use std::io::{self, Read, Write, Seek, SeekFrom, BufRead, BufReader};
use std::fs::{self, File, OpenOptions};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};

use padd::{self, FormatJobRunner, Stream, ThreadPool};

mod logger;

static FORMATTED: AtomicUsize = ATOMIC_USIZE_INIT;
static TOTAL: AtomicUsize = ATOMIC_USIZE_INIT;

pub fn run() {
    let matches = build_app();

    let spec_path = matches.value_of("spec").unwrap();

    logger::info(format!("Loading specification {} ...", spec_path));

    let fjr = match load_spec(&spec_path) {
        Err(err) => {
            logger::fatal(format!("Error loading specification {}: {}", &spec_path, err));
            return;
        }
        Ok(fjr) => fjr
    };

    logger::info(format!("Successfully loaded specification"));

    let target: Option<&Path> = match matches.value_of("target") {
        None => None,
        Some(file) => Some(Path::new(file))
    };

    let file_regex: Option<Regex> = match matches.value_of("matching") {
        None => None,
        Some(regex) => match Regex::new(format!(r#"{}"#, regex).as_str()) {
            Ok(fn_regex) => Some(fn_regex),
            Err(e) => {
                logger::fatal(format!("Failed to build file name regex: {}", e));
                None
            }
        }
    };

    let thread_count: usize = match matches.value_of("threads") {
        None => 1,
        Some(threads) => match str::parse::<usize>(threads) {
            Err(_) => {
                logger::err(format!("Invalid number of threads: '{}'. Falling back to one thread", threads));
                1
            }
            Ok(threads) => {
                if threads == 0 {
                    logger::err(format!("Invalid number of threads: '{}'. Falling back to one thread", threads));
                    1
                } else {
                    threads
                }
            }
        }
    };

    println!();

    let mut sw = Stopwatch::new();
    sw.start();

    let fjr_arc: Arc<FormatJobRunner> = Arc::new(fjr);

    let pool: ThreadPool<FormatPayload> = ThreadPool::spawn(
        thread_count,
        thread_count * 2,
        |payload: FormatPayload| {
            let file_path = Path::new(&payload.file_path);
            format_file(&file_path, &payload.fjr_arc)
        },
    );

    match target {
        Some(target_path) => {
            let fn_regex = match file_regex {
                Some(regex) => regex,
                None => Regex::new(r#".*"#).unwrap(),
            };

            format_target(target_path, &fn_regex, &fjr_arc, &pool)
        }
        None => term_loop(&fjr_arc)
    }

    pool.terminate_and_join();

    sw.stop();

    println!();
    print_final_status(sw.elapsed_ms());
}

fn build_app<'a>() -> ArgMatches<'a> {
    App::new("padd")
        .version("0.1.0")
        .author("Shane Hickman <srhickma@edu.uwaterloo.ca>")
        .about("Text formatter for context-free languages")
        .arg(Arg::with_name("spec")
            .help("Specification file path")
            .takes_value(true)
            .value_name("SPECIFICATION")
            .required(true)
        )
        .arg(Arg::with_name("target")
            .short("t")
            .long("target")
            .help("Sets a the path to format files under")
            .takes_value(true)
            .value_name("PATH")
        )
        .arg(Arg::with_name("matching")
            .short("m")
            .long("matching")
            .help("Sets the regex for file names to format")
            .takes_value(true)
            .value_name("REGEX")
            .requires("target")
        )
        .arg(Arg::with_name("threads")
            .long("threads")
            .help("Sets the number of worker threads")
            .takes_value(true)
            .value_name("NUM")
        )
        .get_matches()
}

fn load_spec(spec_path: &str) -> Result<FormatJobRunner, padd::BuildError> {
    let mut spec = String::new();

    let spec_file = File::open(spec_path);
    match spec_file {
        Ok(_) => {
            match spec_file.unwrap().read_to_string(&mut spec) {
                Ok(_) => {}
                Err(e) => {
                    logger::fatal(format!("Could not read specification file \"{}\": {}", &spec_path, e));
                }
            }
        }
        Err(e) => logger::fatal(format!("Could not find specification file \"{}\": {}", &spec_path, e)),
    }

    FormatJobRunner::build(&spec)
}

fn format_target(target_path: &Path, fn_regex: &Regex, fjr_arc: &Arc<FormatJobRunner>, pool: &ThreadPool<FormatPayload>) {
    let file_name = target_path.file_name().unwrap().to_str().unwrap();
    if target_path.is_dir() {
        fs::read_dir(target_path).unwrap()
            .for_each(|res| {
                match res {
                    Ok(dir_item) => format_target(&dir_item.path(), fn_regex, fjr_arc, pool),
                    Err(e) => logger::fmt_err(format!("An error occurred while searching directory {}: {}", target_path.to_string_lossy(), e)),
                }
            });
    } else if fn_regex.is_match(file_name) {
        pool.enqueue(FormatPayload {
            fjr_arc: fjr_arc.clone(),
            file_path: target_path.to_string_lossy().to_string(),
        });
    }
}

fn term_loop(fjr_arc: &Arc<FormatJobRunner>) {
    loop {
        let mut target_path = String::new();

        match io::stdin().read_line(&mut target_path) {
            Ok(_) => {}
            Err(e) => {
                logger::fmt_err(format!("Failed to read target file \"{}\": {}", target_path, e));
                continue;
            }
        }

        target_path.pop();

        format_file(&Path::new(&target_path), &fjr_arc);
    }
}

fn format_file(target_path: &Path, fjr: &FormatJobRunner) {
    TOTAL.fetch_add(1, Ordering::SeqCst);
    if format_file_internal(target_path, fjr) {
        FORMATTED.fetch_add(1, Ordering::SeqCst);
    }
}

fn format_file_internal(target_path: &Path, fjr: &FormatJobRunner) -> bool {
    logger::fmt(target_path.to_string_lossy().to_string());
    let target_file = OpenOptions::new().read(true).write(true).open(&target_path);
    match target_file {
        Ok(_) => {
            let mut target = target_file.unwrap();

            let result = {
                let mut reader = BufReader::new(&target);

                let mut buffer: Vec<char> = Vec::new();
                let mut cursor: usize = 0;

                let mut getter = &mut || {
                    match buffer.get(cursor) {
                        None => {}
                        Some(c) => {
                            cursor += 1;
                            return Some(*c);
                        }
                    };

                    let mut in_buf = String::new();
                    match reader.read_line(&mut in_buf) {
                        Ok(_) => {}
                        Err(e) => {
                            logger::fmt_err(format!("Could not read target file \"{}\": {}", &target_path.to_string_lossy(), e));
                            return None;
                        }
                    };

                    buffer = in_buf.chars().collect();
                    cursor = 1;
                    in_buf.chars().next()
                };

                let mut stream = Stream::from(&mut getter);

                fjr.format(&mut stream)
            };

            match result {
                Ok(res) => {
                    match target.seek(SeekFrom::Start(0)) {
                        Ok(_) => {}
                        Err(e) => {
                            logger::fmt_err(format!("Could not seek to start of target file \"{}\": {}", &target_path.to_string_lossy(), e));
                            return false;
                        }
                    }
                    match target.set_len(0) {
                        Ok(_) => {}
                        Err(e) => {
                            logger::fmt_err(format!("Could not clear target file \"{}\": {}", &target_path.to_string_lossy(), e));
                            return false;
                        }
                    }
                    match target.write_all(res.as_bytes()) {
                        Ok(_) => logger::fmt_ok(target_path.to_string_lossy().to_string()),
                        Err(e) => {
                            logger::fmt_err(format!("Could not write to target file \"{}\": {}", &target_path.to_string_lossy(), e));
                            return false;
                        }
                    }
                }
                Err(e) => {
                    logger::fmt_err(format!("Error formatting {}: {}", &target_path.to_string_lossy(), e));
                    return false;
                }
            }
        }
        Err(e) => {
            logger::fmt_err(format!("Could not find target file \"{}\": {}", &target_path.to_string_lossy(), e));
            return false;
        }
    }
    true
}

struct FormatPayload {
    fjr_arc: Arc<FormatJobRunner>,
    file_path: String,
}

fn print_final_status(elapsed_ms: i64) {
    let total = TOTAL.load(Ordering::Relaxed);
    let formatted = FORMATTED.load(Ordering::Relaxed);

    let mut formatted_msg: ColoredString = format!("{} formatted", formatted).normal();
    if formatted > 0 {
        formatted_msg = formatted_msg.bright_green()
    }

    let mut failed_msg = format!("{} failed", total - formatted).normal();
    if total > formatted {
        failed_msg = failed_msg.bright_red()
    }

    logger::info(format!("COMPLETE: {}ms : {} processed, {}, {}", elapsed_ms, total, formatted_msg, failed_msg));
}
