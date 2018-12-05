use {
    core::data::Data,
    std::{error, fmt},
};

pub mod compile;
pub mod runtime;

static FAIL_SEQUENCE_LENGTH: usize = 10;

#[derive(PartialEq, Clone, Debug)]
pub struct Token<Kind: fmt::Debug> {
    pub kind: Kind,
    pub lexeme: String,
}

impl<Kind: Data> Data for Token<Kind> {
    fn to_string(&self) -> String {
        format!("{} <- '{}'", self.kind.to_string(), self.lexeme.replace('\n', "\\n").replace('\t', "\\t"))
    }
}

#[derive(Debug)]
pub struct Error {
    sequence: String,
    character: usize,
    line: usize,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "No accepting scans after ({},{}): {}...", self.line, self.character, self.sequence)
    }
}

impl error::Error for Error {
    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

pub type Kind = String;
pub type State = String;
