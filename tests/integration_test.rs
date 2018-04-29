extern crate padd;

use padd::FormatJobRunner;
use std::io;
use std::io::Read;
use std::io::Write;
use std::fs::File;

#[test]
fn test_balanced_brackets(){
    test_fjr("balanced_brackets", "balanced_brackets");
}

#[test]
fn test_def_input_matcher(){
    //setup
    let spec = "
'abcdefghijklmnopqrstuvwxyz'

start 'a' -> a;

a^ACC
'a' -> fail
_ -> a;

s -> acc s `{0}\\n{1}`
-> acc;

acc -> ACC;
    ".to_string();

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(&"abasdfergrthergerghera".to_string()).unwrap();

    //verify
    assert_eq!(res,
"ab
asdfergrthergergher
a"
    );
}

#[test]
fn test_lacs_simple(){
    test_fjr("lacs_simple", "lacs");
}

#[test]
fn test_lacs_medium(){
    test_fjr("lacs_medium", "lacs");
}

#[test]
fn test_lacs_complex() {
    test_fjr("lacs_complex", "lacs");
}

#[test]
fn test_ignore_tokens() {
    //setup
    let spec = "
'a \n\t'

start 'a' -> a
' \n\t' -> ws;

a^ACC
'a' -> a
_ -> fail;

ws^_;

s -> acc s `{0} {1}`
-> acc;

acc -> ACC;
    ".to_string();

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(&"aaaa \t \n  a aa  aa ".to_string()).unwrap();

    //verify
    assert_eq!(res, "aaaa a aa aa");
}

#[test]
fn test_advanced_operators() {
    //setup
    let spec = "
        'inj '
        start
            'i' -> ki
            ' ' -> ws
            _ -> ^ID;
        ki ^ID
            'n' -> ^IN;
        ID | IN | ki
            ' ' -> fail
            _ -> ID;
        ws ^_;
        s
            -> x s
            -> x;
        x
            -> ID
            -> IN ``;".to_string();

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(&"i ij ijjjijijiji inj in iii".to_string()).unwrap();

    //verify
    assert_eq!(res, "iijijjjijijijiinjiii");
}

fn test_fjr(case_name: &str, spec_name: &str){
    //setup
    let fjr = FormatJobRunner::build(&load_spec(spec_name)).unwrap();

    //exercise
    let res = fjr.format(&load_input(case_name)).unwrap();

    //verify
    assert_matches_file(res, case_name)
}

fn load_spec(name: &str) -> String {
    let mut spec = String::new();
    let spec_file = File::open(format!("tests/spec/{}", name));
    match spec_file {
        Ok(_) => {
            spec_file.unwrap().read_to_string(&mut spec);
        },
        Err(e) => panic!("Could't find specification file: {}", e),
    }
    spec
}

fn load_input(name: &str) -> String {
    let mut input = String::new();
    let input_file = File::open(format!("tests/input/{}", name));
    match input_file {
        Ok(_) => {
            input_file.unwrap().read_to_string(&mut input);
        },
        Err(e) => panic!("Could't find input file: {}", e),
    }
    input
}

fn assert_matches_file(result: String, file_name: &str){
    let file_path = format!("tests/output/{}", file_name);
    let mut output = String::new();
    let output_file = File::open(&file_path);
    match output_file {
        Ok(_) => {
            output_file.unwrap().read_to_string(&mut output);
        },
        Err(_) => {
            let mut output_file = File::create(&file_path);
            match output_file {
                Ok(_) => {
                    output_file.unwrap().write(result.as_bytes());
                },
                Err(e) => panic!("Couldn't create output file: {}", e),
            }
            panic!("Couldn't find output file, creating new file with results");
        },
    }

    assert_eq!(output, result);
}