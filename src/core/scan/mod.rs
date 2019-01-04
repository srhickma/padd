use {
    core::{
        data::{
            Data,
            stream::{StreamConsumer, StreamSource},
        },
    },
    std::{error, fmt},
};

pub mod alphabet;
pub mod ecdfa;
pub mod maximal_munch;

static FAIL_SEQUENCE_LENGTH: usize = 10;

pub trait Scanner<State: Data, Kind: Data>: 'static + Send + Sync {
    fn scan<'a, 'b>(
        &self,
        stream: &'a mut StreamSource<char>,
        cdfa: &'b CDFA<State, Kind>,
    ) -> Result<Vec<Token<Kind>>, Error>;
}

pub fn def_scanner<State: Data, Kind: Data>() -> Box<Scanner<State, Kind>> {
    Box::new(maximal_munch::MaximalMunchScanner)
}

pub trait CDFA<State, Kind> {
    fn transition(&self, state: &State, stream: &mut StreamConsumer<char>) -> Option<State>;
    fn has_transition(&self, state: &State, stream: &mut StreamConsumer<char>) -> bool;
    fn accepts(&self, state: &State) -> bool;
    fn acceptor_destination(&self, state: &State, from: &State) -> Option<State>;
    fn tokenize(&self, state: &State) -> Option<Kind>;
    fn start(&self) -> State;
}

pub trait CDFABuilder<State, Kind, CDFAType> {
    fn new() -> Self;
    fn build(self) -> Result<CDFAType, CDFAError>;

    fn set_alphabet(&mut self, chars: impl Iterator<Item=char>) -> &mut Self;
    fn accept(&mut self, state: &State) -> &mut Self;
    fn accept_to(
        &mut self,
        state: &State,
        from: &State,
        to: &State,
    ) -> Result<&mut Self, CDFAError>;
    fn accept_to_from_all(
        &mut self,
        state: &State,
        to: &State,
    ) -> Result<&mut Self, CDFAError>;
    fn mark_start(&mut self, state: &State) -> &mut Self;
    fn mark_trans(&mut self, from: &State, to: &State, on: char) -> Result<&mut Self, CDFAError>;
    fn mark_chain(
        &mut self,
        from: &State,
        to: &State,
        on: impl Iterator<Item=char>,
    ) -> Result<&mut Self, CDFAError>;
    fn mark_range(
        &mut self,
        from: &State,
        to: &State,
        start: char,
        end: char,
    ) -> Result<&mut Self, CDFAError>;
    fn mark_range_for_all<'state_o: 'state_i, 'state_i>(
        &mut self,
        sources: impl Iterator<Item=&'state_i &'state_o State>,
        to: &'state_o State,
        start: char,
        end: char,
    ) -> Result<&mut Self, CDFAError>;
    fn default_to(&mut self, from: &State, to: &State) -> Result<&mut Self, CDFAError>;
    fn tokenize(&mut self, state: &State, token: &Kind) -> &mut Self;
}

#[derive(Debug)]
pub enum CDFAError {
    BuildErr(String),
}

impl fmt::Display for CDFAError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CDFAError::BuildErr(ref err) => write!(f, "Failed to build CDFA: {}", err),
        }
    }
}

impl error::Error for CDFAError {
    fn cause(&self) -> Option<&error::Error> {
        match *self {
            CDFAError::BuildErr(_) => None,
        }
    }
}

impl From<String> for CDFAError {
    fn from(err: String) -> CDFAError {
        CDFAError::BuildErr(err)
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct Token<Symbol: fmt::Debug> {
    kind: Option<Symbol>,
    lexeme: String,
}

impl<Symbol: Data> Token<Symbol> {
    pub fn leaf(kind: Symbol, lexeme: String) -> Self {
        Token {
            kind: Some(kind),
            lexeme,
        }
    }

    pub fn interior(kind: Symbol) -> Self {
        Token {
            kind: Some(kind),
            lexeme: String::new(),
        }
    }

    pub fn null() -> Self {
        Token {
            kind: None,
            lexeme: String::from("NULL"),
        }
    }

    pub fn is_null(&self) -> bool {
        self.kind.is_none()
    }

    pub fn kind(&self) -> &Symbol {
        self.kind.as_ref().unwrap()
    }

    pub fn lexeme(&self) -> &String {
        &self.lexeme
    }
}

impl<Symbol: Data> Data for Token<Symbol> {
    fn to_string(&self) -> String {
        let lexeme_string = format!(
            " <- '{}'",
            self.lexeme.replace('\n', "\\n").replace('\t', "\\t")
        );

        match &self.kind {
            None => lexeme_string,
            Some(kind) => format!("{}{}", kind.to_string(), lexeme_string)
        }
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
        write!(
            f,
            "No accepting scans after ({},{}): {}...",
            self.line,
            self.character,
            self.sequence
        )
    }
}

impl error::Error for Error {
    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

pub type State = String;
