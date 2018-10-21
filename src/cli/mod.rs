extern crate regex;
extern crate stopwatch;
extern crate clap;

use self::regex::Regex;
use self::stopwatch::Stopwatch;
use self::clap::{Arg, ArgGroup, ArgMatches, App};

use std::io::{self, Read, Write, Seek, SeekFrom, BufRead, BufReader};
use std::process;
use std::fs::{self, File, OpenOptions};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};

use padd::{self, FormatJobRunner, Stream};

static FORMATTED: AtomicUsize = ATOMIC_USIZE_INIT;
static TOTAL: AtomicUsize = ATOMIC_USIZE_INIT;

pub fn run() {
    let matches = build_app();

    let spec_path = matches.value_of("spec").unwrap();
    let fjr = match load_spec(&spec_path) {
        Err(err) => {
            error(format!("Error loading specification {}: {}", &spec_path, err));
            return;
        }
        Ok(fjr) => fjr
    };

    println!("Successfully loaded specification");

    let directory: Option<&Path> = match matches.value_of("directory") {
        None => None,
        Some(dir) => Some(Path::new(dir))
    };

    let file_regex: Option<Regex> = match matches.value_of("matching") {
        None => None,
        Some(regex) => match Regex::new(format!(r#"{}"#, regex).as_str()) {
            Ok(fn_regex) => Some(fn_regex),
            Err(e) => {
                error(format!("Failed to build file name regex: {}", e));
                None
            }
        }
    };

    let target: Option<&Path> = match matches.value_of("target") {
        None => None,
        Some(file) => Some(Path::new(file))
    };

    let mut sw = Stopwatch::new();
    sw.start();

    match target {
        Some(target_path) => {
            if directory.is_some() {
                panic!("Target file and directory both specified");
            } else if file_regex.is_some() {
                panic!("Target file and file regex both specified");
            }
            format_file(target_path, &fjr)
        }
        None => match directory {
            Some(dir_path) => {
                let fn_regex = match file_regex {
                    Some(regex) => regex,
                    None => Regex::new(r#".*"#).unwrap(),
                };

                dir_recur(dir_path, &fn_regex, &fjr)
            }
            None => term_loop(&fjr),
        }
    }

    sw.stop();
    let total = TOTAL.load(Ordering::Relaxed);
    let formatted = FORMATTED.load(Ordering::Relaxed);
    println!("COMPLETE: {}ms : {} processed, {} formatted, {} failed", sw.elapsed_ms(), total, formatted, total - formatted);
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
            .help("Sets a single file to format")
            .takes_value(true)
            .value_name("FILE")
        )
        .arg(Arg::with_name("directory")
            .short("d")
            .long("directory")
            .help("Sets the directory to format files under")
            .takes_value(true)
            .value_name("DIRECTORY")
        )
        .arg(Arg::with_name("matching")
            .short("m")
            .long("matching")
            .help("Sets the regex for file names to format")
            .takes_value(true)
            .value_name("REGEX")
            .requires_all(&["directory"])
        )
        .group(ArgGroup::with_name("input")
            .args(&["target", "directory"])
            .required(true)
        )
        .get_matches()
}

fn dir_recur(dir_path: &Path, fn_regex: &Regex, fjr: &FormatJobRunner) {
    fs::read_dir(dir_path).unwrap()
        .for_each(|res| {
            match res {
                Ok(dir_item) => {
                    let path = dir_item.path();
                    let file_name = path.file_name().unwrap().to_str().unwrap();
                    if path.is_dir() {
                        dir_recur(path.as_path(), fn_regex, fjr);
                    } else if fn_regex.is_match(file_name) {
                        format_file(path.as_path(), &fjr);
                    }
                }
                Err(e) => println!("An error occurred while searching directory {}: {}", dir_path.to_string_lossy(), e),
            }
        });
}

fn load_spec(spec_path: &str) -> Result<FormatJobRunner, padd::BuildError> {
    let mut spec = String::new();

    let spec_file = File::open(spec_path);
    match spec_file {
        Ok(_) => {
            match spec_file.unwrap().read_to_string(&mut spec) {
                Ok(_) => {}
                Err(e) => {
                    error(format!("Could not read specification file \"{}\": {}", &spec_path, e));
                }
            }
        }
        Err(e) => error(format!("Could not find specification file \"{}\": {}", &spec_path, e)),
    }

    FormatJobRunner::build(&spec)
}

fn term_loop(fjr: &FormatJobRunner) {
    loop {
        let mut target_path = String::new();

        match io::stdin().read_line(&mut target_path) {
            Ok(_) => {}
            Err(e) => {
                println!("Failed to read target file \"{}\": {}", target_path, e);
                continue;
            }
        }

        target_path.pop();

        format_file(&Path::new(&target_path), &fjr);
    }
}

fn format_file(target_path: &Path, fjr: &FormatJobRunner) {
    TOTAL.fetch_add(1, Ordering::SeqCst);
    if format_file_internal(target_path, fjr) {
        FORMATTED.fetch_add(1, Ordering::SeqCst);
    }
}

fn format_file_internal(target_path: &Path, fjr: &FormatJobRunner) -> bool {
    print!(">> Formatting {}: ", target_path.to_string_lossy());
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
                            println!("Could not read target file \"{}\": {}", &target_path.to_string_lossy(), e);
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
                            println!("Could not seek to start of target file \"{}\": {}", &target_path.to_string_lossy(), e);
                            return false;
                        }
                    }
                    match target.set_len(0) {
                        Ok(_) => {}
                        Err(e) => {
                            println!("Could not clear target file \"{}\": {}", &target_path.to_string_lossy(), e);
                            return false;
                        }
                    }
                    match target.write_all(res.as_bytes()) {
                        Ok(_) => { println!("OK") }
                        Err(e) => {
                            println!("Could not write to target file \"{}\": {}", &target_path.to_string_lossy(), e);
                            return false;
                        }
                    }
                }
                Err(e) => {
                    println!("Error formatting {}: {}", &target_path.to_string_lossy(), e);
                    return false;
                }
            }
        }
        Err(e) => {
            println!("Could not find target file \"{}\": {}", &target_path.to_string_lossy(), e);
            return false;
        }
    }
    true
}

fn error(err_text: String) {
    println!("ERROR: {}", err_text);
    process::exit(0);
}
