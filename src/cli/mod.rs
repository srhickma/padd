use std::env;
use std::process;
use std::fs::File;
use std::io::prelude::*;

pub fn run(){
    let args: Vec<_> = env::args().collect();

    if args.len() < 2 {
        error("Too few parameters provided".to_string());
    }

    let target = &args[1];

    for i in 2..args.len() {
        let arg: &String = &args[i];
        if arg.starts_with("-") {
            match &arg[..] {
                "-s" | "--spec" => {
                    if args.len() == i + 1 {
                        error("Missing specification file path".to_string());
                    }
                    format_file(target, &args[i + 1]);
                },
                a => error(format!("Unrecognized parameter {}", a)),
            }
        }
    }
}

fn format_file(target_path: &String, spec_path: &String){
    let mut spec = String::new();

    let mut spec_file = File::open(spec_path);
    match spec_file {
        Ok(_) => {
            spec_file.unwrap().read_to_string(&mut spec);
        },
        Err(e) => error(format!("Could't find specification file \"{}\": {}", spec_path, e)),
    }

    let mut input = String::new();

    let mut target_file = File::open(target_path);
    match target_file {
        Ok(_) => {
            target_file.unwrap().read_to_string(&mut input);
        },
        Err(e) => error(format!("Could't find target file \"{}\": {}", target_path, e)),
    }
}

fn error(err_text: String){
    println!("{}", err_text);

    process::exit(0);
}