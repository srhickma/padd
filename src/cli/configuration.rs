extern crate serde;
extern crate serde_yaml;

use {
    self::serde::Deserialize,
    std::{
        collections::HashMap,
        error, fmt,
        fs::{self, File, OpenOptions},
        io::{Read, Seek, SeekFrom},
        path::Path,
    },
};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Configuration {
    pub flags: Option<Flags>,
    pub groups: HashMap<String, MatcherGroup>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct MatcherGroup {
    pub flags: Option<Flags>,
    pub pipe: Vec<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Flags {
    pub target: Option<String>,
    pub threads: Option<usize>,
    pub no_skip: Option<bool>,
    pub no_track: Option<bool>,
    pub no_write: Option<bool>,
    pub check: Option<bool>,
}

pub fn read_configuration(path: &Path) -> Result<Configuration, ConfigurationError> {
    let mut conf_str = String::new();

    match File::open(path) {
        Ok(mut file) => {
            if let Err(err) = file.read_to_string(&mut conf_str) {
                return Err(ConfigurationError::IOErr(format!(
                    "Could not read configuration file \"{}\": {}",
                    &path.to_string_lossy().to_string(),
                    err
                )));
            }
        }
        Err(err) => {
            return Err(ConfigurationError::IOErr(format!(
                "Could not find configuration file \"{}\": {}",
                &path.to_string_lossy().to_string(),
                err
            )));
        }
    }

    Ok(serde_yaml::from_str(&conf_str[..])?)
}

pub enum ConfigurationError {
    IOErr(String),
    DeserializationErr(serde_yaml::Error),
}

impl fmt::Display for ConfigurationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ConfigurationError::IOErr(ref err) => write!(f, "IO Error: {}", err),
            ConfigurationError::DeserializationErr(ref err) => {
                write!(f, "Failed to parse configuration file: {}", err)
            }
        }
    }
}

impl From<serde_yaml::Error> for ConfigurationError {
    fn from(err: serde_yaml::Error) -> Self {
        ConfigurationError::DeserializationErr(err)
    }
}
