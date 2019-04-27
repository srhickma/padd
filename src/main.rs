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

    fn read_to_string(path: &Path) -> String {
        let mut contents = String::new();

        File::open(path)
            .unwrap()
            .read_to_string(&mut contents)
            .unwrap();

        contents
    }
}
