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

/// The character sequence length to generate when lexing fails.
static FAIL_SEQUENCE_LENGTH: usize = 10;

/// Lexer: Trait which represents a generic lexer.
///
/// # Type Parameters:
///
/// * `State` - the state-type of the CDFA used to specify lexing behaviour.
/// * `Symbol` - the type of tokens produced by the lexer.
pub trait Lexer<State: Data, Symbol: GrammarSymbol>: 'static + Send + Sync {
    /// Lexes `input` using `cdfa` to specify the language.
    ///
    /// Returns a vector of scanned tokens if the lex is successful, otherwise an error is returned.
    fn lex(&self, input: &str, cdfa: &dyn CDFA<State, Symbol>)
        -> Result<Vec<Token<Symbol>>, Error>;
}

/// Returns the current default lexer.
/// This lexer should be used for all non-testing purposes.
pub fn def_lexer<State: Data, Symbol: GrammarSymbol>() -> Box<dyn Lexer<State, Symbol>> {
    Box::new(longest_match::LongestMatchLexer)
}

/// Compressed Deterministic Finite Automata (CDFA): Trait representing the operations a CDFA
/// implementation must provide to support use by a `Lexer`.
///
/// # Type Parameters
///
/// * `State` - the type used to represent states in the CDFA graph.
/// * `Symbol` - the type of tokens produced by the CDFA.
pub trait CDFA<State: Data, Symbol: GrammarSymbol>: Send + Sync {
    /// Attempts to perform a transition from `state` on `input`, and returns the result.
    fn transition(&self, state: &State, input: &str) -> TransitionResult<State>;

    /// Returns true if the alphabet of the CDFA contains `char`, otherwise false.
    fn alphabet_contains(&self, c: char) -> bool;

    /// Returns true if `state` is accepting, otherwise false.
    fn accepts(&self, state: &State) -> bool;

    /// Returns the default acceptor destination of `state`, or `None` if `state` has no acceptor
    /// destination.
    fn default_acceptor_destination(&self, state: &State) -> Option<State>;

    /// Returns the token kind associated with `state`, if one exists, otherwise `None`.
    fn tokenize(&self, state: &State) -> Option<Symbol>;

    /// Returns the starting state of the CDFA, where lexing should begin.
    fn start(&self) -> State;
}

/// CDFA Builder: Trait representing a builder for a CDFA.
///
/// # Type Parameters
///
/// * `State` - the type used to represent states in the CDFA graph.
/// * `Symbol` - the type of tokens produced by the CDFA.
/// * `CDFAType` - the type of CDFA this builder should produce.
pub trait CDFABuilder<State: Data, Symbol: GrammarSymbol, CDFAType> {
    /// Returns a new builder.
    fn new() -> Self;

    /// Consumes the builder and returns either a CDFA or an error, if a build failure occurred.
    fn build(self) -> Result<CDFAType, CDFAError>;

    /// Sets `chars` as the alphabet of the CDFA.
    fn set_alphabet(&mut self, chars: impl Iterator<Item = char>) -> &mut Self;

    /// Marks `state` as an accepting state.
    fn accept(&mut self, state: &State) -> &mut Self;

    /// Marks `state` as an accepting state, with acceptor destination `to`.
    fn accept_to(&mut self, state: &State, to: &State) -> &mut Self;

    /// Marks `state` as the default start state of the CDFA.
    fn mark_start(&mut self, state: &State) -> &mut Self;

    /// Adds simple transition `transit` on `on` from `from`.
    ///
    /// Returns an error if the transition could not be added.
    fn mark_trans(
        &mut self,
        from: &State,
        transit: Transit<State>,
        on: char,
    ) -> Result<&mut Self, CDFAError>;

    /// Adds chain transition `transit` on `on` from `from`.
    ///
    /// Returns an error if the transition could not be added.
    fn mark_chain(
        &mut self,
        from: &State,
        transit: Transit<State>,
        on: &str,
    ) -> Result<&mut Self, CDFAError>;

    /// Adds range transition `transit` on range [`start`, `end`] from `from`.
    ///
    /// Returns an error if the transition could not be added.
    fn mark_range(
        &mut self,
        from: &State,
        transit: Transit<State>,
        start: char,
        end: char,
    ) -> Result<&mut Self, CDFAError>;

    /// Adds range transition `transit` on range [`start`, `end`] from all states in
    /// `sources`.
    ///
    /// Returns an error if any of the transitions could not be added.
    fn mark_range_for_all<'state_o: 'state_i, 'state_i>(
        &mut self,
        sources: impl Iterator<Item = &'state_i &'state_o State>,
        transit: Transit<State>,
        start: char,
        end: char,
    ) -> Result<&mut Self, CDFAError>
    where
        State: 'state_o;

    /// Adds default transition `transit` from `from`.
    ///
    /// Returns an error if the transition could not be added.
    fn default_to(&mut self, from: &State, transit: Transit<State>)
        -> Result<&mut Self, CDFAError>;

    /// Mark that `state` should be tokenized to `token`.
    fn tokenize(&mut self, state: &State, token: &Symbol) -> &mut Self;
}

/// Transit: Represents the action-phase of a transition.
///
/// # Type Parameters
///
/// * `State` - the state type of the associated CDFA.
///
/// # Fields
///
/// * `dest` - the destination state of the transition.
/// * `consumer` - the input consumption strategy to follow when taking the transition.
/// * `acceptor_destination` - the acceptor destination associated with this transition, not to
/// be confused with the possibly different acceptor destination of the destination state.
#[derive(Clone)]
pub struct Transit<State: Data> {
    dest: State,
    consumer: ConsumerStrategy,
    acceptor_destination: Option<State>,
}

impl<State: Data> Transit<State> {
    /// Creates a new transit to `dest` which consumes all input and has no acceptor destination.
    /// This is the default behaviour, and is utilized by most transitions, hence it is included
    /// here as a shorthand instead of using the builder.
    pub fn to(dest: State) -> Self {
        Self {
            dest,
            consumer: ConsumerStrategy::All,
            acceptor_destination: None,
        }
    }
}

/// Transit Builder: Simple builder for `Transit` structs.
/// Fields and type parameters correspond exactly with those of the target type.
#[derive(Clone)]
pub struct TransitBuilder<State: Data> {
    dest: State,
    consumer: ConsumerStrategy,
    acceptor_destination: Option<State>,
}

impl<State: Data> TransitBuilder<State> {
    /// Creates a new transit builder with destination `dest`, which consumes all input and has no
    /// acceptor destination.
    pub fn to(dest: State) -> Self {
        Self {
            dest,
            consumer: ConsumerStrategy::All,
            acceptor_destination: None,
        }
    }

    /// Sets the consumer strategy of the transit to `consumer`.
    pub fn consumer(&mut self, consumer: ConsumerStrategy) -> &mut Self {
        self.consumer = consumer;
        self
    }

    /// Sets the acceptor destination of the transit to `acceptor_destination`.
    pub fn accept_to(&mut self, acceptor_destination: State) -> &mut Self {
        self.acceptor_destination = Some(acceptor_destination);
        self
    }

    /// Copies the builder configuration into a new `Transit` struct, without consuming the builder.
    pub fn build(&self) -> Transit<State> {
        Transit {
            dest: self.dest.clone(),
            consumer: self.consumer.clone(),
            acceptor_destination: self.acceptor_destination.clone(),
        }
    }
}

/// Consumer Strategy: Represents a strategy of input consumption to by taken by a CDFA transition.
///
/// # Types
///
/// * `All` - when a transition is taken, consume all input matched by the transition.
/// * `None` - when a transition is taken, do not consume any input.
#[derive(Clone)]
pub enum ConsumerStrategy {
    All,
    None,
}

/// Transition Result: Represents the result of a transition attempt.
///
/// # Types
///
/// * `Fail` - indicates that the transition was unsuccessful.
/// * `Ok` - indicates that the transition was a success, and stores the destination of the
/// transition.
///
/// # Type Parameters
///
/// * `State` - the state type of the associated CDFA.
pub enum TransitionResult<State: Data> {
    Fail,
    Ok(TransitionDestination<State>),
}

impl<State: Data> TransitionResult<State> {
    /// Returns a new successful transition result through `transit` traversing `traversed` input
    /// bytes.
    pub fn ok(transit: &Transit<State>, traversed: usize) -> Self {
        let consumed = match transit.consumer {
            ConsumerStrategy::All => traversed,
            ConsumerStrategy::None => 0,
        };

        Self::Ok(TransitionDestination {
            state: transit.dest.clone(),
            consumed,
            acceptor_destination: transit.acceptor_destination.clone(),
        })
    }
}

/// Transition Destination: Represents the destination of a successful state transition.
///
/// # Type Parameters
///
/// * `State` - the state type of the associated CDFA.
///
/// # Fields
///
/// * `state` - the destination state.
/// * `consumed` - the number of input bytes consumed by the transition.
/// * `acceptor_destination` - the optional acceptor destination of the transition, not to be
/// confused with the possibly different acceptor destination of the destination state.
pub struct TransitionDestination<State: Data> {
    state: State,
    consumed: usize,
    acceptor_destination: Option<State>,
}

/// CDFA Error: Represents an error encountered while using or constructing a CDFA.
///
/// # Types
///
/// * `BuildErr` - indicates than an error occurred while building a CDFA.
#[derive(Debug)]
pub enum CDFAError {
    BuildErr(String),
}

impl fmt::Display for CDFAError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::BuildErr(ref err) => write!(f, "Failed to build CDFA: {}", err),
        }
    }
}

impl error::Error for CDFAError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Self::BuildErr(_) => None,
        }
    }
}

impl From<String> for CDFAError {
    fn from(err: String) -> Self {
        CDFAError::BuildErr(err)
    }
}

impl From<interval::Error> for CDFAError {
    fn from(err: interval::Error) -> CDFAError {
        Self::BuildErr(format!("Range matcher error: {}", err))
    }
}

/// Token: A successfully lexed token of input.
///
/// # Type Parameters
///
/// * `Symbol` - the symbol-type of the token, as referenced by the language grammar.
///
/// # Fields
///
/// * `kind` - the kind of token, or `None` if the token represents an epsilon value (null).
/// * `lexeme` - the scanned characters which produced this token.
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct Token<Symbol: fmt::Debug> {
    kind: Option<Symbol>,
    lexeme: String,
}

// TODO(shane) try to achieve better separation between parsing and lexing logic here.
impl<Symbol: Data> Token<Symbol> {
    pub fn leaf(kind: Symbol, lexeme: String) -> Self {
        Self {
            kind: Some(kind),
            lexeme,
        }
    }

    pub fn interior(kind: Symbol) -> Self {
        Self {
            kind: Some(kind),
            lexeme: String::new(),
        }
    }

    pub fn null() -> Self {
        Self {
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
            Self::UnacceptedErr(ref err) => write!(
                f,
                "No accepting tokens after ({},{}): {}...",
                err.line, err.character, err.sequence,
            ),
            Self::AlphabetErr(c) => {
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
    fn from(err: UnacceptedError) -> Self {
        Self::UnacceptedErr(err)
    }
}

#[derive(Debug)]
pub struct UnacceptedError {
    sequence: String,
    character: usize,
    line: usize,
}
