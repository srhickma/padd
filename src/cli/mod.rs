use padd::FormatJobRunner;
use std::env;
use std::io;
use std::io::Read;
use std::process;
use std::fs::File;

pub fn run(){
    let args: Vec<_> = env::args().collect();

    for i in 1..args.len() {
        let arg: &String = &args[i];
        if arg.starts_with("-") {
            match &arg[..] {
                "-s" | "--spec" => {
                    if args.len() == i + 1 {
                        error("Missing specification file path".to_string());
                    }
                    load_spec(&args[i + 1]);
                },
                a => error(format!("Unrecognized parameter {}", a)),
            }
        }
    }
}

fn load_spec(spec_path: &String) {
    let mut spec = String::new();

    let spec_file = File::open(spec_path);
    match spec_file {
        Ok(_) => {
            spec_file.unwrap().read_to_string(&mut spec);
        },
        Err(e) => error(format!("Could't find specification file \"{}\": {}", spec_path, e)),
    }

    let fjr = FormatJobRunner::build(&spec);

    println!("Successfully loaded specification");

    loop {
        let mut target_path = String::new();

        match io::stdin().read_line(&mut target_path){
            Ok(_) => {},
            Err(e) => {
                println!("Failed to read target file path");
                continue;
            },
        }

        target_path.pop();

        let mut input = String::new();

        let mut target_file = File::open(&target_path);
        match target_file {
            Ok(_) => {
                target_file.unwrap().read_to_string(&mut input);
            },
            Err(e) => {
                println!("Could't find target file \"{}\": {}", &target_path, e);
                continue;
            },
        }

        match fjr.format(&input){
            Ok(res) => println!("{}", res),
            Err(e) => println!("{}", e),
        }
    }
}

fn error(err_text: String){
    println!("{}", err_text);
    println!("Usage info goes here");
    process::exit(0);
}