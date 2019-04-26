extern crate clap;
extern crate yaml_rust;

use {
    cli::{cmd, logger},
    std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        process,
        sync::Arc,
        thread,
    },
};

use self::{clap::App, yaml_rust::yaml::Yaml};

const SERVER_ADDRESS: &str = "127.0.0.1:4774";
const SIG_KILL: &str = "kill\0";
const SIG_CHALLENGE: &str = "lost\0";
const SIG_RESPONSE: &str = "found\0";

pub fn start(clap_config: &'static Yaml) {
    logger::info(&format!("Starting padd server on {}", SERVER_ADDRESS));

    let clap_config_arc = Arc::new(clap_config);

    match TcpListener::bind(SERVER_ADDRESS) {
        Ok(listener) => {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let config = clap_config_arc.clone();
                        thread::spawn(move || {
                            handle_stream(stream, config);
                        });
                    }
                    Err(err) => {
                        logger::err(&format!("Failed to read incoming tcp stream: {}", err))
                    }
                }
            }
        }
        Err(err) => logger::fatal(&format!("Failed to bind server: {}", err)),
    };
}

fn handle_stream(stream: TcpStream, clap_config: Arc<&Yaml>) {
    match stream.try_clone() {
        Ok(mut stream_writer) => {
            let mut buf: Vec<u8> = Vec::new();
            for byte_res in stream.bytes() {
                match byte_res {
                    Ok(byte) => {
                        buf.push(byte);

                        if byte == 0 {
                            break;
                        }
                    }
                    Err(err) => {
                        logger::err(&format!("Failed to read byte from stream: {}", err));
                        return;
                    }
                }
            }

            match String::from_utf8(buf) {
                Ok(string) => match &string[..] {
                    SIG_KILL => process::exit(0),
                    SIG_CHALLENGE => {
                        if let Err(err) = stream_writer.write_all(SIG_RESPONSE.as_bytes()) {
                            logger::err(&format!("Failed to write challenge response: {}", err));
                        }
                    }
                    &_ => execute_command(string, clap_config),
                },
                Err(err) => logger::err(&format!(
                    "Failed to convert stream bytes to string: {}",
                    err
                )),
            };
        }
        Err(err) => logger::err(&format!("Failed to clone read stream for writing: {}", err)),
    }
}

pub fn kill() {
    if let Ok(mut stream) = TcpStream::connect(SERVER_ADDRESS) {
        if let Err(err) = stream.write_all(SIG_KILL.as_bytes()) {
            logger::err(&format!("Failed to write kill: {}", err));
        }
    }
}

pub fn running() -> bool {
    if let Ok(mut stream) = TcpStream::connect(SERVER_ADDRESS) {
        if let Err(err) = stream.write(SIG_CHALLENGE.as_bytes()) {
            logger::err(&format!("Failed to write challenge: {}", err));
            return false;
        }

        let mut buf = String::new();

        if let Err(err) = stream.read_to_string(&mut buf) {
            logger::err(&format!("Failed to read stream: {}", err));
            return false;
        }

        if buf == SIG_RESPONSE {
            return true;
        }
    }

    false
}

pub fn send_command(mut command: String) {
    // Add terminal character
    command.push('\0');

    match TcpStream::connect(SERVER_ADDRESS) {
        Ok(mut stream) => {
            if let Err(err) = stream.write_all(command.as_bytes()) {
                logger::err(&format!("Failed to send command: {}", err));
            }
        }
        Err(err) => logger::err(&format!("Failed to connect to padd server: {}", err)),
    }
}

fn execute_command(mut command: String, clap_config: Arc<&Yaml>) {
    logger::info(&format!("Executing command: {}", &command));

    // Remove terminal character
    command.pop();

    let args = command.split_whitespace();
    let matches = App::from_yaml(&clap_config).get_matches_from(args);

    if let Some(matches) = matches.subcommand_matches("fmt") {
        cmd::fmt(&matches);
    }
}
