extern crate clap;
extern crate colored;
extern crate log;
extern crate log4rs;
extern crate strip_ansi_escapes;

use std::{
    error::Error,
    fmt,
    io::{self, Cursor, Read, Seek, SeekFrom, Write},
    panic,
    sync::Mutex,
};

use self::{
    clap::ArgMatches,
    colored::{ColoredString, Colorize},
    log::{LevelFilter, Record},
    log4rs::{
        append::file::FileAppender,
        config::{Appender, Config, Root},
        encode::{pattern::PatternEncoder, Encode, Write as LogWrite},
        Handle,
    },
};

static DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::Info;

lazy_static! {
    static ref PREFIX_ERR: ColoredString = "error".bright_red();
    static ref PREFIX_FATAL: ColoredString = "fatal".on_bright_red();
    static ref PREFIX_FMT: ColoredString = "  FMT".bright_blue();
    static ref PREFIX_FMT_OK: ColoredString = "   OK".bright_green();
    static ref PREFIX_FMT_ERR: ColoredString = "ERROR".bright_red();
    static ref LOGGER_HANDLE: Mutex<Option<Handle>> = Mutex::new(None);
}

macro_rules! catch_fatal {
    ($body: block, $catch: block) => {
        panic::set_hook(Box::new(|info| {
            if !info.payload().is::<Fatal>() {
                //#ccstart
                use backtrace::Backtrace;
                let backtrace = Backtrace::new();

                println!("{}", info);
                error!("{}", info);
                println!("{:?}", backtrace);
                error!("{:?}", backtrace);
                println!("Something terrible has happened, please file an issue at https://github.com/srhickma/padd/issues");
                error!("Something terrible has happened, please file an issue at https://github.com/srhickma/padd/issues");
                //#ccstop
            }
        }));

        if let Err(err) = panic::catch_unwind(|| $body) {
            if err.is::<Fatal>() {
                $catch

                #[allow(unreachable_code)] {
                    let _ = panic::take_hook();
                }
            } else {
                panic::resume_unwind(err)
            }
        }
    };
}

#[derive(Debug)]
pub enum Fatal {
    Error,
}

impl fmt::Display for Fatal {
    fn fmt(&self, _: &mut fmt::Formatter) -> fmt::Result {
        Ok(())
    }
}

impl Error for Fatal {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
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
            Err(err) => panic!("Failed to build log file appender: {}", err),
        };

        let config_res = Config::builder()
            .appender(Appender::builder().build("file", Box::new(file_appender)))
            .build(Root::builder().appender("file").build(log_level));

        let config = match config_res {
            Ok(config) => config,
            Err(err) => panic!("Failed to build logger configuration: {}", err),
        };

        let mut handle_opt = LOGGER_HANDLE.lock().unwrap();

        if handle_opt.is_none() {
            match log4rs::init_config(config) {
                Ok(handle) => {
                    *handle_opt = Some(handle);
                }
                Err(err) => panic!("Failed to initialize logger: {}", err),
            }
        } else if let Some(ref handle) = *handle_opt {
            handle.set_config(config);
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
    panic!(Fatal::Error);
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

pub fn fmt_check_err(string: &str) {
    println!("{}| {}", *PREFIX_FMT_ERR, string);
    error!("{}", string);
}

#[derive(Debug)]
struct SanitizedEncoder {
    encoder: Box<dyn Encode>,
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
        w: &mut dyn LogWrite,
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

    fn sanitize_write(&mut self, w: &mut dyn LogWrite) -> Result<usize, std::io::Error> {
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
