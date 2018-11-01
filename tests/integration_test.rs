extern crate padd;

use padd::FormatJobRunner;
use padd::Stream;
use std::io::Read;
use std::io::Write;
use std::fs::File;

#[test]
fn test_balanced_brackets() {
    test_fjr("balanced_brackets", "balanced_brackets");
}

#[test]
fn test_def_input_matcher() {
    //setup
    let spec = "
'abcdefghijklmnopqrstuvwxyz'

start 'a' -> a;

a^ACC
'a' -> fail
_ -> a;

s -> acc s `{}\\n{}`
-> acc;

acc -> ACC;
    ".to_string();

    let input = "abasdfergrthergerghera".to_string();
    let mut iter = input.chars();
    let mut getter = || iter.next();
    let mut stream = Stream::from(&mut getter);

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(&mut stream).unwrap();

    //verify
    assert_eq!(res,
               "ab
asdfergrthergergher
a"
    );
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
fn test_ignore_tokens() {
    //setup
    let spec = "
'a \n\t'

start 'a' -> a
' ' | '\n' | '\t' -> ^_;

a^ACC
'a' -> a
_ -> fail;

s -> acc s `{} {}`
-> acc;

acc -> ACC;
    ".to_string();

    let input = "aaaa \t \n  a aa  aa ".to_string();
    let mut iter = input.chars();
    let mut getter = || iter.next();
    let mut stream = Stream::from(&mut getter);

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(&mut stream).unwrap();

    //verify
    assert_eq!(res, "aaaa a aa aa");
}

#[test]
fn test_advanced_operators() {
    //setup
    let spec = "
        'inj '
        start
            'in' -> ^IN
            ' ' -> ^_
            _ -> ^ID;
        ID | IN
            ' ' -> fail
            _ -> ID;
        s
            -> x s
            -> x;
        x
            -> ID
            -> IN ``;".to_string();

    let input = "i ij ijjjijijiji inj in iii".to_string();
    let mut iter = input.chars();
    let mut getter = || iter.next();
    let mut stream = Stream::from(&mut getter);

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(&mut stream).unwrap();

    //verify
    assert_eq!(res, "iijijjjijijijiinjiii");
}

#[test]
fn test_single_reference_optional_shorthand() {
    //setup
    let spec = "
'ab'

start
  'a' -> ^A
  'b' -> ^B;

s -> A [b] s
  ->;

b -> B `\n{}\n`;
    ".to_string();

    let input = "ababaaaaababaaba".to_string();
    let mut iter = input.chars();
    let mut getter = || iter.next();
    let mut stream = Stream::from(&mut getter);

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(&mut stream).unwrap();

    //verify
    assert_eq!(res, "a\nb\na\nb\naaaaa\nb\na\nb\naa\nb\na");
}

#[test]
fn test_multiple_reference_optional_shorthand() {
    //setup
    let spec = "
'ab'

start
  'a' -> ^A
  'b' -> ^B;

s -> A [b] s
  ->;

b -> B [b] `\n{} {}`;
    ".to_string();

    let input = "abbbabaaaabbbbababaaba".to_string();
    let mut iter = input.chars();
    let mut getter = || iter.next();
    let mut stream = Stream::from(&mut getter);

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(&mut stream).unwrap();

    //verify
    assert_eq!(res, "a\nb \nb \nb a\nb aaaa\nb \nb \nb \nb a\nb a\nb aa\nb a");
}

#[test]
fn test_optional_shorthand_state_order() {
    //setup
    let spec = "
'ab'

start
  'a' -> ^A
  'b' -> ^B;

s -> [A] [B];
    ".to_string();

    let input = "ab".to_string();
    let mut iter = input.chars();
    let mut getter = || iter.next();
    let mut stream = Stream::from(&mut getter);

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(&mut stream).unwrap();

    //verify
    assert_eq!(res, "ab");
}

fn test_fjr(case_name: &str, spec_name: &str) {
    //setup
    let fjr = FormatJobRunner::build(&load_spec(spec_name)).unwrap();

    let input = load_input(case_name);
    let mut iter = input.chars();
    let mut getter = || iter.next();
    let mut stream = Stream::from(&mut getter);

    //exercise
    let res = fjr.format(&mut stream).unwrap();

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
            let mut output_file = File::create(&file_path);
            match output_file {
                Ok(_) => {
                    output_file.unwrap().write(result.as_bytes()).unwrap();
                }
                Err(e) => panic!("Couldn't create output file: {}", e),
            }
            panic!("Couldn't find output file, creating new file with results");
        }
    }

    if output != result {
        println!("EXPECTED:\n{}\nBUT FOUND:\n{}", output, result);
        panic!("Output did not match file")
    }
}
