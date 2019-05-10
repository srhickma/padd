#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate padd;

use {
    cli::logger::{Fatal},
    std::{env, panic, process}
};

#[macro_use]
mod cli;

fn main() {
    let args: Vec<String> = env::args().collect();

    catch_fatal!({
        cli::run(args.iter().map(|s| &**s).collect());
    }, {
        process::exit(1);
    });
}

#[cfg(test)]
mod tests {
    extern crate log;
    extern crate regex;
    extern crate uuid;

    use super::*;

    use {
        self::{log::LevelFilter, regex::Regex, uuid::Uuid},
        cli::server,
        std::{
            fs::{self, File, OpenOptions},
            io::{prelude::*, Read},
            path::{Path, PathBuf},
            process::Command,
            sync::RwLock,
            thread,
            time::Duration,
        },
    };

    static EXECUTABLE: &'static str = "target/debug/padd";

    lazy_static! {
        static ref TEMP_DIR: &'static Path = Path::new("tests/temp");
        static ref INPUT_DIR: &'static Path = Path::new("tests/input");
        static ref OUTPUT_DIR: &'static Path = Path::new("tests/output");

        static ref LOG_PATH: String = String::from("tests/test.log");

        static ref SERIALIZATION_LOCK: RwLock<()> = RwLock::new(());

        static ref COMPLETION_REGEX: Regex = Regex::new(r"INFO - COMPLETE: \d*ms : (\d*) processed, (\d*) unchanged, (\d*) formatted, (\d*) failed").unwrap();
        static ref FORMATTED_REGEX: Regex = Regex::new(r"DEBUG - Finished formatting ([^\n]*)").unwrap();
        static ref FAILED_REGEX: Regex = Regex::new(r"WARN - Error formatting ([^\n:]*): ([^\n]*)").unwrap();
        static ref CHECK_FAILED_REGEX: Regex = Regex::new(r"ERROR - Formatting check failed for ([^\n:]*)").unwrap();
    }

    macro_rules! serial {
        ($body: block) => {
            let guard = SERIALIZATION_LOCK.write().unwrap();

            if let Err(err) = panic::catch_unwind(|| $body) {
                drop(guard);
                panic::resume_unwind(err);
            }
        };
    }

    macro_rules! parallel {
        ($body: block) => {
            let guard = SERIALIZATION_LOCK.read().unwrap();

            if let Err(err) = panic::catch_unwind(|| $body) {
                drop(guard);
                panic::resume_unwind(err);
            }
        };
    }

    struct TestableFile<'scope> {
        file_name: String,
        temp_dir: &'scope Path,
    }

    impl<'scope> TestableFile<'scope> {
        fn new(file_name: String, temp_dir: &'scope str) -> Self {
            TestableFile {
                file_name,
                temp_dir: Path::new(temp_dir),
            }
        }

        fn copy_to_temp(&self) -> String {
            let input_path = path_from_name(&INPUT_DIR, &self.file_name);
            let temp_path = path_from_name(self.temp_dir, &self.file_name);

            fs::copy(input_path, &temp_path).unwrap();

            temp_path.as_path().to_string_lossy().to_string()
        }

        fn assert_matches_output(&self) {
            let output_path = path_from_name(&OUTPUT_DIR, &self.file_name);
            let temp_path = path_from_name(self.temp_dir, &self.file_name);

            let expected = read_to_string(output_path.as_path());
            let actual = read_to_string(temp_path.as_path());
            if expected != actual {
                println!("EXPECTED:\n{}\nBUT FOUND:\n{}", expected, actual);
                panic!("Temp file did not match output file")
            }
        }
    }

    #[derive(PartialEq)]
    struct FormattedFJ {
        file_name: String,
    }

    #[derive(PartialEq)]
    struct FailedFJ {
        file_name: String,
        error_message: String,
    }

    #[derive(PartialEq)]
    struct CheckFailedFJ {
        file_name: String,
    }

    struct LoggedResults {
        num_processed: usize,
        num_unchanged: usize,
        num_formatted: usize,
        num_failed: usize,
        formatted: Vec<FormattedFJ>,
        failed: Vec<FailedFJ>,
        check_failed: Vec<CheckFailedFJ>,
    }

    impl LoggedResults {
        fn parse(logs: &str) -> Self {
            let completion_captures = COMPLETION_REGEX.captures(logs).unwrap();

            let formatted: Vec<FormattedFJ> = FORMATTED_REGEX
                .captures_iter(logs)
                .map(|capture| FormattedFJ {
                    file_name: capture[1].to_string(),
                })
                .collect();

            let failed: Vec<FailedFJ> = FAILED_REGEX
                .captures_iter(logs)
                .map(|capture| FailedFJ {
                    file_name: capture[1].to_string(),
                    error_message: capture[2].to_string(),
                })
                .collect();

            let check_failed: Vec<CheckFailedFJ> = CHECK_FAILED_REGEX
                .captures_iter(logs)
                .map(|capture| CheckFailedFJ {
                    file_name: capture[1].to_string(),
                })
                .collect();

            LoggedResults {
                num_processed: completion_captures[1].parse::<usize>().unwrap(),
                num_unchanged: completion_captures[2].parse::<usize>().unwrap(),
                num_formatted: completion_captures[3].parse::<usize>().unwrap(),
                num_failed: completion_captures[4].parse::<usize>().unwrap(),
                formatted,
                failed,
                check_failed,
            }
        }
    }

    #[test]
    fn test_fmt_all_java8() {
        //setup
        let temp_dir = create_temp_dir();

        for file_name in files_with_prefix("java8") {
            let file = TestableFile::new(file_name, &temp_dir);
            let temp_file = file.copy_to_temp();

            //exercise
            parallel!({
                cli::run(vec![
                    EXECUTABLE,
                    "fmt",
                    "tests/spec/java8",
                    "-t",
                    &temp_file,
                ]);
            });

            //verify
            file.assert_matches_output();
        }

        //teardown
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_format_directory() {
        //setup
        let temp_dir = create_temp_dir();

        let mut testable_files: Vec<TestableFile> = Vec::new();

        for file_name in files_with_prefix("json") {
            let file = TestableFile::new(file_name, &temp_dir);
            file.copy_to_temp();
            testable_files.push(file);
        }

        //exercise
        parallel!({
            cli::run(vec![EXECUTABLE, "fmt", "tests/spec/json", "-t", &temp_dir]);
        });

        //verify
        assert!(testable_files.len() > 1);
        for file in testable_files {
            file.assert_matches_output();
        }

        //teardown
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_missing_spec() {
        //exercise
        let output = Command::new(EXECUTABLE)
            .args(&["fmt", "-t", "tests/output"])
            .output()
            .unwrap();

        //verify
        let code = output.status.code().unwrap();
        assert_eq!(code, 1);

        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains(&not_provided_matcher("<SPECIFICATION>")));
    }

    #[test]
    fn test_missing_target() {
        //exercise
        let output = Command::new(EXECUTABLE)
            .args(&["fmt", "test/spec/java8"])
            .output()
            .unwrap();

        //verify
        let code = output.status.code().unwrap();
        assert_eq!(code, 1);

        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains(&not_provided_matcher("--target <PATH>")));
    }

    #[test]
    fn test_many_threads() {
        //setup
        let temp_dir = create_temp_dir();

        let mut testable_files: Vec<TestableFile> = Vec::new();

        for file_name in files_with_prefix("java8") {
            let file = TestableFile::new(file_name, &temp_dir);
            file.copy_to_temp();
            testable_files.push(file);
        }

        //exercise
        parallel!({
            cli::run(vec![
                EXECUTABLE,
                "fmt",
                "tests/spec/java8",
                "-t",
                &temp_dir,
                "--threads",
                "16",
            ]);
        });

        //verify
        assert!(testable_files.len() > 1);
        for file in testable_files {
            file.assert_matches_output();
        }

        //teardown
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_invalid_threads() {
        //setup
        let temp_dir = create_temp_dir();

        let mut testable_files: Vec<TestableFile> = Vec::new();

        for file_name in files_with_prefix("json") {
            let file = TestableFile::new(file_name, &temp_dir);
            file.copy_to_temp();
            testable_files.push(file);
        }

        //exercise
        parallel!({
            cli::run(vec![
                EXECUTABLE,
                "fmt",
                "tests/spec/json",
                "-t",
                &temp_dir,
                "--threads",
                "0",
            ]);
        });

        //verify
        assert!(testable_files.len() > 1);
        for file in testable_files {
            file.assert_matches_output();
        }

        //teardown
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_file_regex() {
        //setup
        let temp_dir = create_temp_dir();

        let mut testable_files: Vec<TestableFile> = Vec::new();

        for file_name in files_with_prefix("") {
            let file = TestableFile::new(file_name, &temp_dir);
            file.copy_to_temp();
            testable_files.push(file);
        }

        //exercise
        parallel!({
            cli::run(vec![
                EXECUTABLE,
                "fmt",
                "tests/spec/lacs",
                "-t",
                &temp_dir,
                "-m",
                "lacs_.*",
            ]);
        });

        //verify
        let mut formatted: usize = 0;
        assert!(testable_files.len() > 3);
        for file in testable_files {
            if file.file_name.starts_with("lacs_") {
                file.assert_matches_output();
                formatted += 1;
            }
        }

        assert_eq!(formatted, 3);

        //teardown
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_diff_tracking_unchanged() {
        //setup
        let temp_dir = create_temp_dir();

        let file = TestableFile::new("json_simple".to_string(), &temp_dir);
        let temp_path = file.copy_to_temp();

        parallel!({
            cli::run(vec![EXECUTABLE, "fmt", "tests/spec/json", "-t", &temp_path]);
        });

        //exercise/verify
        assert_does_not_modify_file(&temp_path, &|| {
            parallel!({
                cli::run(vec![EXECUTABLE, "fmt", "tests/spec/json", "-t", &temp_path]);
            });
        });

        file.assert_matches_output();

        //teardown
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_diff_tracking_file_modified() {
        //setup
        let temp_dir = create_temp_dir();

        let file = TestableFile::new("json_simple".to_string(), &temp_dir);
        let temp_path = file.copy_to_temp();

        parallel!({
            cli::run(vec![EXECUTABLE, "fmt", "tests/spec/json", "-t", &temp_path]);
        });

        // Modify the file at a strictly later system time (allowing for fluctuation)
        thread::sleep(Duration::from_millis(10));
        fs::write(&temp_path, "{\"modified\":\"value\"}").unwrap();

        //exercise/verify
        assert_modifies_file(&temp_path, &|| {
            parallel!({
                cli::run(vec![EXECUTABLE, "fmt", "tests/spec/json", "-t", &temp_path]);
            });
        });

        let result = fs::read_to_string(&temp_path).unwrap();
        assert_eq!(result, "{\n    \"modified\": \"value\"\n}\n");

        //teardown
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_diff_tracking_spec_modified() {
        //setup
        let temp_dir = create_temp_dir();

        let file = TestableFile::new("json_simple".to_string(), &temp_dir);
        let temp_path = file.copy_to_temp();

        parallel!({
            cli::run(vec![EXECUTABLE, "fmt", "tests/spec/json", "-t", &temp_path]);
        });

        // Sleep to allow for SystemTime fluctuations
        thread::sleep(Duration::from_millis(10));

        let new_spec_path = path_from_name(Path::new(&temp_dir), "spec");
        fs::copy("tests/spec/json", &new_spec_path).unwrap();

        let mut spec_file = OpenOptions::new()
            .append(true)
            .open(&new_spec_path)
            .unwrap();

        // Trivially modify the specification
        writeln!(spec_file, " ").unwrap();

        //exercise/verify
        assert_modifies_file(&temp_path, &|| {
            parallel!({
                cli::run(vec![
                    EXECUTABLE,
                    "fmt",
                    &new_spec_path.to_string_lossy().to_string(),
                    "-t",
                    &temp_path,
                ]);
            });
        });

        file.assert_matches_output();

        //teardown
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_clear_tracking_file() {
        //setup
        let temp_dir = create_temp_dir();

        let file = TestableFile::new("json_simple".to_string(), &temp_dir);
        let temp_path = file.copy_to_temp();

        parallel!({
            cli::run(vec![EXECUTABLE, "fmt", "tests/spec/json", "-t", &temp_path]);
        });

        //exercise
        parallel!({
            cli::run(vec![EXECUTABLE, "forget", &temp_path]);
        });

        //verify
        assert_modifies_file(&temp_path, &|| {
            parallel!({
                cli::run(vec![EXECUTABLE, "fmt", "tests/spec/json", "-t", &temp_path]);
            });
        });

        file.assert_matches_output();

        //teardown
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_clear_tracking_dir() {
        //setup
        let temp_dir = create_temp_dir();

        let file = TestableFile::new("json_simple".to_string(), &temp_dir);
        let temp_path = file.copy_to_temp();

        parallel!({
            cli::run(vec![EXECUTABLE, "fmt", "tests/spec/json", "-t", &temp_path]);
        });

        //exercise
        parallel!({
            cli::run(vec![EXECUTABLE, "forget", &temp_dir]);
        });

        //verify
        assert_modifies_file(&temp_path, &|| {
            parallel!({
                cli::run(vec![EXECUTABLE, "fmt", "tests/spec/json", "-t", &temp_path]);
            });
        });

        file.assert_matches_output();

        //teardown
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_clear_tracking_without_target() {
        //exercise
        let output = Command::new(EXECUTABLE).args(&["forget"]).output().unwrap();

        //verify
        let code = output.status.code().unwrap();
        assert_eq!(code, 1);

        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains(&not_provided_matcher("<PATH>")));
    }

    #[test]
    fn test_no_skip() {
        //setup
        let temp_dir = create_temp_dir();

        let file = TestableFile::new("json_simple".to_string(), &temp_dir);
        let temp_path = file.copy_to_temp();

        parallel!({
            cli::run(vec![EXECUTABLE, "fmt", "tests/spec/json", "-t", &temp_path]);
        });

        //exercise/verify
        assert_modifies_file(&temp_path, &|| {
            parallel!({
                cli::run(vec![
                    EXECUTABLE,
                    "fmt",
                    "tests/spec/json",
                    "-t",
                    &temp_path,
                    "--no-skip",
                ]);
            });
        });

        file.assert_matches_output();

        //teardown
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_no_track() {
        //setup
        let temp_dir = create_temp_dir();

        let file = TestableFile::new("json_simple".to_string(), &temp_dir);
        let temp_path = file.copy_to_temp();

        //exercise
        parallel!({
            cli::run(vec![
                EXECUTABLE,
                "fmt",
                "tests/spec/json",
                "-t",
                &temp_path,
                "--no-track",
            ]);
        });

        //verify
        assert_modifies_file(&temp_path, &|| {
            parallel!({
                cli::run(vec![EXECUTABLE, "fmt", "tests/spec/json", "-t", &temp_path]);
            });
        });

        file.assert_matches_output();

        //teardown
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_no_write() {
        //setup
        let temp_dir = create_temp_dir();

        let file = TestableFile::new("json_simple".to_string(), &temp_dir);
        let temp_path = file.copy_to_temp();

        //exercise/verify
        assert_does_not_modify_file(&temp_path, &|| {
            parallel!({
                cli::run(vec![
                    EXECUTABLE,
                    "fmt",
                    "tests/spec/json",
                    "-t",
                    &temp_path,
                    "--no-write",
                ]);
            });
        });

        //teardown
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_log_to_file_new() {
        //setup
        let temp_dir = create_temp_dir();

        let file = TestableFile::new("json_simple".to_string(), &temp_dir);
        let temp_path = file.copy_to_temp();

        serial!({
            let _ = fs::remove_file(&&*LOG_PATH);

            //exercise
            cli::run(vec![
                EXECUTABLE,
                "--log",
                &&*LOG_PATH,
                "--level",
                "debug",
                "fmt",
                "tests/spec/json",
                "-t",
                &temp_path,
            ]);

            //verify
            let logs = fs::read_to_string(&&*LOG_PATH).unwrap();
            assert!(logs.contains("INFO - Loading specification tests/spec/json ..."));
            assert!(logs.contains("INFO - COMPLETE:"));

            //teardown
            log::set_max_level(LevelFilter::Off);
            let _ = fs::remove_file(&&*LOG_PATH);
        });

        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_log_to_file_existing() {
        //setup
        let temp_dir = create_temp_dir();

        let file = TestableFile::new("json_simple".to_string(), &temp_dir);
        let temp_path = file.copy_to_temp();

        serial!({
            let _ = fs::remove_file(&&*LOG_PATH);

            //exercise
            for _ in 1..4 {
                cli::run(vec![
                    EXECUTABLE,
                    "--log",
                    &&*LOG_PATH,
                    "--level",
                    "debug",
                    "fmt",
                    "tests/spec/json",
                    "-t",
                    &temp_path,
                ]);
            }

            //verify
            let logs = fs::read_to_string(&&*LOG_PATH).unwrap();
            assert_eq!(logs.matches(r"INFO - COMPLETE:").count(), 3);

            //teardown
            log::set_max_level(LevelFilter::Off);
            let _ = fs::remove_file(&&*LOG_PATH);
        });

        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_set_log_level() {
        //setup
        let temp_dir = create_temp_dir();

        let levels = vec!["trace", "debug", "info", "warn", "error"];

        serial!({
            for level in levels {
                let _ = fs::remove_file(&&*LOG_PATH);

                //exercise
                cli::run(vec![
                    EXECUTABLE,
                    "--log",
                    &&*LOG_PATH,
                    "--level",
                    level,
                    "fmt",
                    "tests/spec/json",
                    "-t",
                    &temp_dir,
                ]);

                trace!("");
                debug!("");
                info!("");
                warn!("");
                error!("");

                //verify
                let logs = fs::read_to_string(&&*LOG_PATH).unwrap();

                match level {
                    "trace" => assert!(logs.contains("TRACE - ")),
                    "debug" => {
                        assert!(!logs.contains("TRACE - "));
                        assert!(logs.contains("DEBUG - "))
                    }
                    "info" => {
                        assert!(!logs.contains("DEBUG - "));
                        assert!(logs.contains("INFO - "));
                    }
                    "warn" => {
                        assert!(!logs.contains("INFO - "));
                        assert!(logs.contains("WARN - "));
                    }
                    "error" => {
                        assert!(!logs.contains("WARN - "));
                        assert!(logs.contains("ERROR - "));
                    }
                    &_ => panic!(),
                }
            }

            //teardown
            log::set_max_level(LevelFilter::Off);
            let _ = fs::remove_file(&&*LOG_PATH);
        });

        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_default_log_level() {
        //setup
        let temp_dir = create_temp_dir();

        serial!({
            let _ = fs::remove_file(&&*LOG_PATH);

            //exercise
            cli::run(vec![
                EXECUTABLE,
                "--log",
                &&*LOG_PATH,
                "fmt",
                "tests/spec/json",
                "-t",
                &temp_dir,
            ]);

            trace!("");
            debug!("");
            info!("");
            warn!("");
            error!("");

            //verify
            let logs = fs::read_to_string(&&*LOG_PATH).unwrap();
            assert!(!logs.contains("DEBUG - "));
            assert!(logs.contains("INFO - "));

            //teardown
            log::set_max_level(LevelFilter::Off);
            let _ = fs::remove_file(&&*LOG_PATH);
        });

        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_invalid_log_level() {
        //exercise
        let output = Command::new(EXECUTABLE)
            .args(&[
                "--log",
                &&*LOG_PATH,
                "--level",
                "invalid",
                "forget",
                "some/path",
            ])
            .output()
            .unwrap();

        //verify
        let code = output.status.code().unwrap();
        assert_eq!(code, 1);

        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains("error: 'invalid' isn't a valid value for '--level <LEVEL>'"));
    }

    #[test]
    fn test_formatting_passed() {
        //setup
        let temp_dir = create_temp_dir();

        let file = TestableFile::new("json_simple".to_string(), &temp_dir);
        let temp_path = file.copy_to_temp();

        serial!({
            let _ = fs::remove_file(&&*LOG_PATH);

            //exercise
            cli::run(vec![
                EXECUTABLE,
                "--log",
                &&*LOG_PATH,
                "--level",
                "debug",
                "fmt",
                "tests/spec/json",
                "-t",
                &temp_path,
            ]);

            //verify
            let logs = fs::read_to_string(&&*LOG_PATH).unwrap();
            let logged_results = LoggedResults::parse(&logs);

            assert_eq!(logged_results.num_processed, 1);
            assert_eq!(logged_results.num_formatted, 1);
            assert_eq!(logged_results.num_failed, 0);
            assert_eq!(logged_results.num_unchanged, 0);

            assert!(logged_results.failed.is_empty());

            assert!(logged_results.formatted.contains(&FormattedFJ {
                file_name: temp_path,
            }));

            //teardown
            log::set_max_level(LevelFilter::Off);
            let _ = fs::remove_file(&&*LOG_PATH);
        });

        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_formatting_failed() {
        //setup
        let temp_dir = create_temp_dir();
        let _ = fs::remove_file(&&*LOG_PATH);

        let file = TestableFile::new("java8_simple".to_string(), &temp_dir);
        let temp_path = file.copy_to_temp();

        serial!({
            let _ = fs::remove_file(&&*LOG_PATH);

            //exercise
            cli::run(vec![
                EXECUTABLE,
                "--log",
                &&*LOG_PATH,
                "--level",
                "debug",
                "fmt",
                "tests/spec/json",
                "-t",
                &temp_path,
            ]);

            //verify
            let logs = fs::read_to_string(&&*LOG_PATH).unwrap();
            let logged_results = LoggedResults::parse(&logs);

            assert_eq!(logged_results.num_processed, 1);
            assert_eq!(logged_results.num_formatted, 0);
            assert_eq!(logged_results.num_failed, 1);
            assert_eq!(logged_results.num_unchanged, 0);

            assert!(logged_results.formatted.is_empty());

            assert!(logged_results.failed.contains(&FailedFJ {
                file_name: temp_path,
                error_message: String::from(
                    "Failed to scan input: No accepting scans after (1,1): class Simp..."
                )
            }));

            //teardown
            log::set_max_level(LevelFilter::Off);
            let _ = fs::remove_file(&&*LOG_PATH);
        });

        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_formatting_unchanged() {
        //setup
        let temp_dir = create_temp_dir();

        let file = TestableFile::new("json_simple".to_string(), &temp_dir);
        let temp_path = file.copy_to_temp();

        serial!({
            cli::run(vec![EXECUTABLE, "fmt", "tests/spec/json", "-t", &temp_path]);

            let _ = fs::remove_file(&&*LOG_PATH);

            //exercise
            cli::run(vec![
                EXECUTABLE,
                "--log",
                &&*LOG_PATH,
                "--level",
                "debug",
                "fmt",
                "tests/spec/json",
                "-t",
                &temp_path,
            ]);

            //verify
            let logs = fs::read_to_string(&&*LOG_PATH).unwrap();
            let logged_results = LoggedResults::parse(&logs);

            assert_eq!(logged_results.num_processed, 1);
            assert_eq!(logged_results.num_formatted, 0);
            assert_eq!(logged_results.num_failed, 0);
            assert_eq!(logged_results.num_unchanged, 1);

            assert!(logged_results.formatted.is_empty());
            assert!(logged_results.failed.is_empty());

            //teardown
            log::set_max_level(LevelFilter::Off);
            let _ = fs::remove_file(&&*LOG_PATH);
        });

        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_check_formatting_ok() {
        //setup
        let temp_dir = create_temp_dir();

        let file = TestableFile::new("json_simple".to_string(), &temp_dir);
        let temp_path = file.copy_to_temp();

        serial!({
            cli::run(vec![EXECUTABLE, "fmt", "tests/spec/json", "-t", &temp_path]);

            let _ = fs::remove_file(&&*LOG_PATH);

            //exercise
            cli::run(vec![
                EXECUTABLE,
                "--log",
                &&*LOG_PATH,
                "--level",
                "debug",
                "fmt",
                "tests/spec/json",
                "-t",
                &temp_path,
                "--check",
            ]);

            //verify
            let logs = fs::read_to_string(&&*LOG_PATH).unwrap();
            let logged_results = LoggedResults::parse(&logs);

            assert_eq!(logged_results.num_processed, 1);
            assert_eq!(logged_results.num_formatted, 1);
            assert_eq!(logged_results.num_failed, 0);
            assert_eq!(logged_results.num_unchanged, 0);

            assert!(logged_results.check_failed.is_empty());

            //teardown
            log::set_max_level(LevelFilter::Off);
            let _ = fs::remove_file(&&*LOG_PATH);
        });

        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_check_formatting_failed() {
        //setup
        let temp_dir = create_temp_dir();

        let file = TestableFile::new("json_simple".to_string(), &temp_dir);
        let temp_path = file.copy_to_temp();

        serial!({
            let _ = fs::remove_file(&&*LOG_PATH);

            //exercise
            let mut failed = false;

            catch_fatal!({
                cli::run(vec![
                    EXECUTABLE,
                    "--log",
                    &&*LOG_PATH,
                    "--level",
                    "debug",
                    "fmt",
                    "tests/spec/json",
                    "-t",
                    &temp_path,
                    "--check",
                ]);
            }, {
                failed = true;
            });

            //verify
            assert!(failed);

            let logs = fs::read_to_string(&&*LOG_PATH).unwrap();
            let logged_results = LoggedResults::parse(&logs);

            assert_eq!(logged_results.num_processed, 1);
            assert_eq!(logged_results.num_formatted, 0);
            assert_eq!(logged_results.num_failed, 1);
            assert_eq!(logged_results.num_unchanged, 0);

            assert!(logged_results.check_failed.contains(&CheckFailedFJ {
                file_name: temp_path,
            }));

            //teardown
            log::set_max_level(LevelFilter::Off);
            let _ = fs::remove_file(&&*LOG_PATH);
        });

        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_start_server() {
        serial!({
            //setup
            server::kill();
            assert!(!server::running());

            thread::spawn(|| {
                // Allow time for the server to start
                thread::sleep(Duration::from_millis(20));

                //verify
                assert!(server::running());

                //teardown
                server::kill();
            });

            //exercise
            cli::run(vec![EXECUTABLE, "start-server"]);

            //verify
            assert!(!server::running());
        });
    }

    #[test]
    fn test_execute_on_server() {
        //setup
        let temp_dir = create_temp_dir();

        let file = TestableFile::new("json_simple".to_string(), &temp_dir);
        let temp_path = file.copy_to_temp();

        serial!({
            server::kill();
            assert!(!server::running());

            thread::spawn(move || {
                // Allow time for the server to start
                thread::sleep(Duration::from_millis(20));

                assert!(server::running());

                //exercise/verify
                assert_does_not_modify_file(&temp_path, &|| {
                    cli::run(vec![EXECUTABLE, "fmt", "tests/spec/json", "-t", &temp_path]);
                });

                assert_modifies_file(&temp_path, &|| {
                    // Wait long enough for server to format file
                    thread::sleep(Duration::from_millis(500));
                });

                //teardown
                server::kill();
            });

            cli::run(vec![EXECUTABLE, "start-server"]);

            //verify
            assert!(!server::running());
        });

        //verify
        file.assert_matches_output();

        //teardown
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_start_daemon() {
        serial!({
            //setup
            server::kill();
            assert!(!server::running());

            let _ = fs::remove_file(&&*LOG_PATH);

            //exercise
            cli::run(vec![EXECUTABLE, "--log", &&*LOG_PATH, "daemon", "start"]);

            //verify
            let logs = fs::read_to_string(&&*LOG_PATH).unwrap();
            assert!(logs.contains("Starting padd daemon"));

            // Allow time for the server to start
            thread::sleep(Duration::from_millis(20));

            assert!(server::running());

            //teardown
            log::set_max_level(LevelFilter::Off);
            let _ = fs::remove_file(&&*LOG_PATH);

            server::kill();
        });
    }

    #[test]
    fn test_start_daemon_already_running() {
        serial!({
            //setup
            server::kill();
            assert!(!server::running());

            cli::run(vec![EXECUTABLE, "daemon", "start"]);

            // Allow time for the server to start
            thread::sleep(Duration::from_millis(20));

            assert!(server::running());

            let _ = fs::remove_file(&&*LOG_PATH);

            //exercise
            cli::run(vec![EXECUTABLE, "--log", &&*LOG_PATH, "daemon", "start"]);

            //verify
            let logs = fs::read_to_string(&&*LOG_PATH).unwrap();
            assert!(logs.contains("Daemon already running"));

            assert!(server::running());

            //teardown
            log::set_max_level(LevelFilter::Off);
            let _ = fs::remove_file(&&*LOG_PATH);

            server::kill();
        });
    }

    #[test]
    fn test_kill_daemon() {
        serial!({
            //setup
            server::kill();
            assert!(!server::running());

            cli::run(vec![EXECUTABLE, "daemon", "start"]);

            // Allow time for the server to start
            thread::sleep(Duration::from_millis(20));

            assert!(server::running());

            //exercise
            cli::run(vec![EXECUTABLE, "daemon", "kill"]);

            // Allow time for the server to stop
            thread::sleep(Duration::from_millis(20));

            //verify
            assert!(!server::running());

            //teardown
            server::kill();
        });
    }

    #[test]
    fn test_kill_daemon_not_running() {
        serial!({
            //setup
            server::kill();
            assert!(!server::running());

            //exercise
            cli::run(vec![EXECUTABLE, "daemon", "kill"]);

            //verify
            assert!(!server::running());
        });
    }

    #[test]
    fn test_format_via_daemon() {
        //setup
        let temp_dir = create_temp_dir();

        let file = TestableFile::new("json_simple".to_string(), &temp_dir);
        let temp_path = file.copy_to_temp();

        serial!({
            server::kill();
            assert!(!server::running());

            cli::run(vec![EXECUTABLE, "daemon", "start"]);

            // Allow time for the server to start
            thread::sleep(Duration::from_millis(20));

            assert!(server::running());

            //exercise/verify
            assert_does_not_modify_file(&temp_path, &|| {
                cli::run(vec![EXECUTABLE, "fmt", "tests/spec/json", "-t", &temp_path]);
            });

            assert_modifies_file(&temp_path, &|| {
                // Wait long enough for server to format file
                thread::sleep(Duration::from_millis(500));
            });

            //teardown
            server::kill();
        });

        //verify
        file.assert_matches_output();

        //teardown
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_cache_fjr() {
        //setup
        let temp_dir = create_temp_dir();

        let file = TestableFile::new("balanced_brackets".to_string(), &temp_dir);
        let temp_path = file.copy_to_temp();

        serial!({
            cli::run(vec![
                EXECUTABLE,
                "fmt",
                "tests/spec/balanced_brackets",
                "-t",
                &temp_path,
            ]);

            let _ = fs::remove_file(&&*LOG_PATH);

            //exercise
            cli::run(vec![
                EXECUTABLE,
                "--log",
                &&*LOG_PATH,
                "fmt",
                "tests/spec/balanced_brackets",
                "-t",
                &temp_path,
                "--no-skip",
            ]);

            //verify
            let logs = fs::read_to_string(&&*LOG_PATH).unwrap();
            assert!(logs.contains("Loading cached specification"));

            //teardown
            log::set_max_level(LevelFilter::Off);
            let _ = fs::remove_file(&&*LOG_PATH);
        });

        //verify
        file.assert_matches_output();

        //teardown
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    fn assert_modifies_file(file_path: &str, modifier: &Fn()) {
        assert!(modifies_file(file_path, modifier));
    }

    fn assert_does_not_modify_file(file_path: &str, modifier: &Fn()) {
        assert!(!modifies_file(file_path, modifier));
    }

    fn modifies_file(file_path: &str, modifier: &Fn()) -> bool {
        let initially_modified_at = Path::new(file_path).metadata().unwrap().modified().unwrap();

        modifier();

        let finally_modified_at = Path::new(file_path).metadata().unwrap().modified().unwrap();

        initially_modified_at != finally_modified_at
    }

    fn create_temp_dir() -> String {
        let temp_dir = format!("tests/temp-{}", Uuid::new_v4().to_string());
        fs::create_dir(&temp_dir).unwrap();
        temp_dir
    }

    fn read_to_string(path: &Path) -> String {
        let mut contents = String::new();

        File::open(path)
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();

        contents
    }

    fn not_provided_matcher(argument: &str) -> String {
        format!(
            "error: The following required arguments were not provided:\n    {}",
            argument
        )
    }

    fn files_with_prefix(prefix: &str) -> Vec<String> {
        fs::read_dir(&*INPUT_DIR)
            .unwrap()
            .map(|entry| entry.unwrap())
            .filter(|entry| !entry.path().is_dir())
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .filter(|file_name| file_name.starts_with(prefix))
            .collect()
    }

    fn path_from_name(dir: &Path, file_name: &str) -> PathBuf {
        let mut path_buf = dir.to_path_buf();
        path_buf.push(&file_name);
        path_buf
    }
}
