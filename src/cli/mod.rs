extern crate regex;

use self::regex::Regex;
use padd::FormatJobRunner;
use std::env;
use std::io;
use std::io::Read;
use std::io::Write;
use std::io::Seek;
use std::io::SeekFrom;
use std::process;
use std::fs::File;
use std::fs::OpenOptions;
use std::fs;
use std::path::Path;

pub fn run(){
    let args: Vec<_> = env::args().collect();

    if args.len() < 2 {
        error("Missing specification file path".to_string());
    }

    let spec_path = args.get(1).unwrap();

    let fjr_res = load_spec(&spec_path);
    if fjr_res.is_err() {
        error(format!("Error loading specification {}: {}", &spec_path, fjr_res.err().unwrap()));
        return;
    }

    let fjr = fjr_res.unwrap();

    println!("Successfully loaded specification");

    let mut directory: Option<&Path> = None;
    let mut file_regex: Option<Regex> = None;
    let mut target: Option<&Path> = None;

    for i in 2..args.len() {
        let arg: &String = &args[i];
        if arg.starts_with("-") {
            match &arg[..] {
                "-d" | "--directory" => {
                    if args.len() == i + 1 {
                        error("Missing directory path".to_string());
                    }
                    directory = Some(Path::new(&args[i + 1]));
                },
                "-t" | "--target" => {
                    if args.len() == i + 1 {
                        error("Missing target path".to_string());
                    }
                    target = Some(Path::new(&args[i + 1]));
                },
                "-m" | "--matching" => {
                    if args.len() == i + 1 {
                        error("Missing file regex".to_string());
                    }
                    match Regex::new(format!(r#"{}"#, &args[i + 1]).as_str()) {
                        Ok(fn_regex) => file_regex = Some(fn_regex),
                        Err(e) => error(format!("Failed to build file name regex: {}", e)),
                    }
                },
                a => error(format!("Unrecognized parameter {}", a)),
            }
        }
    }

    match target {
        Some(target_path) => {
            if directory.is_some() {
                error("Invalid arguments: Target file and directory both specified".to_string());
            } else if file_regex.is_some() {
                error("Invalid arguments: Target file and file regex both specified".to_string());
            }
            format_file(target_path, &fjr)
        },
        None => match directory {
            Some(dir_path) => {
                let fn_regex = match file_regex {
                    Some(regex) => regex,
                    None => Regex::new(r#".*"#).unwrap(),
                };

                dir_recur(dir_path, &fn_regex, &fjr)
            },
            None => term_loop(&fjr),
        }
    }
}

fn dir_recur(dir_path: &Path, fn_regex: &Regex, fjr: &FormatJobRunner){
    fs::read_dir(dir_path).unwrap()
        .for_each(|res| {
            match res {
                Ok(dir_item) => {
                    let path = dir_item.path();
                    let file_name = path.file_name().unwrap().to_str().unwrap();
                    if path.is_dir() {
                        dir_recur(path.as_path(), fn_regex, fjr);
                    } else if fn_regex.is_match(file_name) {
                        format_file(path.as_path(), &fjr);
                    }
                },
                Err(e) => println!("An error occurred while searching directory {}: {}", dir_path.to_string_lossy(), e),
            }
        });
}

fn load_spec(spec_path: &String) -> Result<FormatJobRunner, String> {
    let mut spec = String::new();

    let spec_file = File::open(spec_path);
    match spec_file {
        Ok(_) => {
            match spec_file.unwrap().read_to_string(&mut spec) {
                Ok(_) => {},
                Err(e) => {
                    error(format!("Could't read specification file \"{}\": {}", &spec_path, e));
                },
            }
        },
        Err(e) => error(format!("Could't find specification file \"{}\": {}", &spec_path, e)),
    }

    FormatJobRunner::build(&spec)
}

fn term_loop(fjr: &FormatJobRunner){
    loop {
        let mut target_path = String::new();

        match io::stdin().read_line(&mut target_path){
            Ok(_) => {},
            Err(e) => {
                println!("Failed to read target file \"{}\": {}", target_path, e);
                continue;
            },
        }

        target_path.pop();

        format_file(&Path::new(&target_path), &fjr);
    }
}

fn format_file(target_path: &Path, fjr: &FormatJobRunner){
    print!(">> Formatting {}: ", target_path.to_string_lossy());
    let target_file = OpenOptions::new().read(true).write(true).open(&target_path);
    match target_file {
        Ok(_) => {
            let mut target = target_file.unwrap();
            let mut input = String::new();

            match target.read_to_string(&mut input) {
                Ok(_) => {},
                Err(e) => {
                    println!("Could't read target file \"{}\": {}", &target_path.to_string_lossy(), e);
                    return;
                },
            }

            match fjr.format(&input){
                Ok(res) => {
                    match target.seek(SeekFrom::Start(0)) {
                        Ok(_) => {},
                        Err(e) => {
                            println!("Couldn't seek to start of target file \"{}\": {}", &target_path.to_string_lossy(), e);
                            return;
                        },
                    }
                    match target.write_all(res.as_bytes()) {
                        Ok(_) => {println!("OK")},
                        Err(e) => {
                            println!("Couldn't write to target file \"{}\": {}", &target_path.to_string_lossy(), e);
                            return;
                        },
                    }
                },
                Err(e) => println!("Error formatting {}: {}", &target_path.to_string_lossy(), e),
            }
        },
        Err(e) => {
            println!("Could't find target file \"{}\": {}", &target_path.to_string_lossy(), e);
            return;
        },
    }
}

fn error(err_text: String){
    println!("ERROR: {}", err_text);
    println!("Usage info goes here");
    process::exit(0);
}