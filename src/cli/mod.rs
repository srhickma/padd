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

pub fn run(){
    let args: Vec<_> = env::args().collect();

    if args.len() < 2 {
        error("Missing specification file path".to_string());
    }

//    for i in 1..args.len() {
//        let arg: &String = &args[i];
//        if arg.starts_with("-") {
//            match &arg[..] {
//                "-s" | "--spec" => {
//                    if args.len() == i + 1 {
//                        error("Missing specificat*ion file path".to_string());
//                    }
//                    load_spec(&args[i + 1]);
//                },
//                a => error(format!("Unrecognized parameter {}", a)),
//            }
//        }
//    }

    let spec_path = args.get(1).unwrap();

    let fjr_res = load_spec(&spec_path);
    if fjr_res.is_err() {
        error(format!("Error loading specification {}: {}", &spec_path, fjr_res.err().unwrap()));
        return;
    }

    let fjr = fjr_res.unwrap();

    println!("Successfully loaded specification");

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

        format_file(&target_path, &fjr);
    }
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
        Err(e) => error(format!("Could't find specification file \"{}\": {}", spec_path, e)),
    }

    FormatJobRunner::build(&spec)
}

fn format_file(target_path: &String, fjr: &FormatJobRunner){
    let target_file = OpenOptions::new().read(true).write(true).open(&target_path);
    match target_file {
        Ok(_) => {
            let mut target = target_file.unwrap();
            let mut input = String::new();

            match target.read_to_string(&mut input) {
                Ok(_) => {},
                Err(e) => {
                    println!("Could't read target file \"{}\": {}", &target_path, e);
                    return;
                },
            }

            match fjr.format(&input){
                Ok(res) => {
                    match target.seek(SeekFrom::Start(0)) {
                        Ok(_) => {},
                        Err(e) => {
                            println!("Couldn't seek to start of target file \"{}\": {}", &target_path, e);
                            return;
                        },
                    }
                    match target.write_all(res.as_bytes()) {
                        Ok(_) => {},
                        Err(e) => {
                            println!("Couldn't write to target file \"{}\": {}", &target_path, e);
                            return;
                        },
                    }
                },
                Err(e) => println!("Error formatting {}: {}", &target_path, e),
            }
        },
        Err(e) => {
            println!("Could't find target file \"{}\": {}", &target_path, e);
            return;
        },
    }
}

fn error(err_text: String){
    println!("{}", err_text);
    println!("Usage info goes here");
    process::exit(0);
}