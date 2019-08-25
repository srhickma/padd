#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate backtrace;
extern crate padd;

use {
    cli::logger::Fatal,
    std::{env, panic, process},
};

#[macro_use]
mod cli;

fn main() {
    let args: Vec<String> = env::args().collect();

    catch_fatal!(
        {
            cli::run(args.iter().map(|s| &**s).collect());
        },
        {
            process::exit(1);
        }
    );
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
        static ref TEST_DIR: PathBuf = fs::canonicalize(Path::new("tests")).unwrap();
        static ref INPUT_DIR: PathBuf = fs::canonicalize(Path::new("tests/input")).unwrap();
        static ref OUTPUT_DIR: PathBuf = fs::canonicalize(Path::new("tests/output")).unwrap();

        static ref LOG_PATH: String = String::from("tests/test.log");

        static ref SERIALIZATION_LOCK: RwLock<()> = RwLock::new(());

        static ref COMPLETION_REGEX: Regex = Regex::new(
            r"INFO - COMPLETE: \d*ms : (\d*) processed, (\d*) unchanged, (\d*) formatted, (\d*) failed"
        ).unwrap();
        static ref FORMATTED_REGEX: Regex = Regex::new(
            r"DEBUG - Finished formatting ([^\n]*)"
        ).unwrap();
        static ref FAILED_REGEX: Regex = Regex::new(
            r"(WARN|ERROR) - Error formatting ([^\n:]*): ([^\n]*)"
        ).unwrap();
        static ref CHECK_FAILED_REGEX: Regex = Regex::new(
            r"ERROR - Formatting check failed for ([^\n:]*)"
        ).unwrap();
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

    struct TestDir {
        path_buf: PathBuf,
        path_str: String,
        released: bool,
    }

    impl TestDir {
        fn new() -> Self {
            let mut path_buf = TEST_DIR.clone();
            path_buf.push(format!("temp-{}", Uuid::new_v4().to_string()));
            fs::create_dir(path_buf.as_path()).unwrap();

            let path_str = path_buf.to_string_lossy().to_string();

            TestDir {
                path_buf,
                path_str,
                released: false,
            }
        }

        fn path(&self) -> &Path {
            self.path_buf.as_path()
        }

        fn path_str(&self) -> &str {
            &self.path_str
        }

        fn release(&mut self) {
            self.released = true;
            fs::remove_dir_all(self.path()).unwrap();
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            if !self.released {
                self.release();
            }
        }
    }

    struct TestableFile<'scope> {
        file_name: String,
        path_str: String,
        test_dir: &'scope TestDir,
    }

    impl<'scope> TestableFile<'scope> {
        fn new(file_name: String, test_dir: &'scope TestDir) -> Self {
            let input_path = path_from_name(&INPUT_DIR, &file_name);
            let test_path = path_from_name(test_dir.path(), &file_name);
            fs::copy(input_path, &test_path).unwrap();

            TestableFile {
                file_name,
                path_str: test_path.as_path().to_string_lossy().to_string(),
                test_dir,
            }
        }

        fn path_str(&self) -> &str {
            &self.path_str
        }

        fn assert_modified_by(&self, modifier: &dyn Fn()) {
            assert!(self.modified_by(modifier));
        }

        fn assert_not_modified_by(&self, modifier: &dyn Fn()) {
            assert!(!self.modified_by(modifier));
        }

        fn modified_by(&self, modifier: &dyn Fn()) -> bool {
            let initially_modified_at = Path::new(&self.path_str)
                .metadata()
                .unwrap()
                .modified()
                .unwrap();

            modifier();

            let finally_modified_at = Path::new(&self.path_str)
                .metadata()
                .unwrap()
                .modified()
                .unwrap();

            initially_modified_at != finally_modified_at
        }

        fn assert_matches_output(&self) {
            let output_path = path_from_name(&OUTPUT_DIR, &self.file_name);
            let temp_path = path_from_name(self.test_dir.path(), &self.file_name);

            let expected = read_to_string(output_path.as_path());
            let actual = read_to_string(temp_path.as_path());
            if expected != actual {
                println!("EXPECTED:\n{}\nBUT FOUND:\n{}", expected, actual);
                panic!("Temp file did not match output file")
            }
        }

        fn assert_does_not_match_output(&self) {
            let output_path = path_from_name(&OUTPUT_DIR, &self.file_name);
            let temp_path = path_from_name(self.test_dir.path(), &self.file_name);

            let expected = read_to_string(output_path.as_path());
            let actual = read_to_string(temp_path.as_path());
            if expected == actual {
                panic!("Expected temp file to not match output file")
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
                    file_name: capture[2].to_string(),
                    error_message: capture[3].to_string(),
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
        let mut test_dir = TestDir::new();

        for file_name in files_with_prefix("java8") {
            let file = TestableFile::new(file_name, &test_dir);

            //exercise
            parallel!({
                cli::run(vec![
                    EXECUTABLE,
                    "fmt",
                    "tests/spec/java8",
                    "-t",
                    file.path_str(),
                ]);
            });

            //verify
            file.assert_matches_output();
        }

        //teardown
        test_dir.release();
    }

    #[test]
    fn test_format_directory() {
        //setup
        let mut test_dir = TestDir::new();

        let mut testable_files: Vec<TestableFile> = Vec::new();

        for file_name in files_with_prefix("json") {
            let file = TestableFile::new(file_name, &test_dir);
            testable_files.push(file);
        }

        //exercise
        parallel!({
            cli::run(vec![
                EXECUTABLE,
                "fmt",
                "tests/spec/json",
                "-t",
                test_dir.path_str(),
            ]);
        });

        //verify
        assert!(testable_files.len() > 1);
        for file in testable_files {
            file.assert_matches_output();
        }

        //teardown
        test_dir.release();
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
        let mut test_dir = TestDir::new();

        let mut testable_files: Vec<TestableFile> = Vec::new();

        for file_name in files_with_prefix("java8") {
            let file = TestableFile::new(file_name, &test_dir);
            testable_files.push(file);
        }

        //exercise
        parallel!({
            cli::run(vec![
                EXECUTABLE,
                "fmt",
                "tests/spec/java8",
                "-t",
                test_dir.path_str(),
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
        test_dir.release();
    }

    #[test]
    fn test_invalid_threads_zero() {
        //setup
        let mut test_dir = TestDir::new();

        let mut testable_files: Vec<TestableFile> = Vec::new();

        for file_name in files_with_prefix("json") {
            let file = TestableFile::new(file_name, &test_dir);
            testable_files.push(file);
        }

        //exercise
        parallel!({
            cli::run(vec![
                EXECUTABLE,
                "fmt",
                "tests/spec/json",
                "-t",
                test_dir.path_str(),
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
        test_dir.release();
    }

    #[test]
    fn test_invalid_threads_character() {
        //setup
        let mut test_dir = TestDir::new();

        let mut testable_files: Vec<TestableFile> = Vec::new();

        for file_name in files_with_prefix("json") {
            let file = TestableFile::new(file_name, &test_dir);
            testable_files.push(file);
        }

        //exercise
        parallel!({
            cli::run(vec![
                EXECUTABLE,
                "fmt",
                "tests/spec/json",
                "-t",
                test_dir.path_str(),
                "--threads",
                "a",
            ]);
        });

        //verify
        assert!(testable_files.len() > 1);
        for file in testable_files {
            file.assert_matches_output();
        }

        //teardown
        test_dir.release();
    }

    #[test]
    fn test_file_regex() {
        //setup
        let mut test_dir = TestDir::new();

        let mut testable_files: Vec<TestableFile> = Vec::new();

        for file_name in files_with_prefix("") {
            let file = TestableFile::new(file_name, &test_dir);
            testable_files.push(file);
        }

        //exercise
        parallel!({
            cli::run(vec![
                EXECUTABLE,
                "fmt",
                "tests/spec/lacs",
                "-t",
                test_dir.path_str(),
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
        test_dir.release();
    }

    #[test]
    fn test_diff_tracking_unchanged() {
        //setup
        let mut test_dir = TestDir::new();
        let file = TestableFile::new("json_simple".to_string(), &test_dir);

        parallel!({
            cli::run(vec![
                EXECUTABLE,
                "fmt",
                "tests/spec/json",
                "-t",
                file.path_str(),
            ]);
        });

        //exercise/verify
        file.assert_not_modified_by(&|| {
            parallel!({
                cli::run(vec![
                    EXECUTABLE,
                    "fmt",
                    "tests/spec/json",
                    "-t",
                    file.path_str(),
                ]);
            });
        });

        file.assert_matches_output();

        //teardown
        test_dir.release();
    }

    #[test]
    fn test_diff_tracking_file_modified() {
        //setup
        let mut test_dir = TestDir::new();
        let file = TestableFile::new("json_simple".to_string(), &test_dir);

        parallel!({
            cli::run(vec![
                EXECUTABLE,
                "fmt",
                "tests/spec/json",
                "-t",
                file.path_str(),
            ]);
        });

        // Modify the file at a strictly later system time (allowing for fluctuation)
        thread::sleep(Duration::from_millis(10));
        fs::write(file.path_str(), "{\"modified\":\"value\"}").unwrap();

        //exercise/verify
        file.assert_modified_by(&|| {
            parallel!({
                cli::run(vec![
                    EXECUTABLE,
                    "fmt",
                    "tests/spec/json",
                    "-t",
                    file.path_str(),
                ]);
            });
        });

        let result = fs::read_to_string(file.path_str()).unwrap();
        assert_eq!(result, "{\n    \"modified\": \"value\"\n}\n");

        //teardown
        test_dir.release();
    }

    #[test]
    fn test_diff_tracking_spec_modified() {
        //setup
        let mut test_dir = TestDir::new();
        let file = TestableFile::new("json_simple".to_string(), &test_dir);

        parallel!({
            cli::run(vec![
                EXECUTABLE,
                "fmt",
                "tests/spec/json",
                "-t",
                file.path_str(),
            ]);
        });

        // Sleep to allow for SystemTime fluctuations
        thread::sleep(Duration::from_millis(10));

        let new_spec_path = path_from_name(test_dir.path(), "spec");
        parallel!({
            fs::copy("tests/spec/json", &new_spec_path).unwrap();
        });

        let mut spec_file = OpenOptions::new()
            .append(true)
            .open(&new_spec_path)
            .unwrap();

        // Trivially modify the specification
        writeln!(spec_file, " ").unwrap();

        //exercise/verify
        file.assert_modified_by(&|| {
            parallel!({
                cli::run(vec![
                    EXECUTABLE,
                    "fmt",
                    &new_spec_path.to_string_lossy().to_string(),
                    "-t",
                    file.path_str(),
                ]);
            });
        });

        file.assert_matches_output();

        //teardown
        test_dir.release();
    }

    #[test]
    fn test_clear_tracking_file() {
        //setup
        let mut test_dir = TestDir::new();
        let file = TestableFile::new("json_simple".to_string(), &test_dir);

        parallel!({
            cli::run(vec![
                EXECUTABLE,
                "fmt",
                "tests/spec/json",
                "-t",
                file.path_str(),
            ]);
        });

        //exercise
        parallel!({
            cli::run(vec![EXECUTABLE, "forget", file.path_str()]);
        });

        //verify
        file.assert_modified_by(&|| {
            parallel!({
                cli::run(vec![
                    EXECUTABLE,
                    "fmt",
                    "tests/spec/json",
                    "-t",
                    file.path_str(),
                ]);
            });
        });

        file.assert_matches_output();

        //teardown
        test_dir.release();
    }

    #[test]
    fn test_clear_tracking_dir() {
        //setup
        let mut test_dir = TestDir::new();
        let file = TestableFile::new("json_simple".to_string(), &test_dir);

        parallel!({
            cli::run(vec![
                EXECUTABLE,
                "fmt",
                "tests/spec/json",
                "-t",
                file.path_str(),
            ]);
        });

        //exercise
        parallel!({
            cli::run(vec![EXECUTABLE, "forget", test_dir.path_str()]);
        });

        //verify
        file.assert_modified_by(&|| {
            parallel!({
                cli::run(vec![
                    EXECUTABLE,
                    "fmt",
                    "tests/spec/json",
                    "-t",
                    file.path_str(),
                ]);
            });
        });

        file.assert_matches_output();

        //teardown
        test_dir.release();
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
        let mut test_dir = TestDir::new();
        let file = TestableFile::new("json_simple".to_string(), &test_dir);

        parallel!({
            cli::run(vec![
                EXECUTABLE,
                "fmt",
                "tests/spec/json",
                "-t",
                file.path_str(),
            ]);
        });

        //exercise/verify
        file.assert_modified_by(&|| {
            parallel!({
                cli::run(vec![
                    EXECUTABLE,
                    "fmt",
                    "tests/spec/json",
                    "-t",
                    file.path_str(),
                    "--no-skip",
                ]);
            });
        });

        file.assert_matches_output();

        //teardown
        test_dir.release();
    }

    #[test]
    fn test_no_track() {
        //setup
        let mut test_dir = TestDir::new();
        let file = TestableFile::new("json_simple".to_string(), &test_dir);

        //exercise
        parallel!({
            cli::run(vec![
                EXECUTABLE,
                "fmt",
                "tests/spec/json",
                "-t",
                file.path_str(),
                "--no-track",
            ]);
        });

        //verify
        file.assert_modified_by(&|| {
            parallel!({
                cli::run(vec![
                    EXECUTABLE,
                    "fmt",
                    "tests/spec/json",
                    "-t",
                    file.path_str(),
                ]);
            });
        });

        file.assert_matches_output();

        //teardown
        test_dir.release();
    }

    #[test]
    fn test_no_write() {
        //setup
        let mut test_dir = TestDir::new();
        let file = TestableFile::new("json_simple".to_string(), &test_dir);

        //exercise/verify
        file.assert_not_modified_by(&|| {
            parallel!({
                cli::run(vec![
                    EXECUTABLE,
                    "fmt",
                    "tests/spec/json",
                    "-t",
                    file.path_str(),
                    "--no-write",
                ]);
            });
        });

        //teardown
        test_dir.release();
    }

    #[test]
    fn test_log_to_file_new() {
        //setup
        let mut test_dir = TestDir::new();
        let file = TestableFile::new("json_simple".to_string(), &test_dir);

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
                file.path_str(),
            ]);

            //verify
            let logs = fs::read_to_string(&&*LOG_PATH).unwrap();
            assert!(logs.contains("INFO - Loading specification tests/spec/json ..."));
            assert!(logs.contains("INFO - COMPLETE:"));

            //teardown
            log::set_max_level(LevelFilter::Off);
            let _ = fs::remove_file(&&*LOG_PATH);
        });

        //teardown
        test_dir.release();
    }

    #[test]
    fn test_log_to_file_existing() {
        //setup
        let mut test_dir = TestDir::new();
        let file = TestableFile::new("json_simple".to_string(), &test_dir);

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
                    file.path_str(),
                ]);
            }

            //verify
            let logs = fs::read_to_string(&&*LOG_PATH).unwrap();
            assert_eq!(logs.matches(r"INFO - COMPLETE:").count(), 3);

            //teardown
            log::set_max_level(LevelFilter::Off);
            let _ = fs::remove_file(&&*LOG_PATH);
        });

        //teardown
        test_dir.release();
    }

    #[test]
    fn test_set_log_level() {
        //setup
        let mut test_dir = TestDir::new();
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
                    test_dir.path_str(),
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

        //teardown
        test_dir.release();
    }

    #[test]
    fn test_default_log_level() {
        //setup
        let test_dir = TestDir::new();

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
                test_dir.path_str(),
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
        let mut test_dir = TestDir::new();
        let file = TestableFile::new("json_simple".to_string(), &test_dir);

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
                file.path_str(),
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
                file_name: file.path_str().to_string(),
            }));

            //teardown
            log::set_max_level(LevelFilter::Off);
            let _ = fs::remove_file(&&*LOG_PATH);
        });

        //teardown
        test_dir.release();
    }

    #[test]
    fn test_formatting_failed() {
        //setup
        let mut test_dir = TestDir::new();
        let file = TestableFile::new("java8_simple".to_string(), &test_dir);

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
                file.path_str(),
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
                file_name: file.path_str().to_string(),
                error_message: String::from(
                    "Failed to lex input: No accepting tokens after (1,1): class Simp..."
                )
            }));

            //teardown
            log::set_max_level(LevelFilter::Off);
            let _ = fs::remove_file(&&*LOG_PATH);
        });

        //teardown
        test_dir.release();
    }

    #[test]
    fn test_formatting_unchanged() {
        //setup
        let mut test_dir = TestDir::new();
        let file = TestableFile::new("json_simple".to_string(), &test_dir);

        serial!({
            cli::run(vec![
                EXECUTABLE,
                "fmt",
                "tests/spec/json",
                "-t",
                file.path_str(),
            ]);

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
                file.path_str(),
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

        //teardown
        test_dir.release();
    }

    #[test]
    fn test_check_formatting_ok() {
        //setup
        let mut test_dir = TestDir::new();
        let file = TestableFile::new("json_simple".to_string(), &test_dir);

        serial!({
            cli::run(vec![
                EXECUTABLE,
                "fmt",
                "tests/spec/json",
                "-t",
                file.path_str(),
            ]);

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
                file.path_str(),
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

        //teardown
        test_dir.release();
    }

    #[test]
    fn test_check_formatting_failed() {
        //setup
        let mut test_dir = TestDir::new();
        let file = TestableFile::new("json_simple".to_string(), &test_dir);

        serial!({
            let _ = fs::remove_file(&&*LOG_PATH);
            let mut failed = false;

            //exercise
            catch_fatal!(
                {
                    cli::run(vec![
                        EXECUTABLE,
                        "--log",
                        &&*LOG_PATH,
                        "--level",
                        "debug",
                        "fmt",
                        "tests/spec/json",
                        "-t",
                        file.path_str(),
                        "--check",
                    ]);
                },
                {
                    failed = true;
                }
            );

            //verify
            assert!(failed);

            let logs = fs::read_to_string(&&*LOG_PATH).unwrap();
            let logged_results = LoggedResults::parse(&logs);

            assert_eq!(logged_results.num_processed, 1);
            assert_eq!(logged_results.num_formatted, 0);
            assert_eq!(logged_results.num_failed, 1);
            assert_eq!(logged_results.num_unchanged, 0);

            assert!(logged_results.check_failed.contains(&CheckFailedFJ {
                file_name: file.path_str().to_string(),
            }));

            //teardown
            log::set_max_level(LevelFilter::Off);
            let _ = fs::remove_file(&&*LOG_PATH);
        });

        //teardown
        test_dir.release();
    }

    #[test]
    fn test_check_formatting_error() {
        //setup
        let mut test_dir = TestDir::new();
        let file = TestableFile::new("java8_simple".to_string(), &test_dir);

        serial!({
            let _ = fs::remove_file(&&*LOG_PATH);
            let mut failed = false;

            //exercise
            catch_fatal!(
                {
                    cli::run(vec![
                        EXECUTABLE,
                        "--log",
                        &&*LOG_PATH,
                        "--level",
                        "debug",
                        "fmt",
                        "tests/spec/json",
                        "-t",
                        file.path_str(),
                        "--check",
                    ]);
                },
                {
                    failed = true;
                }
            );

            //verify
            assert!(failed);

            let logs = fs::read_to_string(&&*LOG_PATH).unwrap();
            let logged_results = LoggedResults::parse(&logs);

            assert_eq!(logged_results.num_processed, 1);
            assert_eq!(logged_results.num_formatted, 0);
            assert_eq!(logged_results.num_failed, 1);
            assert_eq!(logged_results.num_unchanged, 0);

            println!("{}", logged_results.failed[0].error_message);

            assert!(logged_results.failed.contains(&FailedFJ {
                file_name: file.path_str().to_string(),
                error_message: String::from(
                    "Failed to lex input: No accepting tokens after (1,1): class Simp..."
                )
            }));

            //teardown
            log::set_max_level(LevelFilter::Off);
            let _ = fs::remove_file(&&*LOG_PATH);
        });

        //teardown
        test_dir.release();
    }

    #[test]
    fn test_specification_not_found() {
        serial!({
            //setup
            let _ = fs::remove_file(&&*LOG_PATH);
            let mut failed = false;

            //exercise
            catch_fatal!(
                {
                    cli::run(vec![
                        EXECUTABLE,
                        "--log",
                        &&*LOG_PATH,
                        "--level",
                        "debug",
                        "fmt",
                        "tests/spec/non-existant-specification",
                        "-t",
                        "some/path",
                    ]);
                },
                {
                    failed = true;
                }
            );

            //verify
            assert!(failed);

            let logs = fs::read_to_string(&&*LOG_PATH).unwrap();
            assert!(logs.contains(
                "ERROR - Error loading specification \
                 tests/spec/non-existant-specification: Could not find specification file \
                 \"tests/spec/non-existant-specification\""
            ));

            //teardown
            log::set_max_level(LevelFilter::Off);
            let _ = fs::remove_file(&&*LOG_PATH);
        });
    }

    #[test]
    fn test_invalid_specification() {
        serial!({
            //setup
            let _ = fs::remove_file(&&*LOG_PATH);
            let mut failed = false;

            //exercise
            catch_fatal!(
                {
                    cli::run(vec![
                        EXECUTABLE,
                        "--log",
                        &&*LOG_PATH,
                        "--level",
                        "debug",
                        "fmt",
                        "tests/output/json_simple",
                        "-t",
                        "some/path",
                    ]);
                },
                {
                    failed = true;
                }
            );

            //verify
            assert!(failed);

            let logs = fs::read_to_string(&&*LOG_PATH).unwrap();
            assert!(logs.contains("ERROR - Error loading specification tests/output/json_simple"));

            //teardown
            log::set_max_level(LevelFilter::Off);
            let _ = fs::remove_file(&&*LOG_PATH);
        });
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
        serial!({
            //setup
            server::kill();
            assert!(!server::running());

            thread::spawn(move || {
                let test_dir = TestDir::new();
                let file = TestableFile::new("json_simple".to_string(), &test_dir);

                // Allow time for the server to start
                thread::sleep(Duration::from_millis(20));

                assert!(server::running());

                //exercise/verify
                file.assert_not_modified_by(&|| {
                    cli::run(vec![
                        EXECUTABLE,
                        "fmt",
                        "tests/spec/json",
                        "-t",
                        file.path_str(),
                    ]);
                });

                file.assert_modified_by(&|| {
                    // Wait long enough for server to format file
                    thread::sleep(Duration::from_millis(500));
                });

                file.assert_matches_output();

                //teardown
                server::kill();
            });

            cli::run(vec![EXECUTABLE, "start-server"]);

            //verify
            assert!(!server::running());
        });
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
        let mut test_dir = TestDir::new();
        let file = TestableFile::new("json_simple".to_string(), &test_dir);

        serial!({
            server::kill();
            assert!(!server::running());

            cli::run(vec![EXECUTABLE, "daemon", "start"]);

            // Allow time for the server to start
            thread::sleep(Duration::from_millis(20));

            assert!(server::running());

            //exercise/verify
            file.assert_not_modified_by(&|| {
                cli::run(vec![
                    EXECUTABLE,
                    "fmt",
                    "tests/spec/json",
                    "-t",
                    file.path_str(),
                ]);
            });

            file.assert_modified_by(&|| {
                // Wait long enough for server to format file
                thread::sleep(Duration::from_millis(500));
            });

            //teardown
            server::kill();
        });

        //verify
        file.assert_matches_output();

        //teardown
        test_dir.release();
    }

    #[test]
    fn test_cache_fjr() {
        //setup
        let mut test_dir = TestDir::new();
        let file = TestableFile::new("balanced_brackets".to_string(), &test_dir);

        serial!({
            cli::run(vec![
                EXECUTABLE,
                "fmt",
                "tests/spec/balanced_brackets",
                "-t",
                file.path_str(),
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
                file.path_str(),
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
        test_dir.release();
    }

    #[test]
    fn test_pwd_shorthand() {
        //setup
        let mut test_dir1 = TestDir::new();
        let file1 = TestableFile::new("balanced_brackets".to_string(), &test_dir1);

        let mut test_dir2 = TestDir::new();
        let file2 = TestableFile::new("balanced_brackets".to_string(), &test_dir2);

        let usr_dir = env::current_dir().unwrap();

        serial!({
            env::set_current_dir(test_dir1.path()).unwrap();

            //exercise
            cli::run(vec![
                &format!("../../{}", EXECUTABLE),
                "fmt",
                "../spec/balanced_brackets",
                "-t",
                ".",
            ]);

            env::set_current_dir(&usr_dir).unwrap();
        });

        //verify
        file1.assert_matches_output();
        file2.assert_does_not_match_output();

        //teardown
        test_dir1.release();
        test_dir2.release();
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
