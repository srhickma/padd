use std::fmt;
use core::data::Data;
use core::data::stream::StreamSource;
use core::data::stream::StreamConsumer;
use core::scan;

pub mod alphabet;
pub mod ecdfa;
pub mod maximal_munch;

pub trait Scanner<State: Data, Kind: Data> {
    fn scan<'a, 'b>(&self, stream: &'a mut StreamSource<char>, cdfa: &'b CDFA<State, Kind>) -> Result<Vec<Token<Kind>>, scan::Error>;
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

pub trait CDFABuilder<State, Kind> {
    fn new() -> Self;

    fn set_alphabet(&mut self, chars: impl Iterator<Item=char>) -> &mut Self;
    fn mark_accepting(&mut self, state: &State) -> &mut Self;
    fn mark_start(&mut self, state: &State) -> &mut Self;
    fn mark_trans(&mut self, from: &State, to: &State, on: char) -> &mut Self;
    fn mark_chain(&mut self, from: &State, to: &State, on: impl Iterator<Item=char>) -> &mut Self;
    fn mark_def(&mut self, from: &State, to: &State) -> &mut Self;
    fn mark_token(&mut self, state: &State, token: &Kind) -> &mut Self;
}

#[derive(PartialEq, Clone, Debug)]
pub struct Token<Kind: fmt::Debug> {
    pub kind: Kind,
    pub lexeme: String
}

impl<Kind: Data> Data for Token<Kind> {
    fn to_string(&self) -> String {
        format!("{} <- '{}'", self.kind.to_string(), self.lexeme.replace('\n', "\\n").replace('\t', "\\t"))
    }
}
