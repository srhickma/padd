use {
    core::{data::Data, parse::grammar::GrammarSymbol},
    std::{error, fmt},
};

pub mod alphabet;
pub mod ecdfa;
pub mod maximal_munch;

static FAIL_SEQUENCE_LENGTH: usize = 10;

//TODO(shane) change these types to Symbol
pub trait Scanner<State: Data, Symbol: GrammarSymbol>: 'static + Send + Sync {
    fn scan(&self, input: &[char], cdfa: &CDFA<State, Symbol>)
        -> Result<Vec<Token<Symbol>>, Error>;
}

pub fn def_scanner<State: Data, Symbol: GrammarSymbol>() -> Box<Scanner<State, Symbol>> {
    Box::new(maximal_munch::MaximalMunchScanner)
}

pub trait CDFA<State: Data, Symbol: GrammarSymbol>: Send + Sync {
    fn transition(&self, state: &State, input: &[char]) -> TransitionResult<State>;
    fn has_transition(&self, state: &State, input: &[char]) -> bool;
    fn accepts(&self, state: &State) -> bool;
    fn acceptor_destination(&self, state: &State, from: &State) -> Option<State>;
    fn tokenize(&self, state: &State) -> Option<Symbol>;
    fn start(&self) -> State;
}

pub trait CDFABuilder<State: Data, Symbol: GrammarSymbol, CDFAType> {
    fn new() -> Self;
    fn build(self) -> Result<CDFAType, CDFAError>;

    fn set_alphabet(&mut self, chars: impl Iterator<Item = char>) -> &mut Self;
    fn accept(&mut self, state: &State) -> &mut Self;
    fn accept_to(
        &mut self,
        state: &State,
        from: &State,
        to: &State,
    ) -> Result<&mut Self, CDFAError>;
    fn accept_to_from_all(&mut self, state: &State, to: &State) -> Result<&mut Self, CDFAError>;
    fn mark_start(&mut self, state: &State) -> &mut Self;
    fn mark_trans(&mut self, from: &State, to: &State, on: char) -> Result<&mut Self, CDFAError>;
    fn mark_chain(
        &mut self,
        from: &State,
        to: &State,
        on: impl Iterator<Item = char>,
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
        sources: impl Iterator<Item = &'state_i &'state_o State>,
        to: &'state_o State,
        start: char,
        end: char,
    ) -> Result<&mut Self, CDFAError>;
    fn default_to(&mut self, from: &State, to: &State) -> Result<&mut Self, CDFAError>;
    fn tokenize(&mut self, state: &State, token: &Symbol) -> &mut Self;
}

pub struct TransitionResult<State> {
    state: Option<State>,
    consumed: usize,
}

impl<State> TransitionResult<State> {
    pub fn fail() -> Self {
        TransitionResult {
            state: None,
            consumed: 0,
        }
    }

    pub fn direct(state: State) -> Self {
        TransitionResult::new(state, 1)
    }

    pub fn new(state: State, consumed: usize) -> Self {
        TransitionResult {
            state: Some(state),
            consumed,
        }
    }
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
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
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

    pub fn lexeme_escaped(&self) -> String {
        self.lexeme
            .replace('\n', "\\n")
            .replace('\t', "\\t")
            .replace('\r', "\\r")
    }
}

impl<Symbol: Data> Data for Token<Symbol> {
    fn to_string(&self) -> String {
        let lexeme_string = format!(" <- '{}'", self.lexeme_escaped());

        match &self.kind {
            None => lexeme_string,
            Some(kind) => format!("{}{}", kind.to_string(), lexeme_string),
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
            self.line, self.character, self.sequence
        )
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

pub type State = String;
