extern crate clap;
extern crate colored;
extern crate log;
extern crate log4rs;
extern crate strip_ansi_escapes;

use std::{
    error::Error,
    io::{self, Cursor, Read, Seek, SeekFrom, Write},
    process,
};

use self::{
    clap::ArgMatches,
    colored::{ColoredString, Colorize},
    log::{LevelFilter, Record},
    log4rs::{
        append::file::FileAppender,
        config::{Appender, Config, Root},
        encode::{pattern::PatternEncoder, Encode, Write as LogWrite},
    },
};

static DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::Info;

lazy_static! {
    static ref PREFIX_ERR: ColoredString = "error".bright_red();
    static ref PREFIX_FATAL: ColoredString = "fatal".on_bright_red();
    static ref PREFIX_FMT: ColoredString = "  FMT".bright_blue();
    static ref PREFIX_FMT_OK: ColoredString = "   OK".bright_green();
    static ref PREFIX_FMT_ERR: ColoredString = "ERROR".bright_red();
}

pub fn init(matches: &ArgMatches) {
    if let Some(log_file) = matches.value_of("logfile") {
        let log_level = match matches.value_of("loglevel") {
            Some("error") => LevelFilter::Error,
            Some("warn") => LevelFilter::Warn,
            Some("info") => LevelFilter::Info,
            Some("debug") => LevelFilter::Debug,
            Some("trace") => LevelFilter::Trace,
            _ => DEFAULT_LOG_LEVEL,
        };

        let pattern_encoder = PatternEncoder::new("{d(%Y-%m-%d %H:%M:%S)} {l} - {m}{n}");
        let sanitized_encoder = SanitizedEncoder::wrap(pattern_encoder);

        let file_appender_res = FileAppender::builder()
            .encoder(Box::new(sanitized_encoder))
            .build(log_file);

        let file_appender = match file_appender_res {
            Ok(file_appender) => file_appender,
            Err(err) => {
                self::err(&format!("Failed to build log file appender: {}", err));
                return;
            }
        };

        let config_res = Config::builder()
            .appender(Appender::builder().build("file", Box::new(file_appender)))
            .build(Root::builder().appender("file").build(log_level));

        let config = match config_res {
            Ok(config) => config,
            Err(err) => {
                self::err(&format!("Failed to build logger configuration: {}", err));
                return;
            }
        };

        if let Err(err) = log4rs::init_config(config) {
            self::err(&format!("Failed to initialize logger: {}", err));
        }
    }

    info!(
        "
                _    _
  _ __  __ _ __| |__| |
 | '_ \\/ _` / _` / _` |
 | .__/\\__,_\\__,_\\__,_|
 |_|
                     "
    );
}

pub fn info(string: &str) {
    println!("{}", string);
    info!("{}", string);
}

pub fn err(string: &str) {
    println!("{}: {}", *PREFIX_ERR, string);
    error!("{}", string);
}

pub fn fatal(string: &str) {
    println!("{}: {}", *PREFIX_FATAL, string);
    error!("{}", string);
    process::exit(1);
}

pub fn fmt(string: &str) {
    println!("{}| {}", *PREFIX_FMT, string);
    debug!("Formatting {} ...", string);
}

pub fn fmt_ok(string: &str) {
    println!("{}| {}", *PREFIX_FMT_OK, string);
    debug!("Finished formatting {}", string);
}

pub fn fmt_err(string: &str) {
    println!("{}| {}", *PREFIX_FMT_ERR, string);
    warn!("{}", string);
}

#[derive(Debug)]
struct SanitizedEncoder {
    encoder: Box<Encode>,
}

impl SanitizedEncoder {
    fn wrap(encoder: impl Encode) -> Self {
        SanitizedEncoder {
            encoder: Box::new(encoder),
        }
    }
}

impl Encode for SanitizedEncoder {
    fn encode(
        &self,
        w: &mut LogWrite,
        record: &Record,
    ) -> Result<(), Box<dyn Error + Sync + Send>> {
        let mut writer = SanitizedLogWriter::new();
        self.encoder.encode(&mut writer, record)?;
        writer.sanitize_write(w)?;
        Ok(())
    }
}

struct SanitizedLogWriter {
    cursor: Cursor<Vec<u8>>,
}

impl SanitizedLogWriter {
    fn new() -> Self {
        SanitizedLogWriter {
            cursor: Cursor::new(Vec::new()),
        }
    }

    fn sanitize_write(&mut self, w: &mut LogWrite) -> Result<usize, std::io::Error> {
        let mut buf = Vec::new();
        self.cursor.seek(SeekFrom::Start(0))?;
        self.cursor.read_to_end(&mut buf)?;

        let sanitized = strip_ansi_escapes::strip(buf)?;
        w.write(&sanitized)
    }
}

impl Write for SanitizedLogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.cursor.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.cursor.flush()
    }
}

impl LogWrite for SanitizedLogWriter {}
