use {
    core::{
        data::{
            Data,
            stream::{StreamConsumer, StreamSource},
        },
        scan::{self, Token},
    },
    std::{error, fmt},
};

pub mod alphabet;
pub mod ecdfa;
pub mod maximal_munch;

pub trait Scanner<State: Data, Kind: Data>: 'static + Send + Sync {
    fn scan<'a, 'b>(
        &self,
        stream: &'a mut StreamSource<char>,
        cdfa: &'b CDFA<State, Kind>,
    ) -> Result<Vec<Token<Kind>>, scan::Error>;
}

pub fn def_scanner<State: Data, Kind: Data>() -> Box<Scanner<State, Kind>> {
    Box::new(maximal_munch::MaximalMunchScanner)
}

pub trait CDFA<State, Kind> {
    fn transition(&self, state: &State, stream: &mut StreamConsumer<char>) -> Option<State>;
    fn has_transition(&self, state: &State, stream: &mut StreamConsumer<char>) -> bool;
    fn accepts(&self, state: &State) -> bool;
    fn tokenize(&self, state: &State) -> Option<Kind>;
    fn start(&self) -> State;
}

pub trait CDFABuilder<State, Kind, CDFAType> {
    fn new() -> Self;
    fn build(self) -> Result<CDFAType, CDFAError>;

    fn set_alphabet(&mut self, chars: impl Iterator<Item=char>) -> &mut Self;
    fn mark_accepting(&mut self, state: &State) -> &mut Self;
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
    fn mark_def(&mut self, from: &State, to: &State) -> Result<&mut Self, CDFAError>;
    fn mark_token(&mut self, state: &State, token: &Kind) -> &mut Self;
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
