extern crate clap;
extern crate colored;
extern crate crypto;
extern crate regex;
extern crate stopwatch;

use {
    cli::thread_pool::ThreadPool,
    padd::{self, FormatJob, FormatJobRunner},
    std::{
        cmp,
        fs::{self, File, OpenOptions},
        io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write},
        path::{Path, PathBuf},
        str::FromStr,
        sync::{
            Arc,
            atomic::{ATOMIC_USIZE_INIT, AtomicUsize, Ordering},
        },
        time::{Duration, SystemTime, UNIX_EPOCH},
    },
};

use self::{
    clap::{App, AppSettings, Arg, ArgMatches, SubCommand},
    colored::{ColoredString, Colorize},
    crypto::{
        digest::Digest,
        sha2::Sha256,
    },
    regex::Regex,
    stopwatch::Stopwatch,
};

mod logger;
mod thread_pool;

const TRACKER_DIR: &str = ".padd";
const TRACKER_EXTENSION: &str = ".trk";

static FORMATTED: AtomicUsize = ATOMIC_USIZE_INIT;
static FAILED: AtomicUsize = ATOMIC_USIZE_INIT;
static TOTAL: AtomicUsize = ATOMIC_USIZE_INIT;

pub fn run() {
    let matches = build_app();

    if let Some(matches) = matches.subcommand_matches("fmt") {
        fmt(&matches);
    }

    if let Some(matches) = matches.subcommand_matches("forget") {
        forget(&matches);
    }
}

fn build_app<'a>() -> ArgMatches<'a> {
    App::new("padd")
        .version("0.1.0")
        .author("Shane Hickman <srhickma@edu.uwaterloo.ca>")
        .about("Text formatter for context-free languages")
        .setting(AppSettings::SubcommandRequired)
        .setting(AppSettings::VersionlessSubcommands)
        .subcommand(SubCommand::with_name("fmt")
            .about("Formatter")
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
            .arg(Arg::with_name("no-skip")
                .long("no-skip")
                .help("Do not skip files which haven't changed since they were last formatted")
            )
            .arg(Arg::with_name("no-track")
                .long("no-track")
                .help("Do not track file changes")
            )
            .arg(Arg::with_name("no-write")
                .long("no-write")
                .help("Do not write changes to files")
            )
        )
        .subcommand(SubCommand::with_name("forget")
            .about("Clears all file tracking data")
            .arg(Arg::with_name("target")
                .short("t")
                .long("target")
                .help("Sets a the path to format files under")
                .takes_value(true)
                .value_name("PATH")
                .required(true)
            )
        )
        .get_matches()
}

fn fmt(matches: &ArgMatches) {
    let mut sw = Stopwatch::new();
    sw.start();

    let spec_path = matches.value_of("spec").unwrap();

    logger::info(format!("Loading specification {} ...", spec_path));

    let (fjr, spec_sha) = match load_spec(&spec_path) {
        Err(err) => {
            logger::fatal(format!("Error loading specification {}: {}", &spec_path, err));
            return;
        }
        Ok(fjr) => fjr
    };

    logger::info(format!("Successfully loaded specification: sha256: {}", &spec_sha));

    let target: Option<&Path> = match matches.value_of("target") {
        None => None,
        Some(file) => Some(Path::new(file))
    };

    let file_regex: Option<Regex> = match matches.value_of("matching") {
        None => None,
        Some(regex) => match Regex::new(regex) {
            Ok(fn_regex) => Some(fn_regex),
            Err(err) => {
                logger::fatal(format!("Failed to build file name regex: {}", err));
                None
            }
        }
    };

    let thread_count: usize = match matches.value_of("threads") {
        None => 1,
        Some(threads) => match str::parse::<usize>(threads) {
            Err(_) => {
                logger::err(format!(
                    "Invalid number of threads: '{}'. Falling back to one thread", threads
                ));
                1
            }
            Ok(threads) => {
                if threads == 0 {
                    logger::err(format!(
                        "Invalid number of threads: '{}'. Falling back to one thread", threads
                    ));
                    1
                } else {
                    threads
                }
            }
        }
    };

    let no_skip = matches.is_present("no-skip");
    let no_track = matches.is_present("no-track");
    let no_write = matches.is_present("no-write");

    println!();

    let fjr_arc: Arc<FormatJobRunner> = Arc::new(fjr);

    let pool: ThreadPool<FormatPayload> = ThreadPool::spawn(
        thread_count,
        thread_count * 2,
        |payload: FormatPayload| {
            let file_path = payload.file_path.as_path();
            format_file(file_path, &payload.fjr_arc, payload.no_write);

            if !payload.no_track {
                track_file(file_path, &payload.spec_sha);
            }
        },
    );

    match target {
        Some(target_path) => {
            let fn_regex = match file_regex {
                Some(regex) => regex,
                None => Regex::new(r#".*"#).unwrap(),
            };

            let criteria = TargetSearchCriteria {
                fn_regex: &fn_regex,
                spec_sha: &spec_sha,
                no_skip,
                no_track,
                no_write,
                fjr_arc: &fjr_arc,
                pool: &pool,
            };

            format_target(target_path, &criteria)
        }
        None => term_loop(&fjr_arc)
    }

    pool.terminate_and_join().unwrap();

    sw.stop();
    print_final_status(sw.elapsed_ms());
}

fn load_spec(spec_path: &str) -> Result<(FormatJobRunner, String), padd::BuildError> {
    let mut spec = String::new();

    let spec_file = File::open(spec_path);
    match spec_file {
        Ok(_) => {
            if let Err(err) = spec_file.unwrap().read_to_string(&mut spec) {
                logger::fatal(format!(
                    "Could not read specification file \"{}\": {}", &spec_path, err
                ));
            }
        }
        Err(err) => logger::fatal(format!(
            "Could not find specification file \"{}\": {}", &spec_path, err
        )),
    }

    let fjr = FormatJobRunner::build(&spec)?;

    let mut sha = Sha256::new();
    sha.input_str(&spec[..]);

    Ok((fjr, sha.result_str().to_string()))
}

fn format_target(
    target_path: &Path,
    criteria: &TargetSearchCriteria,
) {
    let path_string = target_path.to_string_lossy().to_string();
    let file_name = target_path.file_name().unwrap().to_str().unwrap();
    if target_path.is_dir() {
        if target_path.ends_with(TRACKER_DIR) {
            return; // Don't format tracker files
        }

        fs::read_dir(target_path).unwrap()
            .for_each(|res| {
                match res {
                    Ok(dir_item) => format_target(&dir_item.path(), criteria),
                    Err(err) => logger::err(format!(
                        "An error occurred while searching directory {}: {}", path_string, err
                    )),
                }
            });
    } else if criteria.fn_regex.is_match(file_name) {
        TOTAL.fetch_add(1, Ordering::SeqCst);

        if criteria.no_skip || needs_formatting(target_path, criteria.spec_sha) {
            criteria.pool.enqueue(FormatPayload {
                fjr_arc: criteria.fjr_arc.clone(),
                file_path: PathBuf::from(target_path),
                spec_sha: criteria.spec_sha.clone(),
                no_track: criteria.no_track,
                no_write: criteria.no_write,
            }).unwrap();
        }
    }
}

fn track_file(file_path: &Path, spec_sha: &str) {
    let tracker_path_buf = tracker_for(file_path);
    let tracker_path = tracker_path_buf.as_path();
    let tracker_path_string = tracker_path.to_string_lossy().to_string();

    let tracker_dir_path = tracker_path.parent().unwrap();
    if !tracker_dir_path.exists() {
        if let Err(err) = fs::create_dir(tracker_dir_path) {
            logger::err(format!(
                "Failed to create tracker directory for {}: {}", tracker_path_string, err
            ))
        }
    }

    match File::create(tracker_path) {
        Err(err) => logger::err(format!(
            "Failed to create tracker file {}: {}", tracker_path_string, err
        )),
        Ok(mut tracker_file) => {
            let since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            let elapsed_millis = since_epoch.as_secs() * 1000 +
                u64::from(since_epoch.subsec_nanos()) / 1_000_000;
            let line = format!("{}\n{}\n", spec_sha, elapsed_millis);

            if let Err(err) = tracker_file.write_all(line.as_bytes()) {
                logger::err(format!(
                    "Failed to write to tracker file {}: {}", tracker_path_string, err
                ))
            }
        }
    }
}

fn needs_formatting(file_path: &Path, spec_sha: &str) -> bool {
    if let Some(formatted_at) = formatted_at(file_path, spec_sha) {
        if let Some(modified_at) = modified_at(file_path) {
            let formatted_dur = formatted_at.duration_since(UNIX_EPOCH).unwrap();
            let modified_dur = modified_at.duration_since(UNIX_EPOCH).unwrap();

            if modified_dur.cmp(&formatted_dur) != cmp::Ordering::Greater {
                return false;
            }
        }
    }

    true
}

fn modified_at(file_path: &Path) -> Option<SystemTime> {
    let path_string = file_path.to_string_lossy().to_string();

    match fs::metadata(file_path) {
        Err(err) => logger::err(format!(
            "Failed to read metadata for {}: {}", path_string, err
        )),
        Ok(metadata) => match metadata.modified() {
            Err(err) => logger::err(format!(
                "Failed to read modified for {}: {}", path_string, err
            )),
            Ok(modified_at) => return Some(modified_at)
        }
    }

    None
}

fn formatted_at(file_path: &Path, spec_sha: &str) -> Option<SystemTime> {
    let tracker_path_buf = tracker_for(file_path);
    let tracker_path = tracker_path_buf.as_path();
    let tracker_path_string = tracker_path.to_string_lossy().to_string();

    if tracker_path.exists() {
        match File::open(tracker_path) {
            Err(err) => logger::err(format!(
                "Failed to open tracker file {}: {}", tracker_path_string, err
            )),
            Ok(tracker_file) => {
                let mut tracker_reader = BufReader::new(tracker_file);

                match read_tracker_line(&mut tracker_reader) {
                    Err(err) => logger::err(format!(
                        "Tracker missing spec sha {}: {}", tracker_path_string, err
                    )),
                    Ok(tracked_spec_sha) => if tracked_spec_sha == *spec_sha {
                        match read_tracker_line(&mut tracker_reader) {
                            Err(err) => logger::err(format!(
                                "Tracker missing timestamp {}: {}", tracker_path_string, err
                            )),
                            Ok(timestamp) => match u64::from_str(&timestamp[..]) {
                                Err(err) => logger::err(format!(
                                    "Failed to parse tracker timestamp {}: {}",
                                    tracker_path_string,
                                    err
                                )),
                                Ok(millis) => return Some(
                                    UNIX_EPOCH + Duration::from_millis(millis)
                                )
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

fn read_tracker_line(reader: &mut BufReader<File>) -> io::Result<String> {
    let mut line = String::new();
    reader.read_line(&mut line)?;

    let line_len = line.len();
    line.truncate(line_len - 1);
    Ok(line)
}

fn tracker_for(file_path: &Path) -> PathBuf {
    let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();
    let mut tracker_dir_buf = file_path.parent().unwrap().to_path_buf();
    tracker_dir_buf.push(TRACKER_DIR);
    tracker_dir_buf.push(format!("{}{}", file_name, TRACKER_EXTENSION));
    tracker_dir_buf
}

fn term_loop(fjr_arc: &Arc<FormatJobRunner>) {
    loop {
        let mut target_path = String::new();

        if let Err(err) = io::stdin().read_line(&mut target_path) {
            logger::fmt_err(format!("Failed to read target file \"{}\": {}", target_path, err));
            continue;
        }

        target_path.pop();

        format_file(&Path::new(&target_path), &fjr_arc, false);
    }
}

fn format_file(target_path: &Path, fjr: &FormatJobRunner, no_write: bool) {
    if format_file_internal(target_path, fjr, no_write) {
        FORMATTED.fetch_add(1, Ordering::SeqCst);
    } else {
        FAILED.fetch_add(1, Ordering::SeqCst);
    }
}

fn format_file_internal(target_path: &Path, fjr: &FormatJobRunner, no_write: bool) -> bool {
    logger::fmt(target_path.to_string_lossy().to_string());
    let target_file = OpenOptions::new().read(true).write(true).open(&target_path);
    let target_path_string = target_path.to_string_lossy().to_string();
    match target_file {
        Ok(_) => {
            let mut target = target_file.unwrap();

            let result = {
                let mut text = String::new();

                if let Err(err) = target.read_to_string(&mut text) {
                    logger::fatal(format!(
                        "Could not read target file \"{}\": {}", target_path_string, err
                    ));
                }

                fjr.format(FormatJob::from_text(text))
            };

            match result {
                Ok(res) => {
                    if no_write {
                        logger::fmt_ok(target_path_string);
                        return true;
                    }

                    if let Err(err) = target.seek(SeekFrom::Start(0)) {
                        logger::fmt_err(format!(
                            "Could not seek to start of target file \"{}\": {}",
                            target_path_string,
                            err
                        ));
                        return false;
                    }
                    if let Err(err) = target.set_len(0) {
                        logger::fmt_err(format!(
                            "Could not clear target file \"{}\": {}", target_path_string, err
                        ));
                        return false;
                    }
                    match target.write_all(res.as_bytes()) {
                        Ok(_) => logger::fmt_ok(target_path_string),
                        Err(err) => {
                            logger::fmt_err(format!(
                                "Could not write to target file \"{}\": {}", target_path_string, err
                            ));
                            return false;
                        }
                    }
                }
                Err(err) => {
                    logger::fmt_err(format!("Error formatting {}: {}", target_path_string, err));
                    return false;
                }
            }
        }
        Err(err) => {
            logger::fmt_err(format!(
                "Could not find target file \"{}\": {}", target_path_string, err
            ));
            return false;
        }
    }
    true
}

fn print_final_status(elapsed_ms: i64) {
    let total = TOTAL.load(Ordering::Relaxed);
    let formatted = FORMATTED.load(Ordering::Relaxed);
    let failed = FAILED.load(Ordering::Relaxed);
    let unchanged = total - failed - formatted;

    let mut unchanged_msg = format!("{} unchanged", unchanged).normal();
    if unchanged > 0 {
        unchanged_msg = unchanged_msg.yellow()
    }

    let mut formatted_msg: ColoredString = format!("{} formatted", formatted).normal();
    if formatted > 0 {
        formatted_msg = formatted_msg.bright_green()
    }

    let mut failed_msg = format!("{} failed", failed).normal();
    if failed > 0 {
        failed_msg = failed_msg.bright_red()
    }

    println!();
    logger::info(format!(
        "COMPLETE: {}ms : {} processed, {}, {}, {}",
        elapsed_ms,
        total,
        unchanged_msg,
        formatted_msg,
        failed_msg
    ));
}

struct FormatPayload {
    fjr_arc: Arc<FormatJobRunner>,
    file_path: PathBuf,
    spec_sha: String,
    no_track: bool,
    no_write: bool,
}

struct TargetSearchCriteria<'outer> {
    fn_regex: &'outer Regex,
    spec_sha: &'outer String,
    no_skip: bool,
    no_track: bool,
    no_write: bool,
    fjr_arc: &'outer Arc<FormatJobRunner>,
    pool: &'outer ThreadPool<FormatPayload>,
}

fn forget(matches: &ArgMatches) {
    let target: &Path = match matches.value_of("target") {
        None => panic!("No target path specified"),
        Some(file) => Path::new(file)
    };

    println!("Clearing all tracking data from {} ...", target.to_string_lossy().to_string());

    clear_tracking(target);

    let total = TOTAL.load(Ordering::Relaxed);
    match total {
        1 => println!("Removed 1 tracking directory"),
        _ => println!("Removed {} tracking directories", total)
    }
}

fn clear_tracking(target_path: &Path) {
    let path_string = target_path.to_string_lossy().to_string();
    if target_path.is_dir() {
        if target_path.ends_with(TRACKER_DIR) {
            if let Err(err) = fs::remove_dir_all(target_path) {
                logger::err(format!(
                    "Could not delete tracking directory {}: {}", path_string, err
                ))
            }
            TOTAL.fetch_add(1, Ordering::SeqCst);
            return;
        }

        fs::read_dir(target_path).unwrap()
            .for_each(|res| {
                match res {
                    Ok(dir_item) => clear_tracking(&dir_item.path()),
                    Err(err) => logger::err(format!(
                        "An error occurred while searching directory {}: {}", path_string, err
                    )),
                }
            });
    }
}
