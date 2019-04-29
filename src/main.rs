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
    use super::*;

    use std::{
        fs::{self, File},
        io::Read,
        path::Path,
        process::Command,
    };

    #[test]
    fn test_fmt_all_java8() {
        let temp_dir = Path::new("tests/temp");
        let input_dir = Path::new("tests/input");
        let output_dir = Path::new("tests/output");

        let _ = fs::remove_dir_all(temp_dir);
        fs::create_dir(temp_dir).unwrap();

        fs::read_dir(input_dir)
            .unwrap()
            .map(|entry| entry.unwrap())
            .filter(|entry| !entry.path().is_dir())
            .filter(|entry| entry.file_name().to_string_lossy().starts_with("java8"))
            .for_each(|entry| {
                let file_name = entry.file_name();

                let mut temp_path_buf = temp_dir.to_path_buf();
                temp_path_buf.push(&file_name);
                let temp_path = temp_path_buf.as_path();

                let mut expected_path_buf = output_dir.to_path_buf();
                expected_path_buf.push(&file_name);
                let expected_path = expected_path_buf.as_path();

                fs::copy(entry.path(), temp_path).unwrap();

                cli::run(vec![
                    "padd",
                    "fmt",
                    "tests/spec/java8",
                    "-t",
                    &temp_path.to_string_lossy().to_string(),
                ]);

                let expected = read_to_string(expected_path);
                let actual = read_to_string(temp_path);
                if expected != actual {
                    println!("EXPECTED:\n{}\nBUT FOUND:\n{}", expected, actual);
                    panic!("Temp file did not match real file")
                }
            });

        fs::remove_dir_all(temp_dir).unwrap();
    }

    #[test]
    fn test_format_directory() {
        //TODO(shane)
    }

    #[test]
    fn test_missing_spec() {
        let output = Command::new("target/debug/padd")
            .args(&["fmt", "-t", "tests/output"])
            .output()
            .unwrap();

        let code = output.status.code().unwrap();
        assert_eq!(code, 1);

        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains(&not_provided_matcher("<SPECIFICATION>")));
    }

    #[test]
    fn test_missing_target() {
        let output = Command::new("target/debug/padd")
            .args(&["fmt", "test/spec/java8"])
            .output()
            .unwrap();

        let code = output.status.code().unwrap();
        assert_eq!(code, 1);

        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains(&not_provided_matcher("--target <PATH>")));
    }

    #[test]
    fn test_many_threads() {
        //TODO(shane)
    }

    #[test]
    fn test_invalid_threads() {
        //TODO(shane)
    }

    #[test]
    fn test_file_regex() {
        //TODO(shane)
    }

    #[test]
    fn test_no_skip() {
        //TODO(shane)
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
    fn test_diff_tracking() {
        //TODO(shane)
    }

    #[test]
    fn test_clear_tracking() {
        //TODO(shane)
    }

    #[test]
    fn test_clear_tracking_without_target() {
        let output = Command::new("target/debug/padd")
            .args(&["forget"])
            .output()
            .unwrap();

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

    fn read_to_string(path: &Path) -> String {
        let mut contents = String::new();

        File::open(path)
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();

        contents
    }

    fn not_provided_matcher(argument: &str) -> String {
        format!("error: The following required arguments were not provided:\n    {}", argument)
    }
}
