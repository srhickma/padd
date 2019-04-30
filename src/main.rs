#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate padd;

use std::env;

mod cli;

fn main() {
    let args: Vec<String> = env::args().collect();
    cli::run(args.iter().map(|s| &**s).collect());
}

#[cfg(test)]
mod tests {
    extern crate uuid;

    use super::*;

    use {
        self::uuid::Uuid,
        std::{
            fs::{self, File, OpenOptions},
            io::{prelude::*, Read},
            path::{Path, PathBuf},
            process::Command,
            thread,
            time::Duration,
        },
    };

    static EXECUTABLE: &'static str = "target/debug/padd";

    lazy_static! {
        static ref TEMP_DIR: &'static Path = Path::new("tests/temp");
        static ref INPUT_DIR: &'static Path = Path::new("tests/input");
        static ref OUTPUT_DIR: &'static Path = Path::new("tests/output");
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

    #[test]
    fn test_fmt_all_java8() {
        //setup
        let temp_dir = create_temp_dir();

        for file_name in files_with_prefix("java8") {
            let file = TestableFile::new(file_name, &temp_dir);
            let temp_file = file.copy_to_temp();

            //exercise
            cli::run(vec!["padd", "fmt", "tests/spec/java8", "-t", &temp_file]);

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
        cli::run(vec!["padd", "fmt", "tests/spec/json", "-t", &temp_dir]);

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
        cli::run(vec![
            "padd",
            "fmt",
            "tests/spec/java8",
            "-t",
            &temp_dir,
            "--threads",
            "16",
        ]);

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
        cli::run(vec![
            "padd",
            "fmt",
            "tests/spec/json",
            "-t",
            &temp_dir,
            "--threads",
            "0",
        ]);

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
        cli::run(vec![
            "padd",
            "fmt",
            "tests/spec/lacs",
            "-t",
            &temp_dir,
            "-m",
            "lacs_.*",
        ]);

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

        cli::run(vec!["padd", "fmt", "tests/spec/json", "-t", &temp_path]);

        let initially_modified_at = Path::new(&temp_path)
            .metadata()
            .unwrap()
            .modified()
            .unwrap();

        //exercise
        cli::run(vec!["padd", "fmt", "tests/spec/json", "-t", &temp_path]);

        //verify
        let finally_modified_at = Path::new(&temp_path)
            .metadata()
            .unwrap()
            .modified()
            .unwrap();

        assert_eq!(initially_modified_at, finally_modified_at);

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

        cli::run(vec!["padd", "fmt", "tests/spec/json", "-t", &temp_path]);

        // Modify the file at a strictly later system time (allowing for fluctuation)
        thread::sleep(Duration::from_millis(10));
        fs::write(&temp_path, "{\"modified\":\"value\"}").unwrap();

        let initially_modified_at = Path::new(&temp_path)
            .metadata()
            .unwrap()
            .modified()
            .unwrap();

        //exercise
        cli::run(vec!["padd", "fmt", "tests/spec/json", "-t", &temp_path]);

        //verify
        let finally_modified_at = Path::new(&temp_path)
            .metadata()
            .unwrap()
            .modified()
            .unwrap();

        assert_ne!(initially_modified_at, finally_modified_at);

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

        cli::run(vec!["padd", "fmt", "tests/spec/json", "-t", &temp_path]);

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

        let initially_modified_at = Path::new(&temp_path)
            .metadata()
            .unwrap()
            .modified()
            .unwrap();

        //exercise
        cli::run(vec![
            "padd",
            "fmt",
            &new_spec_path.to_string_lossy().to_string(),
            "-t",
            &temp_path,
        ]);

        //verify
        let finally_modified_at = Path::new(&temp_path)
            .metadata()
            .unwrap()
            .modified()
            .unwrap();

        assert_ne!(initially_modified_at, finally_modified_at);

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

        cli::run(vec!["padd", "fmt", "tests/spec/json", "-t", &temp_path]);

        let initially_modified_at = Path::new(&temp_path)
            .metadata()
            .unwrap()
            .modified()
            .unwrap();

        //exercise
        cli::run(vec!["padd", "forget", &temp_path]);

        //verify
        cli::run(vec!["padd", "fmt", "tests/spec/json", "-t", &temp_path]);

        let finally_modified_at = Path::new(&temp_path)
            .metadata()
            .unwrap()
            .modified()
            .unwrap();

        assert_ne!(initially_modified_at, finally_modified_at);

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

        cli::run(vec!["padd", "fmt", "tests/spec/json", "-t", &temp_path]);

        let initially_modified_at = Path::new(&temp_path)
            .metadata()
            .unwrap()
            .modified()
            .unwrap();

        //exercise
        cli::run(vec!["padd", "forget", &temp_dir]);

        //verify
        cli::run(vec!["padd", "fmt", "tests/spec/json", "-t", &temp_path]);

        let finally_modified_at = Path::new(&temp_path)
            .metadata()
            .unwrap()
            .modified()
            .unwrap();

        assert_ne!(initially_modified_at, finally_modified_at);

        file.assert_matches_output();

        //teardown
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_no_skip() {
        //setup
        let temp_dir = create_temp_dir();

        let file = TestableFile::new("json_simple".to_string(), &temp_dir);
        let temp_path = file.copy_to_temp();

        cli::run(vec!["padd", "fmt", "tests/spec/json", "-t", &temp_path]);

        let initially_modified_at = Path::new(&temp_path)
            .metadata()
            .unwrap()
            .modified()
            .unwrap();

        //exercise
        cli::run(vec![
            "padd",
            "fmt",
            "tests/spec/json",
            "-t",
            &temp_path,
            "--no-skip",
        ]);

        //verify
        let finally_modified_at = Path::new(&temp_path)
            .metadata()
            .unwrap()
            .modified()
            .unwrap();

        assert_ne!(initially_modified_at, finally_modified_at);

        file.assert_matches_output();

        //teardown
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_no_track() {
        //TODO(shane)
    }

    #[test]
    fn test_no_write() {
        //TODO(shane)
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
    fn test_tracking_files_skipped() {
        //TODO(shane)
    }

    #[test]
    fn test_check_formatting_passed() {
        //TODO(shane)
    }

    #[test]
    fn test_check_formatting_failed() {
        //TODO(shane)
    }

    #[test]
    fn test_start_server() {
        //TODO(shane)
    }

    #[test]
    fn test_start_daemon() {
        //TODO(shane)
    }

    #[test]
    fn test_start_daemon_already_running() {
        //TODO(shane)
    }

    #[test]
    fn test_kill_daemon() {
        //TODO(shane)
    }

    #[test]
    fn test_kill_daemon_not_running() {
        //TODO(shane)
    }

    #[test]
    fn test_format_via_daemon() {
        //TODO(shane)
    }

    #[test]
    fn test_cache_fjr() {
        //TODO(shane)
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
