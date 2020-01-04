extern crate colored;
extern crate difference;
extern crate padd;

use {
    colored::Colorize,
    difference::{Changeset, Difference},
    padd::{FormatJob, FormatJobRunner},
    std::{
        fs::File,
        io::{Read, Write},
    },
};

#[test]
fn test_balanced_brackets() {
    test_fjr("balanced_brackets", "balanced_brackets");
}

#[test]
fn test_lacs_simple() {
    test_fjr("lacs_simple", "lacs");
}

#[test]
fn test_lacs_medium() {
    test_fjr("lacs_medium", "lacs");
}

#[test]
fn test_lacs_complex() {
    test_fjr("lacs_complex", "lacs");
}

#[test]
fn test_java8_simple() {
    test_fjr("java8_simple", "java8");
}

#[test]
fn test_java8_medium() {
    test_fjr("java8_medium", "java8");
}

#[test]
fn test_java8_complex_spring() {
    test_fjr("java8_complex_spring", "java8");
}

#[test]
fn test_java8_complex_guice() {
    test_fjr("java8_complex_guice", "java8");
}

#[test]
fn test_java8_concepts() {
    test_fjr("java8_concepts", "java8");
}

#[test]
fn test_java8_annotation() {
    test_fjr("java8_annotation", "java8");
}

#[test]
fn test_java8_interface() {
    test_fjr("java8_interface", "java8");
}

#[test]
fn test_java8_inline() {
    test_fjr("java8_inline", "java8");
}

#[test]
fn test_java8_line_comments() {
    test_fjr("java8_line_comments", "java8");
}

#[test]
fn test_java8_block_comments() {
    test_fjr("java8_block_comments", "java8");
}

#[test]
fn test_json_simple() {
    test_fjr("json_simple", "json");
}

#[test]
fn test_json_complex() {
    test_fjr("json_complex", "json");
}

#[test]
fn tw_lines() {
    test_fjr("tw_lines", "trailing_whitespace");
}

#[test]
fn tw_no_newline() {
    test_fjr("tw_no_newline", "trailing_whitespace");
}

#[test]
fn tw_only_whitespace() {
    test_fjr("tw_only_whitespace", "trailing_whitespace");
}

// TODO enable when ready
// #[test]
// fn line_breaker_test() {
//     test_fjr("line_breaker_test", "line_breaker");
// }

fn test_fjr(case_name: &str, spec_name: &str) {
    //setup
    let fjr = FormatJobRunner::build(&load_spec(spec_name)).unwrap();

    let input = load_input(case_name);

    //exercise
    let res = fjr.format(FormatJob::from_text(input)).unwrap();

    //verify
    assert_matches_file(res, case_name)
}

fn load_spec(name: &str) -> String {
    let mut spec = String::new();
    let spec_file = File::open(format!("tests/spec/{}", name));
    match spec_file {
        Ok(_) => {
            spec_file.unwrap().read_to_string(&mut spec).unwrap();
        }
        Err(e) => panic!("Could't find specification file: {}", e),
    }
    spec
}

fn load_input(name: &str) -> String {
    let mut input = String::new();
    let input_file = File::open(format!("tests/input/{}", name));
    match input_file {
        Ok(_) => {
            input_file.unwrap().read_to_string(&mut input).unwrap();
        }
        Err(e) => panic!("Could't find input file: {}", e),
    }
    input
}

fn assert_matches_file(result: String, file_name: &str) {
    let file_path = format!("tests/output/{}", file_name);
    let mut output = String::new();
    let output_file = File::open(&file_path);
    match output_file {
        Ok(_) => {
            output_file.unwrap().read_to_string(&mut output).unwrap();
        }
        Err(_) => {
            let output_file = File::create(&file_path);
            match output_file {
                Ok(_) => {
                    output_file.unwrap().write(result.as_bytes()).unwrap();
                }
                Err(e) => panic!("Couldn't create output file: {}", e),
            }
            panic!("Couldn't find output file, creating new file with results");
        }
    }

    let change_set = Changeset::new(&output, &result, "\n");
    if change_set.distance != 0 {
        print_pretty_diff(&change_set);
        panic!("Output did not match file");
    }
}

fn print_pretty_diff(change_set: &Changeset) {
    for diff in &change_set.diffs {
        match diff {
            Difference::Same(string) => {
                for line in string.split("\n") {
                    println!(" |{}", line);
                }
            }
            Difference::Rem(string) => {
                for line in string.split("\n") {
                    println!("{}", format!("-|{}", line).bright_red());
                }
            }
            Difference::Add(string) => {
                for line in string.split("\n") {
                    println!("{}", format!("+|{}", line).bright_green());
                }
            }
        };
    }
}
