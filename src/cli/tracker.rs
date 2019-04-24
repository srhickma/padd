extern crate clap;
extern crate colored;
extern crate crypto;
extern crate regex;
extern crate stopwatch;

use {
    cli::logger,
    std::{
        cmp,
        fs::{self, File},
        io::{self, BufRead, BufReader, Write},
        path::{Path, PathBuf},
        str::FromStr,
        time::{Duration, SystemTime, UNIX_EPOCH},
    },
};

pub const TRACKER_DIR: &str = ".padd";
const TRACKER_EXTENSION: &str = ".trk";

pub fn track_file(file_path: &Path, spec_sha: &str) {
    let tracker_path_buf = tracker_for(file_path);
    let tracker_path = tracker_path_buf.as_path();
    let tracker_path_string = tracker_path.to_string_lossy().to_string();

    let tracker_dir_path = tracker_path.parent().unwrap();
    if !tracker_dir_path.exists() {
        if let Err(err) = fs::create_dir(tracker_dir_path) {
            logger::err(&format!(
                "Failed to create tracker directory for {}: {}",
                tracker_path_string, err
            ))
        }
    }

    match File::create(tracker_path) {
        Err(err) => logger::err(&format!(
            "Failed to create tracker file {}: {}",
            tracker_path_string, err
        )),
        Ok(mut tracker_file) => {
            let since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            let elapsed_millis =
                since_epoch.as_secs() * 1000 + u64::from(since_epoch.subsec_nanos()) / 1_000_000;
            let line = format!("{}\n{}\n", spec_sha, elapsed_millis);

            if let Err(err) = tracker_file.write_all(line.as_bytes()) {
                logger::err(&format!(
                    "Failed to write to tracker file {}: {}",
                    tracker_path_string, err
                ))
            }
        }
    }
}

pub fn needs_formatting(file_path: &Path, spec_sha: &str) -> bool {
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
        Err(err) => logger::err(&format!(
            "Failed to read metadata for {}: {}",
            path_string, err
        )),
        Ok(metadata) => match metadata.modified() {
            Err(err) => logger::err(&format!(
                "Failed to read modified for {}: {}",
                path_string, err
            )),
            Ok(modified_at) => return Some(modified_at),
        },
    }

    None
}

fn formatted_at(file_path: &Path, spec_sha: &str) -> Option<SystemTime> {
    let tracker_path_buf = tracker_for(file_path);
    let tracker_path = tracker_path_buf.as_path();
    let tracker_path_string = tracker_path.to_string_lossy().to_string();

    if tracker_path.exists() {
        match File::open(tracker_path) {
            Err(err) => logger::err(&format!(
                "Failed to open tracker file {}: {}",
                tracker_path_string, err
            )),
            Ok(tracker_file) => {
                let mut tracker_reader = BufReader::new(tracker_file);

                match read_tracker_line(&mut tracker_reader) {
                    Err(err) => logger::err(&format!(
                        "Tracker missing spec sha {}: {}",
                        tracker_path_string, err
                    )),
                    Ok(tracked_spec_sha) => {
                        if tracked_spec_sha == *spec_sha {
                            match read_tracker_line(&mut tracker_reader) {
                                Err(err) => logger::err(&format!(
                                    "Tracker missing timestamp {}: {}",
                                    tracker_path_string, err
                                )),
                                Ok(timestamp) => match u64::from_str(&timestamp[..]) {
                                    Err(err) => logger::err(&format!(
                                        "Failed to parse tracker timestamp {}: {}",
                                        tracker_path_string, err
                                    )),
                                    Ok(millis) => {
                                        return Some(UNIX_EPOCH + Duration::from_millis(millis))
                                    }
                                },
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

pub fn clear_tracking(target_path: &Path) {
    println!(
        "Clearing all tracking data from {} ...",
        target_path.to_string_lossy().to_string()
    );

    let cleared = clear_directory_tracking(target_path);
    match cleared {
        1 => println!("Removed 1 tracking directory"),
        _ => println!("Removed {} tracking directories", cleared),
    };
}

fn clear_directory_tracking(target_path: &Path) -> usize {
    let mut cleared: usize = 0;

    let path_string = target_path.to_string_lossy().to_string();
    if target_path.is_dir() {
        if target_path.ends_with(TRACKER_DIR) {
            if let Err(err) = fs::remove_dir_all(target_path) {
                logger::err(&format!(
                    "Could not delete tracking directory {}: {}",
                    path_string, err
                ))
            }
            cleared += 1;
        } else {
            fs::read_dir(target_path)
                .unwrap()
                .for_each(|res| match res {
                    Ok(dir_item) => cleared += clear_directory_tracking(&dir_item.path()),
                    Err(err) => logger::err(&format!(
                        "An error occurred while searching directory {}: {}",
                        path_string, err
                    )),
                });
        }
    }

    cleared
}
