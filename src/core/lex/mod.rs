use {
    core::{
        data::{interval, Data},
        parse::grammar::GrammarSymbol,
    },
    std::{error, fmt},
};

pub mod alphabet;
pub mod ecdfa;
pub mod longest_match;

static FAIL_SEQUENCE_LENGTH: usize = 10;

pub trait Lexer<State: Data, Symbol: GrammarSymbol>: 'static + Send + Sync {
    fn lex(
        &self,
        input: &[char],
        cdfa: &dyn CDFA<State, Symbol>,
    ) -> Result<Vec<Token<Symbol>>, Error>;
}

pub fn def_lexer<State: Data, Symbol: GrammarSymbol>() -> Box<dyn Lexer<State, Symbol>> {
    Box::new(longest_match::LongestMatchLexer)
}

pub trait CDFA<State: Data, Symbol: GrammarSymbol>: Send + Sync {
    fn transition(&self, state: &State, input: &[char]) -> TransitionResult<State>;
    fn alphabet_contains(&self, c: char) -> bool;
    fn accepts(&self, state: &State) -> bool;
    fn default_acceptor_destination(&self, state: &State) -> Option<State>;
    fn tokenize(&self, state: &State) -> Option<Symbol>;
    fn start(&self) -> State;
}

pub trait CDFABuilder<State: Data, Symbol: GrammarSymbol, CDFAType> {
    fn new() -> Self;
    fn build(self) -> Result<CDFAType, CDFAError>;
    fn set_alphabet(&mut self, chars: impl Iterator<Item = char>) -> &mut Self;
    fn accept(&mut self, state: &State) -> &mut Self;
    fn accept_to(&mut self, state: &State, to: &State) -> &mut Self;
    fn mark_start(&mut self, state: &State) -> &mut Self;
    fn mark_trans(
        &mut self,
        from: &State,
        transit: Transit<State>,
        on: char,
    ) -> Result<&mut Self, CDFAError>;
    fn mark_chain(
        &mut self,
        from: &State,
        transit: Transit<State>,
        on: impl Iterator<Item = char>,
    ) -> Result<&mut Self, CDFAError>;
    fn mark_range(
        &mut self,
        from: &State,
        transit: Transit<State>,
        start: char,
        end: char,
    ) -> Result<&mut Self, CDFAError>;
    fn mark_range_for_all<'state_o: 'state_i, 'state_i>(
        &mut self,
        sources: impl Iterator<Item = &'state_i &'state_o State>,
        transit: Transit<State>,
        start: char,
        end: char,
    ) -> Result<&mut Self, CDFAError>
    where
        State: 'state_o;
    fn default_to(&mut self, from: &State, transit: Transit<State>)
        -> Result<&mut Self, CDFAError>;
    fn tokenize(&mut self, state: &State, token: &Symbol) -> &mut Self;
}

#[derive(Clone)]
pub struct Transit<State: Data> {
    dest: State,
    consumer: ConsumerStrategy,
    acceptor_destination: Option<State>,
}

impl<State: Data> Transit<State> {
    pub fn to(dest: State) -> Self {
        Transit {
            dest,
            consumer: ConsumerStrategy::All,
            acceptor_destination: None,
        }
    }
}

#[derive(Clone)]
pub struct TransitBuilder<State: Data> {
    dest: State,
    consumer: ConsumerStrategy,
    acceptor_destination: Option<State>,
}

impl<State: Data> TransitBuilder<State> {
    pub fn to(dest: State) -> Self {
        TransitBuilder {
            dest,
            consumer: ConsumerStrategy::All,
            acceptor_destination: None,
        }
    }

    pub fn consumer(&mut self, consumer: ConsumerStrategy) -> &mut Self {
        self.consumer = consumer;
        self
    }

    pub fn accept_to(&mut self, acceptor_destination: State) -> &mut Self {
        self.acceptor_destination = Some(acceptor_destination);
        self
    }

    pub fn build(&self) -> Transit<State> {
        Transit {
            dest: self.dest.clone(),
            consumer: self.consumer.clone(),
            acceptor_destination: self.acceptor_destination.clone(),
        }
    }
}

#[derive(Clone)]
pub enum ConsumerStrategy {
    All,
    None,
}

pub enum TransitionResult<State: Data> {
    Fail,
    Ok(TransitionDestination<State>),
}

impl<State: Data> TransitionResult<State> {
    pub fn direct(transit: &Transit<State>) -> Self {
        TransitionResult::ok(transit, 1)
    }

    pub fn ok(transit: &Transit<State>, traversed: usize) -> Self {
        let consumed = match transit.consumer {
            ConsumerStrategy::All => traversed,
            ConsumerStrategy::None => 0,
        };

        TransitionResult::Ok(TransitionDestination {
            state: transit.dest.clone(),
            consumed,
            acceptor_destination: transit.acceptor_destination.clone(),
        })
    }
}

pub struct TransitionDestination<State: Data> {
    state: State,
    consumed: usize,
    acceptor_destination: Option<State>,
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

impl From<interval::Error> for CDFAError {
    fn from(err: interval::Error) -> CDFAError {
        CDFAError::BuildErr(format!("Range matcher error: {}", err))
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

    pub fn kind_opt(&self) -> &Option<Symbol> {
        &self.kind
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
pub enum Error {
    UnacceptedErr(UnacceptedError),
    AlphabetErr(char),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::UnacceptedErr(ref err) => write!(
                f,
                "No accepting tokens after ({},{}): {}...",
                err.line, err.character, err.sequence,
            ),
            Error::AlphabetErr(c) => {
                write!(f, "Consuming character outside lexer alphabet: '{}'", c,)
            }
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

impl From<UnacceptedError> for Error {
    fn from(err: UnacceptedError) -> Error {
        Error::UnacceptedErr(err)
    }
}

#[derive(Debug)]
pub struct UnacceptedError {
    sequence: String,
    character: usize,
    line: usize,
}
