extern crate crypto;
extern crate regex;

use {
    cli::{
        logger,
        thread_pool::ThreadPool,
        tracker::{self, TRACKER_DIR},
    },
    padd::{self, FormatJob, FormatJobRunner},
    std::{
        collections::HashMap,
        error, fmt,
        fs::{self, File, OpenOptions},
        io::{Read, Seek, SeekFrom, Write},
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
    },
};

use self::{
    crypto::{digest::Digest, sha2::Sha256},
    regex::Regex,
};

const THREAD_POOL_QUEUE_LENGTH_PER_WORKER: usize = 2;

lazy_static! {
    static ref FJR_CACHE: Mutex<HashMap<String, Arc<FormatJobRunner>>> = Mutex::new(HashMap::new());
}

#[derive(Clone)]
pub struct Formatter {
    fjr_arc: Arc<FormatJobRunner>,
    spec_sha: String,
}

pub struct FormatCommand<'path> {
    pub formatter: Formatter,
    pub target_path: &'path Path,
    pub file_regex: Option<Regex>,
    pub thread_count: usize,
    pub no_skip: bool,
    pub no_track: bool,
    pub no_write: bool,
    pub check: bool,
}

struct FormatInstance<'outer> {
    formatter: &'outer Formatter,
    pool: &'outer ThreadPool<FormatPayload>,
    criteria: FormatCriteria<'outer>,
    metrics: Arc<Mutex<FormatMetrics>>,
}

struct FormatCriteria<'outer> {
    fn_regex: &'outer Regex,
    no_skip: bool,
    no_track: bool,
    no_write: bool,
    check: bool,
}

pub struct FormatMetrics {
    pub formatted: usize,
    pub failed: usize,
    pub total: usize,
}

impl FormatMetrics {
    fn new() -> Self {
        FormatMetrics {
            formatted: 0,
            failed: 0,
            total: 0,
        }
    }

    fn copy(&self) -> Self {
        FormatMetrics {
            formatted: self.formatted,
            failed: self.failed,
            total: self.total,
        }
    }

    fn inc_formatted(&mut self) {
        self.formatted += 1;
    }

    fn inc_failed(&mut self) {
        self.failed += 1;
    }

    fn inc_total(&mut self) {
        self.total += 1;
    }
}

struct FormatPayload {
    formatter: Formatter,
    file_path: PathBuf,
    no_track: bool,
    no_write: bool,
    check: bool,
    metrics: Arc<Mutex<FormatMetrics>>,
}

impl FormatPayload {
    fn from(path: &Path, instance: &FormatInstance) -> Self {
        FormatPayload {
            file_path: PathBuf::from(path),
            formatter: instance.formatter.clone(),
            no_track: instance.criteria.no_track,
            no_write: instance.criteria.no_write,
            check: instance.criteria.check,
            metrics: instance.metrics.clone(),
        }
    }
}

pub fn generate_formatter(spec_path: &str) -> Result<Formatter, GenerationError> {
    logger::info(&format!("Loading specification {} ...", spec_path));

    let mut spec = String::new();

    match File::open(spec_path) {
        Ok(mut spec_file) => {
            if let Err(err) = spec_file.read_to_string(&mut spec) {
                return Err(GenerationError::FileErr(format!(
                    "Could not read specification file \"{}\": {}",
                    &spec_path, err
                )));
            }
        }
        Err(err) => {
            return Err(GenerationError::FileErr(format!(
                "Could not find specification file \"{}\": {}",
                &spec_path, err
            )))
        }
    }

    let mut sha = Sha256::new();
    sha.input_str(&spec[..]);
    let spec_sha = sha.result_str().to_string();

    let fjr_arc = {
        let mut fjr_cache = FJR_CACHE.lock().unwrap();

        if fjr_cache.contains_key(&spec_sha) {
            logger::info(&format!(
                "Loading cached specification: sha256: {}",
                &spec_sha
            ));
        }

        #[allow(clippy::or_fun_call)]
        fjr_cache
            .entry(spec_sha.clone())
            .or_insert(Arc::new(FormatJobRunner::build(&spec)?))
            .clone()
    };

    logger::info(&format!(
        "Successfully loaded specification: sha256: {}",
        &spec_sha
    ));

    Ok(Formatter { fjr_arc, spec_sha })
}

#[derive(Debug)]
pub enum GenerationError {
    FileErr(String),
    BuildErr(padd::BuildError),
}

impl fmt::Display for GenerationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            GenerationError::FileErr(ref err) => write!(f, "{}", err),
            GenerationError::BuildErr(ref err) => write!(f, "{}", err),
        }
    }
}

impl error::Error for GenerationError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            GenerationError::FileErr(_) => None,
            GenerationError::BuildErr(ref err) => Some(err),
        }
    }
}

impl From<padd::BuildError> for GenerationError {
    fn from(err: padd::BuildError) -> GenerationError {
        GenerationError::BuildErr(err)
    }
}

pub fn format(cmd: FormatCommand) -> FormatMetrics {
    let pool: ThreadPool<FormatPayload> = ThreadPool::spawn(
        cmd.thread_count,
        cmd.thread_count * THREAD_POOL_QUEUE_LENGTH_PER_WORKER,
        |payload: FormatPayload| {
            let file_path = payload.file_path.as_path();
            let file_path_string = file_path.to_string_lossy().to_string();

            logger::fmt(&file_path_string);

            let result = if payload.check {
                check_file(file_path, &payload.formatter.fjr_arc)
            } else {
                format_file(file_path, &payload.formatter.fjr_arc, payload.no_write)
            };

            match result {
                Ok(_) => {
                    logger::fmt_ok(&file_path_string);
                    payload.metrics.lock().unwrap().inc_formatted();
                }
                Err(err) => {
                    if payload.check {
                        logger::fmt_check_err(&format!("{}", err));
                    } else {
                        logger::fmt_err(&format!("{}", err));
                    }
                    payload.metrics.lock().unwrap().inc_failed();
                }
            }

            if !payload.no_track {
                tracker::track_file(file_path, &payload.formatter.spec_sha);
            }
        },
    );

    let fn_regex = match cmd.file_regex {
        Some(regex) => regex,
        None => Regex::new(r#".*"#).unwrap(),
    };

    let mut instance = FormatInstance {
        formatter: &cmd.formatter,
        pool: &pool,
        criteria: FormatCriteria {
            fn_regex: &fn_regex,
            no_skip: cmd.no_skip,
            no_track: cmd.no_track,
            no_write: cmd.no_write,
            check: cmd.check,
        },
        metrics: Arc::new(Mutex::new(FormatMetrics::new())),
    };

    format_target(&cmd.target_path, &mut instance);

    pool.terminate_and_join().unwrap();

    let metrics = instance.metrics.lock().unwrap();
    metrics.copy()
}

fn format_target(target_path: &Path, instance: &mut FormatInstance) {
    let path_string = target_path.to_string_lossy().to_string();
    let file_name = target_path.file_name().unwrap().to_str().unwrap();
    if target_path.is_dir() {
        if target_path.ends_with(TRACKER_DIR) {
            return; // Don't format tracker files
        }

        fs::read_dir(target_path)
            .unwrap()
            .for_each(|res| match res {
                Ok(dir_item) => format_target(&dir_item.path(), instance),
                Err(err) => logger::err(&format!(
                    "An error occurred while searching directory {}: {}",
                    path_string, err
                )),
            });
    } else if instance.criteria.fn_regex.is_match(file_name) {
        instance.metrics.lock().unwrap().inc_total();

        if instance.criteria.no_skip
            || instance.criteria.check
            || tracker::needs_formatting(target_path, &instance.formatter.spec_sha)
        {
            let payload = FormatPayload::from(target_path, instance);
            instance.pool.enqueue(payload).unwrap();
        }
    }
}

fn format_file(
    target_path: &Path,
    fjr: &FormatJobRunner,
    no_write: bool,
) -> Result<(), FormattingError> {
    let target_file = OpenOptions::new().read(true).write(true).open(&target_path);
    let target_path_string = target_path.to_string_lossy().to_string();
    match target_file {
        Ok(_) => {
            let mut target = target_file.unwrap();

            let result = {
                let mut text = String::new();

                if let Err(err) = target.read_to_string(&mut text) {
                    return Err(FormattingError::FileErr(format!(
                        "Could not read target file \"{}\": {}",
                        target_path_string, err
                    )));
                }

                fjr.format(FormatJob::from_text(text))
            };

            match result {
                Ok(res) => {
                    if no_write {
                        return Ok(());
                    }

                    if let Err(err) = target.seek(SeekFrom::Start(0)) {
                        return Err(FormattingError::FileErr(format!(
                            "Could not seek to start of target file \"{}\": {}",
                            target_path_string, err
                        )));
                    }
                    if let Err(err) = target.set_len(0) {
                        return Err(FormattingError::FileErr(format!(
                            "Could not clear target file \"{}\": {}",
                            target_path_string, err
                        )));
                    }

                    match target.write_all(res.as_bytes()) {
                        Ok(_) => Ok(()),
                        Err(err) => Err(FormattingError::FileErr(format!(
                            "Could not write to target file \"{}\": {}",
                            target_path_string, err
                        ))),
                    }
                }
                Err(err) => Err(FormattingError::FormatErr(err, target_path_string)),
            }
        }
        Err(err) => Err(FormattingError::FileErr(format!(
            "Could not find target file \"{}\": {}",
            target_path_string, err
        ))),
    }
}

fn check_file(target_path: &Path, fjr: &FormatJobRunner) -> Result<(), FormattingError> {
    let target_file = OpenOptions::new().read(true).write(true).open(&target_path);
    let target_path_string = target_path.to_string_lossy().to_string();
    match target_file {
        Ok(target_file) => {
            let mut target = target_file;

            let mut text = String::new();

            if let Err(err) = target.read_to_string(&mut text) {
                return Err(FormattingError::FileErr(format!(
                    "Could not read target file \"{}\": {}",
                    target_path_string, err
                )));
            }

            let result = fjr.format(FormatJob::from_text(text.clone()));

            match result {
                Ok(res) => {
                    if res == text {
                        Ok(())
                    } else {
                        Err(FormattingError::CheckErr(target_path_string))
                    }
                }
                Err(err) => Err(FormattingError::FormatErr(err, target_path_string)),
            }
        }
        Err(err) => Err(FormattingError::FileErr(format!(
            "Could not find target file \"{}\": {}",
            target_path_string, err
        ))),
    }
}

#[derive(Debug)]
pub enum FormattingError {
    FileErr(String),
    FormatErr(padd::FormatError, String),
    CheckErr(String),
}

impl fmt::Display for FormattingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FormattingError::FileErr(ref err) => write!(f, "{}", err),
            FormattingError::FormatErr(ref err, ref target) => {
                write!(f, "Error formatting {}: {}", target, err)
            }
            FormattingError::CheckErr(ref target) => {
                write!(f, "Formatting check failed for {}", target)
            }
        }
    }
}

impl error::Error for FormattingError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            FormattingError::FileErr(_) => None,
            FormattingError::FormatErr(ref err, _) => Some(err),
            FormattingError::CheckErr(_) => None,
        }
    }
}
