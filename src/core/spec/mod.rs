use {
    core::{
        fmt::{self, Formatter},
        lex::{self, CDFA},
        parse::{
            self,
            grammar::{self, Grammar, GrammarBuilder, GrammarSymbol},
            Tree,
        },
        spec::lang::SpecSymbol,
    },
    std::{self, error},
};

mod gen;
mod lang;
mod region;

lazy_static! {
    /// The default transition matcher for CDFA specifications.
    pub static ref DEF_MATCHER: String = String::from("_");
}

/// Spec Gen Result: Stores the CDFA, grammar, and formatter produced during specification
/// generation.
type SpecGenResult<Symbol> = (
    Box<dyn CDFA<usize, Symbol>>,
    Box<dyn Grammar<Symbol>>,
    Formatter<Symbol>,
);

/// Parses a specification from the `input` string, returning the result or an error if `input` is
/// not a valid specification.
pub fn parse_spec(input: &str) -> Result<Tree<SpecSymbol>, ParseError> {
    lang::parse_spec(input)
}

/// Generates a specification from a parse tree, returning the result or an error if `parse` does
/// not represent a valid specification.
///
/// # Parameters
///
/// * `parse` - the specification parse tree.
/// * `grammar_builder` - a builder for the specification grammar.
pub fn generate_spec<Symbol: 'static + GrammarSymbol, GrammarType, GrammarBuilderType>(
    parse: &Tree<SpecSymbol>,
    grammar_builder: GrammarBuilderType,
) -> Result<SpecGenResult<Symbol>, GenError>
where
    GrammarType: 'static + Grammar<Symbol>,
    GrammarBuilderType: GrammarBuilder<String, Symbol, GrammarType>,
{
    gen::generate_spec(parse, grammar_builder)
}

/// Gen Error: Represents and error encountered while generating a specification.
///
/// # Types
///
/// * `MatcherErr` - indicates an error in a CDFA transition matcher definition.
/// * `MappingErr` - indicates an error in the CDFA to grammar symbol mapping.
/// * `CDFAErr` - indicates an internal error encountered while building a CDFA.
/// * `FormatterErr` - indicates an internal error encountered while building a formatter.
/// * `GrammarBuildErr` - indicates an internal error encountered while building a grammar.
/// * `RegionErr` - indicates and error encountered while traversing specification regions.
#[derive(Debug)]
pub enum GenError {
    MatcherErr(String),
    MappingErr(String),
    CDFAErr(lex::CDFAError),
    FormatterErr(fmt::BuildError),
    GrammarBuildErr(grammar::BuildError),
    RegionErr(region::Error),
}

impl std::fmt::Display for GenError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Self::MatcherErr(ref err) => write!(f, "Matcher definition error: {}", err),
            Self::MappingErr(ref err) => write!(f, "ECDFA to grammar mapping error: {}", err),
            Self::CDFAErr(ref err) => write!(f, "ECDFA generation error: {}", err),
            Self::FormatterErr(ref err) => write!(f, "Formatter build error: {}", err),
            Self::GrammarBuildErr(ref err) => write!(f, "Grammar build error: {}", err),
            Self::RegionErr(ref err) => write!(f, "Region error: {}", err),
        }
    }
}

impl error::Error for GenError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Self::MatcherErr(_) => None,
            Self::MappingErr(_) => None,
            Self::CDFAErr(ref err) => Some(err),
            Self::FormatterErr(ref err) => Some(err),
            Self::GrammarBuildErr(ref err) => Some(err),
            Self::RegionErr(ref err) => Some(err),
        }
    }
}

impl From<lex::CDFAError> for GenError {
    fn from(err: lex::CDFAError) -> Self {
        Self::CDFAErr(err)
    }
}

impl From<grammar::BuildError> for GenError {
    fn from(err: grammar::BuildError) -> Self {
        Self::GrammarBuildErr(err)
    }
}

impl From<fmt::BuildError> for GenError {
    fn from(err: fmt::BuildError) -> Self {
        Self::FormatterErr(err)
    }
}

impl From<region::Error> for GenError {
    fn from(err: region::Error) -> Self {
        Self::RegionErr(err)
    }
}

/// Parse Error: Represents an error encountered while parsing a specification.
///
/// # Types
///
/// * `LexErr` - indicates a syntactic error.
/// * `ParseErr` - indicates a semantic error.
#[derive(Debug)]
pub enum ParseError {
    LexErr(lex::Error),
    ParseErr(parse::Error),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Self::LexErr(ref err) => write!(f, "Lex error: {}", err),
            Self::ParseErr(ref err) => write!(f, "Parse error: {}", err),
        }
    }
}

impl error::Error for ParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Self::LexErr(ref err) => Some(err),
            Self::ParseErr(ref err) => Some(err),
        }
    }
}

impl From<lex::Error> for ParseError {
    fn from(err: lex::Error) -> ParseError {
        ParseError::LexErr(err)
    }
}

impl From<parse::Error> for ParseError {
    fn from(err: parse::Error) -> Self {
        Self::ParseErr(err)
    }
}

#[cfg(test)]
mod tests {
    use core::{data::Data, lex::Token, parse::grammar::SimpleGrammarBuilder};

    use super::*;

    #[test]
    fn parse_spec_spaces() {
        //setup
        let input = "alphabet' 'cdfa{start;}grammar{s|s b;}";

        //exercise
        let tree = lang::parse_spec(input).unwrap();

        //verify
        assert_eq!(
            tree.to_string(),
            "└── Spec
    └── Regions
        ├── Regions
        │   ├── Regions
        │   │   └── Region
        │   │       └── Alphabet
        │   │           ├── TAlphabet <- 'alphabet'
        │   │           └── TCil <- '' ''
        │   └── Region
        │       └── CDFA
        │           ├── TCDFA <- 'cdfa'
        │           ├── TLeftBrace <- '{'
        │           ├── States
        │           │   └── State
        │           │       ├── StateDeclarator
        │           │       │   └── Targets
        │           │       │       └── TId <- 'start'
        │           │       ├── TransitionsOpt
        │           │       │   └──  <- 'NULL'
        │           │       └── TSemi <- ';'
        │           └── TRightBrace <- '}'
        └── Region
            └── Grammar
                ├── TGrammar <- 'grammar'
                ├── TLeftBrace <- '{'
                ├── Productions
                │   └── Production
                │       ├── TId <- 's'
                │       ├── PatternOpt
                │       │   └──  <- 'NULL'
                │       ├── RightHandSides
                │       │   └── RightHandSide
                │       │       ├── TOr <- '|'
                │       │       ├── Ids
                │       │       │   ├── Ids
                │       │       │   │   ├── Ids
                │       │       │   │   │   └──  <- 'NULL'
                │       │       │   │   └── TId <- 's'
                │       │       │   └── TId <- 'b'
                │       │       └── PatternOpt
                │       │           └──  <- 'NULL'
                │       └── TSemi <- ';'
                └── TRightBrace <- '}'"
        );
    }

    #[test]
    fn parse_spec_simple() {
        //setup
        let input = "
alphabet ' \t\n{}'

cdfa {
    start
        ' ' -> ws
        '\t' -> ws
        '\n' -> ws
        '{' -> lbr
        '}' -> rbr;

    ws  ^WHITESPACE
        ' ' -> ws
        '\t' -> ws
        '\n' -> ws;

    lbr ^LBRACKET;

    rbr ^RBRACKET;
}

grammar {
    s
        | s b
        |;
    b
        | LBRACKET s RBRACKET ``
        | w;

    w | WHITESPACE `[prefix]{0}\n\n{1;prefix=[prefix]\t}[prefix]{2}\n\n`;
}
        ";

        //exercise
        let tree = lang::parse_spec(input).unwrap();

        //verify
        assert_eq!(tree.to_string(),
                   "└── Spec
    └── Regions
        ├── Regions
        │   ├── Regions
        │   │   └── Region
        │   │       └── Alphabet
        │   │           ├── TAlphabet <- 'alphabet'
        │   │           └── TCil <- '' \\t\\n{}''
        │   └── Region
        │       └── CDFA
        │           ├── TCDFA <- 'cdfa'
        │           ├── TLeftBrace <- '{'
        │           ├── States
        │           │   ├── States
        │           │   │   ├── States
        │           │   │   │   ├── States
        │           │   │   │   │   └── State
        │           │   │   │   │       ├── StateDeclarator
        │           │   │   │   │       │   └── Targets
        │           │   │   │   │       │       └── TId <- 'start'
        │           │   │   │   │       ├── TransitionsOpt
        │           │   │   │   │       │   └── Transitions
        │           │   │   │   │       │       ├── Transitions
        │           │   │   │   │       │       │   ├── Transitions
        │           │   │   │   │       │       │   │   ├── Transitions
        │           │   │   │   │       │       │   │   │   ├── Transitions
        │           │   │   │   │       │       │   │   │   │   └── Transition
        │           │   │   │   │       │       │   │   │   │       ├── TransitionPattern
        │           │   │   │   │       │       │   │   │   │       │   └── Matchers
        │           │   │   │   │       │       │   │   │   │       │       └── Matcher
        │           │   │   │   │       │       │   │   │   │       │           └── TCil <- '' ''
        │           │   │   │   │       │       │   │   │   │       ├── TransitionMethod
        │           │   │   │   │       │       │   │   │   │       │   └── TArrow <- '->'
        │           │   │   │   │       │       │   │   │   │       └── TransitionDestination
        │           │   │   │   │       │       │   │   │   │           └── TId <- 'ws'
        │           │   │   │   │       │       │   │   │   └── Transition
        │           │   │   │   │       │       │   │   │       ├── TransitionPattern
        │           │   │   │   │       │       │   │   │       │   └── Matchers
        │           │   │   │   │       │       │   │   │       │       └── Matcher
        │           │   │   │   │       │       │   │   │       │           └── TCil <- ''\\t''
        │           │   │   │   │       │       │   │   │       ├── TransitionMethod
        │           │   │   │   │       │       │   │   │       │   └── TArrow <- '->'
        │           │   │   │   │       │       │   │   │       └── TransitionDestination
        │           │   │   │   │       │       │   │   │           └── TId <- 'ws'
        │           │   │   │   │       │       │   │   └── Transition
        │           │   │   │   │       │       │   │       ├── TransitionPattern
        │           │   │   │   │       │       │   │       │   └── Matchers
        │           │   │   │   │       │       │   │       │       └── Matcher
        │           │   │   │   │       │       │   │       │           └── TCil <- ''\\n''
        │           │   │   │   │       │       │   │       ├── TransitionMethod
        │           │   │   │   │       │       │   │       │   └── TArrow <- '->'
        │           │   │   │   │       │       │   │       └── TransitionDestination
        │           │   │   │   │       │       │   │           └── TId <- 'ws'
        │           │   │   │   │       │       │   └── Transition
        │           │   │   │   │       │       │       ├── TransitionPattern
        │           │   │   │   │       │       │       │   └── Matchers
        │           │   │   │   │       │       │       │       └── Matcher
        │           │   │   │   │       │       │       │           └── TCil <- ''{''
        │           │   │   │   │       │       │       ├── TransitionMethod
        │           │   │   │   │       │       │       │   └── TArrow <- '->'
        │           │   │   │   │       │       │       └── TransitionDestination
        │           │   │   │   │       │       │           └── TId <- 'lbr'
        │           │   │   │   │       │       └── Transition
        │           │   │   │   │       │           ├── TransitionPattern
        │           │   │   │   │       │           │   └── Matchers
        │           │   │   │   │       │           │       └── Matcher
        │           │   │   │   │       │           │           └── TCil <- ''}''
        │           │   │   │   │       │           ├── TransitionMethod
        │           │   │   │   │       │           │   └── TArrow <- '->'
        │           │   │   │   │       │           └── TransitionDestination
        │           │   │   │   │       │               └── TId <- 'rbr'
        │           │   │   │   │       └── TSemi <- ';'
        │           │   │   │   └── State
        │           │   │   │       ├── StateDeclarator
        │           │   │   │       │   ├── Targets
        │           │   │   │       │   │   └── TId <- 'ws'
        │           │   │   │       │   └── Acceptor
        │           │   │   │       │       ├── THat <- '^'
        │           │   │   │       │       ├── IdOrDef
        │           │   │   │       │       │   └── TId <- 'WHITESPACE'
        │           │   │   │       │       └── AcceptorDestinationOpt
        │           │   │   │       │           └──  <- 'NULL'
        │           │   │   │       ├── TransitionsOpt
        │           │   │   │       │   └── Transitions
        │           │   │   │       │       ├── Transitions
        │           │   │   │       │       │   ├── Transitions
        │           │   │   │       │       │   │   └── Transition
        │           │   │   │       │       │   │       ├── TransitionPattern
        │           │   │   │       │       │   │       │   └── Matchers
        │           │   │   │       │       │   │       │       └── Matcher
        │           │   │   │       │       │   │       │           └── TCil <- '' ''
        │           │   │   │       │       │   │       ├── TransitionMethod
        │           │   │   │       │       │   │       │   └── TArrow <- '->'
        │           │   │   │       │       │   │       └── TransitionDestination
        │           │   │   │       │       │   │           └── TId <- 'ws'
        │           │   │   │       │       │   └── Transition
        │           │   │   │       │       │       ├── TransitionPattern
        │           │   │   │       │       │       │   └── Matchers
        │           │   │   │       │       │       │       └── Matcher
        │           │   │   │       │       │       │           └── TCil <- ''\\t''
        │           │   │   │       │       │       ├── TransitionMethod
        │           │   │   │       │       │       │   └── TArrow <- '->'
        │           │   │   │       │       │       └── TransitionDestination
        │           │   │   │       │       │           └── TId <- 'ws'
        │           │   │   │       │       └── Transition
        │           │   │   │       │           ├── TransitionPattern
        │           │   │   │       │           │   └── Matchers
        │           │   │   │       │           │       └── Matcher
        │           │   │   │       │           │           └── TCil <- ''\\n''
        │           │   │   │       │           ├── TransitionMethod
        │           │   │   │       │           │   └── TArrow <- '->'
        │           │   │   │       │           └── TransitionDestination
        │           │   │   │       │               └── TId <- 'ws'
        │           │   │   │       └── TSemi <- ';'
        │           │   │   └── State
        │           │   │       ├── StateDeclarator
        │           │   │       │   ├── Targets
        │           │   │       │   │   └── TId <- 'lbr'
        │           │   │       │   └── Acceptor
        │           │   │       │       ├── THat <- '^'
        │           │   │       │       ├── IdOrDef
        │           │   │       │       │   └── TId <- 'LBRACKET'
        │           │   │       │       └── AcceptorDestinationOpt
        │           │   │       │           └──  <- 'NULL'
        │           │   │       ├── TransitionsOpt
        │           │   │       │   └──  <- 'NULL'
        │           │   │       └── TSemi <- ';'
        │           │   └── State
        │           │       ├── StateDeclarator
        │           │       │   ├── Targets
        │           │       │   │   └── TId <- 'rbr'
        │           │       │   └── Acceptor
        │           │       │       ├── THat <- '^'
        │           │       │       ├── IdOrDef
        │           │       │       │   └── TId <- 'RBRACKET'
        │           │       │       └── AcceptorDestinationOpt
        │           │       │           └──  <- 'NULL'
        │           │       ├── TransitionsOpt
        │           │       │   └──  <- 'NULL'
        │           │       └── TSemi <- ';'
        │           └── TRightBrace <- '}'
        └── Region
            └── Grammar
                ├── TGrammar <- 'grammar'
                ├── TLeftBrace <- '{'
                ├── Productions
                │   ├── Productions
                │   │   ├── Productions
                │   │   │   └── Production
                │   │   │       ├── TId <- 's'
                │   │   │       ├── PatternOpt
                │   │   │       │   └──  <- 'NULL'
                │   │   │       ├── RightHandSides
                │   │   │       │   ├── RightHandSides
                │   │   │       │   │   └── RightHandSide
                │   │   │       │   │       ├── TOr <- '|'
                │   │   │       │   │       ├── Ids
                │   │   │       │   │       │   ├── Ids
                │   │   │       │   │       │   │   ├── Ids
                │   │   │       │   │       │   │   │   └──  <- 'NULL'
                │   │   │       │   │       │   │   └── TId <- 's'
                │   │   │       │   │       │   └── TId <- 'b'
                │   │   │       │   │       └── PatternOpt
                │   │   │       │   │           └──  <- 'NULL'
                │   │   │       │   └── RightHandSide
                │   │   │       │       ├── TOr <- '|'
                │   │   │       │       ├── Ids
                │   │   │       │       │   └──  <- 'NULL'
                │   │   │       │       └── PatternOpt
                │   │   │       │           └──  <- 'NULL'
                │   │   │       └── TSemi <- ';'
                │   │   └── Production
                │   │       ├── TId <- 'b'
                │   │       ├── PatternOpt
                │   │       │   └──  <- 'NULL'
                │   │       ├── RightHandSides
                │   │       │   ├── RightHandSides
                │   │       │   │   └── RightHandSide
                │   │       │   │       ├── TOr <- '|'
                │   │       │   │       ├── Ids
                │   │       │   │       │   ├── Ids
                │   │       │   │       │   │   ├── Ids
                │   │       │   │       │   │   │   ├── Ids
                │   │       │   │       │   │   │   │   └──  <- 'NULL'
                │   │       │   │       │   │   │   └── TId <- 'LBRACKET'
                │   │       │   │       │   │   └── TId <- 's'
                │   │       │   │       │   └── TId <- 'RBRACKET'
                │   │       │   │       └── PatternOpt
                │   │       │   │           └── TPattern <- '``'
                │   │       │   └── RightHandSide
                │   │       │       ├── TOr <- '|'
                │   │       │       ├── Ids
                │   │       │       │   ├── Ids
                │   │       │       │   │   └──  <- 'NULL'
                │   │       │       │   └── TId <- 'w'
                │   │       │       └── PatternOpt
                │   │       │           └──  <- 'NULL'
                │   │       └── TSemi <- ';'
                │   └── Production
                │       ├── TId <- 'w'
                │       ├── PatternOpt
                │       │   └──  <- 'NULL'
                │       ├── RightHandSides
                │       │   └── RightHandSide
                │       │       ├── TOr <- '|'
                │       │       ├── Ids
                │       │       │   ├── Ids
                │       │       │   │   └──  <- 'NULL'
                │       │       │   └── TId <- 'WHITESPACE'
                │       │       └── PatternOpt
                │       │           └── TPattern <- '`[prefix]{0}\\n\\n{1;prefix=[prefix]\\t}[prefix]{2}\\n\\n`'
                │       └── TSemi <- ';'
                └── TRightBrace <- '}'"
        );
    }

    #[test]
    fn parse_spec_complex() {
        //setup
        let input = "
alphabet 'vinj '

cdfa {
    start
        'i' -> ki
        _ -> ^ID;

    ki
        'n' -> ^IN -> ki;

    ID | ki
        ' ' -> fail
        _ ->> ID;

    state   ^ACC -> other_start
        'v' -> ^_
        'i' .. 'j' -> def_acc;

    def_acc ^_;
}

grammar {
    s `{} {}`
        | ID s
        | ID `{}`;
}
        ";

        //exercise
        let tree = lang::parse_spec(input).unwrap();

        //verify
        assert_eq!(
            tree.to_string(),
            "└── Spec
    └── Regions
        ├── Regions
        │   ├── Regions
        │   │   └── Region
        │   │       └── Alphabet
        │   │           ├── TAlphabet <- 'alphabet'
        │   │           └── TCil <- ''vinj ''
        │   └── Region
        │       └── CDFA
        │           ├── TCDFA <- 'cdfa'
        │           ├── TLeftBrace <- '{'
        │           ├── States
        │           │   ├── States
        │           │   │   ├── States
        │           │   │   │   ├── States
        │           │   │   │   │   ├── States
        │           │   │   │   │   │   └── State
        │           │   │   │   │   │       ├── StateDeclarator
        │           │   │   │   │   │       │   └── Targets
        │           │   │   │   │   │       │       └── TId <- 'start'
        │           │   │   │   │   │       ├── TransitionsOpt
        │           │   │   │   │   │       │   └── Transitions
        │           │   │   │   │   │       │       ├── Transitions
        │           │   │   │   │   │       │       │   └── Transition
        │           │   │   │   │   │       │       │       ├── TransitionPattern
        │           │   │   │   │   │       │       │       │   └── Matchers
        │           │   │   │   │   │       │       │       │       └── Matcher
        │           │   │   │   │   │       │       │       │           └── TCil <- ''i''
        │           │   │   │   │   │       │       │       ├── TransitionMethod
        │           │   │   │   │   │       │       │       │   └── TArrow <- '->'
        │           │   │   │   │   │       │       │       └── TransitionDestination
        │           │   │   │   │   │       │       │           └── TId <- 'ki'
        │           │   │   │   │   │       │       └── Transition
        │           │   │   │   │   │       │           ├── TransitionPattern
        │           │   │   │   │   │       │           │   └── TDef <- '_'
        │           │   │   │   │   │       │           ├── TransitionMethod
        │           │   │   │   │   │       │           │   └── TArrow <- '->'
        │           │   │   │   │   │       │           └── TransitionDestination
        │           │   │   │   │   │       │               └── Acceptor
        │           │   │   │   │   │       │                   ├── THat <- '^'
        │           │   │   │   │   │       │                   ├── IdOrDef
        │           │   │   │   │   │       │                   │   └── TId <- 'ID'
        │           │   │   │   │   │       │                   └── AcceptorDestinationOpt
        │           │   │   │   │   │       │                       └──  <- 'NULL'
        │           │   │   │   │   │       └── TSemi <- ';'
        │           │   │   │   │   └── State
        │           │   │   │   │       ├── StateDeclarator
        │           │   │   │   │       │   └── Targets
        │           │   │   │   │       │       └── TId <- 'ki'
        │           │   │   │   │       ├── TransitionsOpt
        │           │   │   │   │       │   └── Transitions
        │           │   │   │   │       │       └── Transition
        │           │   │   │   │       │           ├── TransitionPattern
        │           │   │   │   │       │           │   └── Matchers
        │           │   │   │   │       │           │       └── Matcher
        │           │   │   │   │       │           │           └── TCil <- ''n''
        │           │   │   │   │       │           ├── TransitionMethod
        │           │   │   │   │       │           │   └── TArrow <- '->'
        │           │   │   │   │       │           └── TransitionDestination
        │           │   │   │   │       │               └── Acceptor
        │           │   │   │   │       │                   ├── THat <- '^'
        │           │   │   │   │       │                   ├── IdOrDef
        │           │   │   │   │       │                   │   └── TId <- 'IN'
        │           │   │   │   │       │                   └── AcceptorDestinationOpt
        │           │   │   │   │       │                       ├── TArrow <- '->'
        │           │   │   │   │       │                       └── TId <- 'ki'
        │           │   │   │   │       └── TSemi <- ';'
        │           │   │   │   └── State
        │           │   │   │       ├── StateDeclarator
        │           │   │   │       │   └── Targets
        │           │   │   │       │       ├── Targets
        │           │   │   │       │       │   └── TId <- 'ID'
        │           │   │   │       │       ├── TOr <- '|'
        │           │   │   │       │       └── TId <- 'ki'
        │           │   │   │       ├── TransitionsOpt
        │           │   │   │       │   └── Transitions
        │           │   │   │       │       ├── Transitions
        │           │   │   │       │       │   └── Transition
        │           │   │   │       │       │       ├── TransitionPattern
        │           │   │   │       │       │       │   └── Matchers
        │           │   │   │       │       │       │       └── Matcher
        │           │   │   │       │       │       │           └── TCil <- '' ''
        │           │   │   │       │       │       ├── TransitionMethod
        │           │   │   │       │       │       │   └── TArrow <- '->'
        │           │   │   │       │       │       └── TransitionDestination
        │           │   │   │       │       │           └── TId <- 'fail'
        │           │   │   │       │       └── Transition
        │           │   │   │       │           ├── TransitionPattern
        │           │   │   │       │           │   └── TDef <- '_'
        │           │   │   │       │           ├── TransitionMethod
        │           │   │   │       │           │   └── TDoubleArrow <- '->>'
        │           │   │   │       │           └── TransitionDestination
        │           │   │   │       │               └── TId <- 'ID'
        │           │   │   │       └── TSemi <- ';'
        │           │   │   └── State
        │           │   │       ├── StateDeclarator
        │           │   │       │   ├── Targets
        │           │   │       │   │   └── TId <- 'state'
        │           │   │       │   └── Acceptor
        │           │   │       │       ├── THat <- '^'
        │           │   │       │       ├── IdOrDef
        │           │   │       │       │   └── TId <- 'ACC'
        │           │   │       │       └── AcceptorDestinationOpt
        │           │   │       │           ├── TArrow <- '->'
        │           │   │       │           └── TId <- 'other_start'
        │           │   │       ├── TransitionsOpt
        │           │   │       │   └── Transitions
        │           │   │       │       ├── Transitions
        │           │   │       │       │   └── Transition
        │           │   │       │       │       ├── TransitionPattern
        │           │   │       │       │       │   └── Matchers
        │           │   │       │       │       │       └── Matcher
        │           │   │       │       │       │           └── TCil <- ''v''
        │           │   │       │       │       ├── TransitionMethod
        │           │   │       │       │       │   └── TArrow <- '->'
        │           │   │       │       │       └── TransitionDestination
        │           │   │       │       │           └── Acceptor
        │           │   │       │       │               ├── THat <- '^'
        │           │   │       │       │               ├── IdOrDef
        │           │   │       │       │               │   └── TDef <- '_'
        │           │   │       │       │               └── AcceptorDestinationOpt
        │           │   │       │       │                   └──  <- 'NULL'
        │           │   │       │       └── Transition
        │           │   │       │           ├── TransitionPattern
        │           │   │       │           │   └── Matchers
        │           │   │       │           │       └── Matcher
        │           │   │       │           │           ├── TCil <- ''i''
        │           │   │       │           │           ├── TRange <- '..'
        │           │   │       │           │           └── TCil <- ''j''
        │           │   │       │           ├── TransitionMethod
        │           │   │       │           │   └── TArrow <- '->'
        │           │   │       │           └── TransitionDestination
        │           │   │       │               └── TId <- 'def_acc'
        │           │   │       └── TSemi <- ';'
        │           │   └── State
        │           │       ├── StateDeclarator
        │           │       │   ├── Targets
        │           │       │   │   └── TId <- 'def_acc'
        │           │       │   └── Acceptor
        │           │       │       ├── THat <- '^'
        │           │       │       ├── IdOrDef
        │           │       │       │   └── TDef <- '_'
        │           │       │       └── AcceptorDestinationOpt
        │           │       │           └──  <- 'NULL'
        │           │       ├── TransitionsOpt
        │           │       │   └──  <- 'NULL'
        │           │       └── TSemi <- ';'
        │           └── TRightBrace <- '}'
        └── Region
            └── Grammar
                ├── TGrammar <- 'grammar'
                ├── TLeftBrace <- '{'
                ├── Productions
                │   └── Production
                │       ├── TId <- 's'
                │       ├── PatternOpt
                │       │   └── TPattern <- '`{} {}`'
                │       ├── RightHandSides
                │       │   ├── RightHandSides
                │       │   │   └── RightHandSide
                │       │   │       ├── TOr <- '|'
                │       │   │       ├── Ids
                │       │   │       │   ├── Ids
                │       │   │       │   │   ├── Ids
                │       │   │       │   │   │   └──  <- 'NULL'
                │       │   │       │   │   └── TId <- 'ID'
                │       │   │       │   └── TId <- 's'
                │       │   │       └── PatternOpt
                │       │   │           └──  <- 'NULL'
                │       │   └── RightHandSide
                │       │       ├── TOr <- '|'
                │       │       ├── Ids
                │       │       │   ├── Ids
                │       │       │   │   └──  <- 'NULL'
                │       │       │   └── TId <- 'ID'
                │       │       └── PatternOpt
                │       │           └── TPattern <- '`{}`'
                │       └── TSemi <- ';'
                └── TRightBrace <- '}'"
        );
    }

    #[test]
    fn generate_spec_simple() {
        //setup
        let spec = "
alphabet ' \\t\\n\\r{}'

cdfa {
    start
        ' ' | '\\t' | '\\n' | '\\r' -> ws
        '{' -> lbr
        '}' -> rbr;

    ws  ^WHITESPACE
        ' ' | '\\t' | '\\n' | '\\r' -> ws;

    lbr ^LBRACKET;

    rbr ^RBRACKET;
}

grammar {
    s
        | s b
        | ;

    b
        | LBRACKET s RBRACKET `[prefix]{0}\\n\\n{1;prefix=[prefix]\\t}[prefix]{2}\\n\\n`
        | w ;

    w   | WHITESPACE ``;
}
        ";

        let input = "  {  {  {{{\t}}}\n\r {} } \r }   { {}\n } ".to_string();
        let chars: Vec<char> = input.chars().collect();

        let lexer = lex::def_lexer();
        let parser = parse::def_parser();

        //specification
        let tree = lang::parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, grammar, formatter) =
            generate_spec(&parse, SimpleGrammarBuilder::new()).unwrap();

        //input
        let tokens = lexer.lex(&chars[..], &*cdfa);
        let tree = parser.parse(tokens.unwrap(), &*grammar);
        let parse = tree.unwrap();

        //exercise
        let res = formatter.format(&parse);

        //verify
        assert_eq!(
            res,
            "{

	{

		{

			{

				{

				}

			}

		}

		{

		}

	}

}

{

	{

	}

}\n\n"
        );
    }

    #[test]
    fn generate_spec_advanced_operators() {
        //setup
        let spec = "
alphabet 'inj '

cdfa {
    start
        'in' -> ^IN
        ' ' -> ^_
        _ -> ^ID;

    ID | IN
        ' ' -> fail
        _ -> ID;
}

grammar {
    s |;
}
        ";

        let input = "i ij ijjjijijiji inj in iii".to_string();
        let chars: Vec<char> = input.chars().collect();

        let lexer = lex::def_lexer();

        let tree = lang::parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, _, _) = generate_spec(&parse, SimpleGrammarBuilder::new()).unwrap();

        //exercise
        let tokens = lexer.lex(&chars[..], &*cdfa).unwrap();

        let mut result = String::new();
        for token in tokens {
            result.push_str(&token.to_string());
            result.push('\n');
        }

        //verify
        assert_eq!(
            result,
            "\
ID <- 'i'
ID <- 'ij'
ID <- 'ijjjijijiji'
ID <- 'inj'
IN <- 'in'
ID <- 'iii'
"
        );
    }

    #[test]
    fn default_matcher_conflict() {
        //setup
        let spec = "
alphabet ' c'

cdfa {
    start
        ' ' -> ^WS
        'c' -> id;

    id      ^ID
        'c' | '_' -> id;
}

grammar {
    s |;
}
        ";

        let input = "c c".to_string();
        let chars: Vec<char> = input.chars().collect();

        let lexer = lex::def_lexer();

        let tree = lang::parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, _, _) = generate_spec(&parse, SimpleGrammarBuilder::new()).unwrap();

        //exercise
        let tokens = lexer.lex(&chars[..], &*cdfa).unwrap();

        //verify
        assert_eq!(
            tokens_string(tokens),
            "\nkind=ID lexeme=c\nkind=WS lexeme= \nkind=ID lexeme=c"
        )
    }

    #[test]
    fn complex_id() {
        //setup
        let spec = "
alphabet ' ab_'

cdfa {
    start
        ' ' -> ws
        _ -> id;

    ws      ^_;

    id      ^ID
        'a' | 'b' | '_' -> id;
}

grammar {
    s
        | ids
        |;
    ids
        | ids ID
        | ID;
}
        ";

        let input = "a ababab _abab ab_abba_".to_string();
        let chars: Vec<char> = input.chars().collect();

        let lexer = lex::def_lexer();

        let tree = lang::parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, _, _) = generate_spec(&parse, SimpleGrammarBuilder::new()).unwrap();

        //exercise
        let tokens = lexer.lex(&chars[..], &*cdfa).unwrap();

        //verify
        assert_eq!(tokens_string(tokens), "\nkind=ID lexeme=a\nkind=ID lexeme=ababab\nkind=ID lexeme=_abab\nkind=ID lexeme=ab_abba_")
    }

    #[test]
    fn multi_character_lexing() {
        //setup
        let spec = "
alphabet 'abcdefghijklmnopqrstuvwxyz '

cdfa {
    start
        'if' -> ^IF
        'else' -> ^ELSE
        'for' -> ^FOR
        'fob' -> ^FOB
        'final' -> ^FINAL
        ' ' -> ^_
        _ -> id;

    id  ^ID
        ' ' -> fail
        _ -> id;
}

grammar {
    s |;
}
        ";
        let tree = lang::parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, _, _) = generate_spec(&parse, SimpleGrammarBuilder::new()).unwrap();

        let input = "fdkgdfjgdjglkdjglkdjgljbnhbduhoifjeoigjeoghknhkjdfjgoirjt for if endif elseif somethign eldsfnj hi bob joe here final for fob else if id idhere fobre f ".to_string();
        let chars: Vec<char> = input.chars().collect();

        let lexer = lex::def_lexer();

        //exercise
        let tokens = lexer.lex(&chars[..], &*cdfa).unwrap();

        //verify
        assert_eq!(
            tokens_string(tokens),
            "
kind=ID lexeme=fdkgdfjgdjglkdjglkdjgljbnhbduhoifjeoigjeoghknhkjdfjgoirjt
kind=FOR lexeme=for
kind=IF lexeme=if
kind=ID lexeme=endif
kind=ELSE lexeme=else
kind=IF lexeme=if
kind=ID lexeme=somethign
kind=ID lexeme=eldsfnj
kind=ID lexeme=hi
kind=ID lexeme=bob
kind=ID lexeme=joe
kind=ID lexeme=here
kind=FINAL lexeme=final
kind=FOR lexeme=for
kind=FOB lexeme=fob
kind=ELSE lexeme=else
kind=IF lexeme=if
kind=ID lexeme=id
kind=ID lexeme=idhere
kind=FOB lexeme=fob
kind=ID lexeme=re
kind=ID lexeme=f"
        )
    }

    #[test]
    fn single_reference_optional_shorthand() {
        //setup
        let spec = "
alphabet 'ab'

cdfa {
    start
        'a' -> ^A
        'b' -> ^B;
}

grammar {
    s
        | A [B] s
        |;
}
        ";

        let tree = lang::parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, grammar, _) = generate_spec(&parse, SimpleGrammarBuilder::new()).unwrap();

        let input = "ababaaaba".to_string();
        let chars: Vec<char> = input.chars().collect();

        let lexer = lex::def_lexer();
        let parser = parse::def_parser();

        //exercise
        let tokens = lexer.lex(&chars[..], &*cdfa).unwrap();
        let tree = parser.parse(tokens, &*grammar).unwrap();

        //verify
        assert_eq!(
            tree.to_string(),
            "└── s
    ├── A <- 'a'
    ├── opt#B
    │   └── B <- 'b'
    └── s
        ├── A <- 'a'
        ├── opt#B
        │   └── B <- 'b'
        └── s
            ├── A <- 'a'
            ├── opt#B
            │   └──  <- 'NULL'
            └── s
                ├── A <- 'a'
                ├── opt#B
                │   └──  <- 'NULL'
                └── s
                    ├── A <- 'a'
                    ├── opt#B
                    │   └── B <- 'b'
                    └── s
                        ├── A <- 'a'
                        ├── opt#B
                        │   └──  <- 'NULL'
                        └── s
                            └──  <- 'NULL'"
        );
    }

    #[test]
    fn def_pattern() {
        //setup
        let spec = "
alphabet 'ab'

cdfa {
    start
        'a' -> ^A
        'b' -> ^B;
}

grammar {
    s `{} {}`
        | s A
        | s B
        | `SEPARATED:`;
}
        ";

        let tree = lang::parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, grammar, formatter) =
            generate_spec(&parse, SimpleGrammarBuilder::new()).unwrap();

        let input = "abaa".to_string();
        let chars: Vec<char> = input.chars().collect();

        let lexer = lex::def_lexer();
        let parser = parse::def_parser();

        //exercise
        let tokens = lexer.lex(&chars[..], &*cdfa).unwrap();
        let tree = parser.parse(tokens, &*grammar).unwrap();
        let res = formatter.format(&tree);

        //verify
        assert_eq!(res, "SEPARATED: a b a a");
    }

    #[test]
    fn range_based_matchers() {
        //setup
        let spec = "
alphabet 'abcdefghijklmnopqrstuvwxyz'

cdfa {
    start
        'a'..'d' -> ^A
        'e'..'k' | 'l' -> ^B
        'm'..'m' -> ^C
        'n'..'o' -> ^D
        _ -> ^E;

    E
        'p'..'z' -> E;
}

grammar {
    s |;
}
        ";

        let input = "abcdefghijklmnopqrstuvwxyz".to_string();
        let chars: Vec<char> = input.chars().collect();

        let lexer = lex::def_lexer();
        let tree = lang::parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, _, _) = generate_spec(&parse, SimpleGrammarBuilder::new()).unwrap();

        //exercise
        let tokens = lexer.lex(&chars[..], &*cdfa).unwrap();

        //verify
        assert_eq!(
            tokens_string(tokens),
            "
kind=A lexeme=a
kind=A lexeme=b
kind=A lexeme=c
kind=A lexeme=d
kind=B lexeme=e
kind=B lexeme=f
kind=B lexeme=g
kind=B lexeme=h
kind=B lexeme=i
kind=B lexeme=j
kind=B lexeme=k
kind=B lexeme=l
kind=C lexeme=m
kind=D lexeme=n
kind=D lexeme=o
kind=E lexeme=pqrstuvwxyz"
        )
    }

    #[test]
    fn context_sensitive_lexer() {
        //setup
        let spec = "
alphabet 'a!123456789'

cdfa {
    start
        'a' -> a
        '!' -> bang_in;

    bang_in ^BANG -> hidden;

    a       ^A
        'a' -> a;

    hidden
        '1' .. '9' -> num
        '!' -> ^BANG -> start;

    num     ^NUM;
}

grammar {
    s |;
}
        ";

        let input = "!!aaa!!a!49913!a".to_string();
        let chars: Vec<char> = input.chars().collect();

        let lexer = lex::def_lexer();
        let tree = lang::parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, _, _) = generate_spec(&parse, SimpleGrammarBuilder::new()).unwrap();

        //exercise
        let tokens = lexer.lex(&chars[..], &*cdfa).unwrap();

        //verify
        assert_eq!(
            tokens_string(tokens),
            "
kind=BANG lexeme=!
kind=BANG lexeme=!
kind=A lexeme=aaa
kind=BANG lexeme=!
kind=BANG lexeme=!
kind=A lexeme=a
kind=BANG lexeme=!
kind=NUM lexeme=4
kind=NUM lexeme=9
kind=NUM lexeme=9
kind=NUM lexeme=1
kind=NUM lexeme=3
kind=BANG lexeme=!
kind=A lexeme=a"
        )
    }

    #[test]
    fn injectable_terminals_region() {
        //setup
        let spec = "
alphabet ''

cdfa {
    start ;
}

inject left id1
inject right id2 `{}\n`

inject
left
id3
`some pattern`

grammar {
    s |;
}
        ";

        //exercise
        let tree = lang::parse_spec(spec).unwrap();

        //verify
        assert_eq!(
            tree.to_string(),
            "└── Spec
    └── Regions
        ├── Regions
        │   ├── Regions
        │   │   ├── Regions
        │   │   │   ├── Regions
        │   │   │   │   ├── Regions
        │   │   │   │   │   └── Region
        │   │   │   │   │       └── Alphabet
        │   │   │   │   │           ├── TAlphabet <- 'alphabet'
        │   │   │   │   │           └── TCil <- ''''
        │   │   │   │   └── Region
        │   │   │   │       └── CDFA
        │   │   │   │           ├── TCDFA <- 'cdfa'
        │   │   │   │           ├── TLeftBrace <- '{'
        │   │   │   │           ├── States
        │   │   │   │           │   └── State
        │   │   │   │           │       ├── StateDeclarator
        │   │   │   │           │       │   └── Targets
        │   │   │   │           │       │       └── TId <- 'start'
        │   │   │   │           │       ├── TransitionsOpt
        │   │   │   │           │       │   └──  <- 'NULL'
        │   │   │   │           │       └── TSemi <- ';'
        │   │   │   │           └── TRightBrace <- '}'
        │   │   │   └── Region
        │   │   │       └── Injectable
        │   │   │           ├── TInjectable <- 'inject'
        │   │   │           ├── TInjectionAffinity <- 'left'
        │   │   │           ├── TId <- 'id1'
        │   │   │           └── PatternOpt
        │   │   │               └──  <- 'NULL'
        │   │   └── Region
        │   │       └── Injectable
        │   │           ├── TInjectable <- 'inject'
        │   │           ├── TInjectionAffinity <- 'right'
        │   │           ├── TId <- 'id2'
        │   │           └── PatternOpt
        │   │               └── TPattern <- '`{}\\n`'
        │   └── Region
        │       └── Injectable
        │           ├── TInjectable <- 'inject'
        │           ├── TInjectionAffinity <- 'left'
        │           ├── TId <- 'id3'
        │           └── PatternOpt
        │               └── TPattern <- '`some pattern`'
        └── Region
            └── Grammar
                ├── TGrammar <- 'grammar'
                ├── TLeftBrace <- '{'
                ├── Productions
                │   └── Production
                │       ├── TId <- 's'
                │       ├── PatternOpt
                │       │   └──  <- 'NULL'
                │       ├── RightHandSides
                │       │   └── RightHandSide
                │       │       ├── TOr <- '|'
                │       │       ├── Ids
                │       │       │   └──  <- 'NULL'
                │       │       └── PatternOpt
                │       │           └──  <- 'NULL'
                │       └── TSemi <- ';'
                └── TRightBrace <- '}'"
        )
    }

    #[test]
    fn non_consuming_transitions() {
        //setup
        let spec = "
alphabet 'abc'

cdfa {
    start1
        'a' -> ^A1
        'b' ->> start2
        _ ->> start3;

    start2
        'b' -> ^B2
        'c' -> ^C2;

    start3
        'c' -> ^C3;
}

grammar {
    s |;
}
        ";

        let input = "abca".to_string();
        let chars: Vec<char> = input.chars().collect();

        let lexer = lex::def_lexer();
        let tree = lang::parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, _, _) = generate_spec(&parse, SimpleGrammarBuilder::new()).unwrap();

        //exercise
        let tokens = lexer.lex(&chars[..], &*cdfa).unwrap();

        //verify
        assert_eq!(
            tokens_string(tokens),
            "
kind=A1 lexeme=a
kind=B2 lexeme=b
kind=C3 lexeme=c
kind=A1 lexeme=a"
        )
    }

    #[test]
    fn inject_left_before_nested_empty_optional() {
        //setup
        let spec = "
alphabet 'abc'

cdfa {
    start
        'a' -> ^A
        'b' -> ^B
        'c' -> ^C;
}

inject left C

grammar {
    s
        | v B
        | ;

    v
        | A [s];
}
        ";

        let input = "acb".to_string();
        let chars: Vec<char> = input.chars().collect();

        let lexer = lex::def_lexer();
        let tree = lang::parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, grammar, _) = generate_spec(&parse, SimpleGrammarBuilder::new()).unwrap();

        //exercise
        let tokens = lexer.lex(&chars[..], &*cdfa).unwrap();
        let parse = parse::def_parser().parse(tokens, &*grammar).unwrap();

        //verify
        assert_eq!(
            parse.to_string(),
            "└── s
    ├── v
    │   ├── A <- 'a'
    │   ├── << C <- 'c'
    │   └── opt#s
    │       └──  <- 'NULL'
    └── B <- 'b'"
        )
    }

    #[test]
    fn inline_list() {
        //setup
        let spec = "
cdfa {
    start
        'a' -> ^A;
}

grammar {
    s | {A};
}
        ";

        let input = "aaa".to_string();
        let chars: Vec<char> = input.chars().collect();

        let lexer = lex::def_lexer();
        let tree = lang::parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, grammar, _) = generate_spec(&parse, SimpleGrammarBuilder::new()).unwrap();

        //exercise
        let tokens = lexer.lex(&chars[..], &*cdfa).unwrap();
        let parse = parse::def_parser().parse(tokens, &*grammar).unwrap();

        //verify
        assert_eq!(
            parse.to_string(),
            "└── s
    └── ?
        ├── A <- 'a'
        ├── A <- 'a'
        └── A <- 'a'"
        )
    }

    fn tokens_string(tokens: Vec<Token<String>>) -> String {
        let mut res_string = String::new();
        for token in tokens {
            res_string = format!(
                "{}\nkind={} lexeme={}",
                res_string,
                Data::to_string(token.kind()),
                token.lexeme()
            );
        }
        res_string
    }
}
