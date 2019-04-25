use {
    core::{
        fmt::{self, Formatter},
        parse::{self, grammar::Grammar, Tree},
        scan::{self, ecdfa::EncodedCDFA},
    },
    std::{self, error},
};

mod gen;
mod lang;
mod region;

pub type Symbol = lang::Symbol;

pub static DEF_MATCHER: &'static str = "_";

pub fn parse_spec(input: &str) -> Result<Tree<Symbol>, ParseError> {
    lang::parse_spec(input)
}

pub fn generate_spec(
    parse: &Tree<Symbol>,
) -> Result<(EncodedCDFA<String>, Grammar<String>, Formatter), GenError> {
    gen::generate_spec(parse)
}

#[derive(Debug)]
pub enum GenError {
    MatcherErr(String),
    MappingErr(String),
    CDFAErr(scan::CDFAError),
    FormatterErr(fmt::BuildError),
    RegionErr(region::Error),
}

impl std::fmt::Display for GenError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            GenError::MatcherErr(ref err) => write!(f, "Matcher definition error: {}", err),
            GenError::MappingErr(ref err) => write!(f, "ECDFA to grammar mapping error: {}", err),
            GenError::CDFAErr(ref err) => write!(f, "ECDFA generation error: {}", err),
            GenError::FormatterErr(ref err) => write!(f, "Formatter build error: {}", err),
            GenError::RegionErr(ref err) => write!(f, "Region error: {}", err),
        }
    }
}

impl error::Error for GenError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            GenError::MatcherErr(_) => None,
            GenError::MappingErr(_) => None,
            GenError::CDFAErr(ref err) => Some(err),
            GenError::FormatterErr(ref err) => Some(err),
            GenError::RegionErr(ref err) => Some(err),
        }
    }
}

impl From<scan::CDFAError> for GenError {
    fn from(err: scan::CDFAError) -> GenError {
        GenError::CDFAErr(err)
    }
}

impl From<fmt::BuildError> for GenError {
    fn from(err: fmt::BuildError) -> GenError {
        GenError::FormatterErr(err)
    }
}

impl From<region::Error> for GenError {
    fn from(err: region::Error) -> GenError {
        GenError::RegionErr(err)
    }
}

#[derive(Debug)]
pub enum ParseError {
    ScanErr(scan::Error),
    ParseErr(parse::Error),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            ParseError::ScanErr(ref err) => write!(f, "Scan error: {}", err),
            ParseError::ParseErr(ref err) => write!(f, "Parse error: {}", err),
        }
    }
}

impl error::Error for ParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            ParseError::ScanErr(ref err) => Some(err),
            ParseError::ParseErr(ref err) => Some(err),
        }
    }
}

impl From<scan::Error> for ParseError {
    fn from(err: scan::Error) -> ParseError {
        ParseError::ScanErr(err)
    }
}

impl From<parse::Error> for ParseError {
    fn from(err: parse::Error) -> ParseError {
        ParseError::ParseErr(err)
    }
}

#[cfg(test)]
mod tests {
    use core::{data::Data, scan::Token};

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
        │           │   │   │   │       │       │   │   │   │       ├── Matchers
        │           │   │   │   │       │       │   │   │   │       │   └── Matcher
        │           │   │   │   │       │       │   │   │   │       │       └── TCil <- '' ''
        │           │   │   │   │       │       │   │   │   │       ├── TArrow <- '->'
        │           │   │   │   │       │       │   │   │   │       └── TransitionDestination
        │           │   │   │   │       │       │   │   │   │           └── TId <- 'ws'
        │           │   │   │   │       │       │   │   │   └── Transition
        │           │   │   │   │       │       │   │   │       ├── Matchers
        │           │   │   │   │       │       │   │   │       │   └── Matcher
        │           │   │   │   │       │       │   │   │       │       └── TCil <- ''\\t''
        │           │   │   │   │       │       │   │   │       ├── TArrow <- '->'
        │           │   │   │   │       │       │   │   │       └── TransitionDestination
        │           │   │   │   │       │       │   │   │           └── TId <- 'ws'
        │           │   │   │   │       │       │   │   └── Transition
        │           │   │   │   │       │       │   │       ├── Matchers
        │           │   │   │   │       │       │   │       │   └── Matcher
        │           │   │   │   │       │       │   │       │       └── TCil <- ''\\n''
        │           │   │   │   │       │       │   │       ├── TArrow <- '->'
        │           │   │   │   │       │       │   │       └── TransitionDestination
        │           │   │   │   │       │       │   │           └── TId <- 'ws'
        │           │   │   │   │       │       │   └── Transition
        │           │   │   │   │       │       │       ├── Matchers
        │           │   │   │   │       │       │       │   └── Matcher
        │           │   │   │   │       │       │       │       └── TCil <- ''{''
        │           │   │   │   │       │       │       ├── TArrow <- '->'
        │           │   │   │   │       │       │       └── TransitionDestination
        │           │   │   │   │       │       │           └── TId <- 'lbr'
        │           │   │   │   │       │       └── Transition
        │           │   │   │   │       │           ├── Matchers
        │           │   │   │   │       │           │   └── Matcher
        │           │   │   │   │       │           │       └── TCil <- ''}''
        │           │   │   │   │       │           ├── TArrow <- '->'
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
        │           │   │   │       │       │   │       ├── Matchers
        │           │   │   │       │       │   │       │   └── Matcher
        │           │   │   │       │       │   │       │       └── TCil <- '' ''
        │           │   │   │       │       │   │       ├── TArrow <- '->'
        │           │   │   │       │       │   │       └── TransitionDestination
        │           │   │   │       │       │   │           └── TId <- 'ws'
        │           │   │   │       │       │   └── Transition
        │           │   │   │       │       │       ├── Matchers
        │           │   │   │       │       │       │   └── Matcher
        │           │   │   │       │       │       │       └── TCil <- ''\\t''
        │           │   │   │       │       │       ├── TArrow <- '->'
        │           │   │   │       │       │       └── TransitionDestination
        │           │   │   │       │       │           └── TId <- 'ws'
        │           │   │   │       │       └── Transition
        │           │   │   │       │           ├── Matchers
        │           │   │   │       │           │   └── Matcher
        │           │   │   │       │           │       └── TCil <- ''\\n''
        │           │   │   │       │           ├── TArrow <- '->'
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
        _ -> ID;

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
        │           │   │   │   │   │       │       │       ├── Matchers
        │           │   │   │   │   │       │       │       │   └── Matcher
        │           │   │   │   │   │       │       │       │       └── TCil <- ''i''
        │           │   │   │   │   │       │       │       ├── TArrow <- '->'
        │           │   │   │   │   │       │       │       └── TransitionDestination
        │           │   │   │   │   │       │       │           └── TId <- 'ki'
        │           │   │   │   │   │       │       └── Transition
        │           │   │   │   │   │       │           ├── TDef <- '_'
        │           │   │   │   │   │       │           ├── TArrow <- '->'
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
        │           │   │   │   │       │           ├── Matchers
        │           │   │   │   │       │           │   └── Matcher
        │           │   │   │   │       │           │       └── TCil <- ''n''
        │           │   │   │   │       │           ├── TArrow <- '->'
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
        │           │   │   │       │       │       ├── Matchers
        │           │   │   │       │       │       │   └── Matcher
        │           │   │   │       │       │       │       └── TCil <- '' ''
        │           │   │   │       │       │       ├── TArrow <- '->'
        │           │   │   │       │       │       └── TransitionDestination
        │           │   │   │       │       │           └── TId <- 'fail'
        │           │   │   │       │       └── Transition
        │           │   │   │       │           ├── TDef <- '_'
        │           │   │   │       │           ├── TArrow <- '->'
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
        │           │   │       │       │       ├── Matchers
        │           │   │       │       │       │   └── Matcher
        │           │   │       │       │       │       └── TCil <- ''v''
        │           │   │       │       │       ├── TArrow <- '->'
        │           │   │       │       │       └── TransitionDestination
        │           │   │       │       │           └── Acceptor
        │           │   │       │       │               ├── THat <- '^'
        │           │   │       │       │               ├── IdOrDef
        │           │   │       │       │               │   └── TDef <- '_'
        │           │   │       │       │               └── AcceptorDestinationOpt
        │           │   │       │       │                   └──  <- 'NULL'
        │           │   │       │       └── Transition
        │           │   │       │           ├── Matchers
        │           │   │       │           │   └── Matcher
        │           │   │       │           │       ├── TCil <- ''i''
        │           │   │       │           │       ├── TRange <- '..'
        │           │   │       │           │       └── TCil <- ''j''
        │           │   │       │           ├── TArrow <- '->'
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
alphabet ' \\t\\n{}'

cdfa {
    start
        ' ' | '\\t' | '\\n' -> ws
        '{' -> lbr
        '}' -> rbr;

    ws  ^WHITESPACE
        ' ' | '\\t' | '\\n' -> ws;

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

        let input = "  {  {  {{{\t}}}\n {} }  }   { {}\n } ".to_string();
        let chars: Vec<char> = input.chars().collect();

        let scanner = scan::def_scanner();
        let parser = parse::def_parser();

        //specification
        let tree = lang::parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, grammar, formatter) = generate_spec(&parse).unwrap();

        //input
        let tokens = scanner.scan(&chars[..], &cdfa);
        let tree = parser.parse(tokens.unwrap(), &grammar);
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

        let scanner = scan::def_scanner();

        let tree = lang::parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, _, _) = generate_spec(&parse).unwrap();

        //exercise
        let tokens = scanner.scan(&chars[..], &cdfa).unwrap();

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

        let scanner = scan::def_scanner();

        let tree = lang::parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, _, _) = generate_spec(&parse).unwrap();

        //exercise
        let tokens = scanner.scan(&chars[..], &cdfa).unwrap();

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

        let scanner = scan::def_scanner();

        let tree = lang::parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, _, _) = generate_spec(&parse).unwrap();

        //exercise
        let tokens = scanner.scan(&chars[..], &cdfa).unwrap();

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
        let (cdfa, _, _) = generate_spec(&parse).unwrap();

        let input = "fdkgdfjgdjglkdjglkdjgljbnhbduhoifjeoigjeoghknhkjdfjgoirjt for if endif elseif somethign eldsfnj hi bob joe here final for fob else if id idhere fobre f ".to_string();
        let chars: Vec<char> = input.chars().collect();

        let scanner = scan::def_scanner();

        //exercise
        let tokens = scanner.scan(&chars[..], &cdfa).unwrap();

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
        let (cdfa, grammar, _) = generate_spec(&parse).unwrap();

        let input = "ababaaaba".to_string();
        let chars: Vec<char> = input.chars().collect();

        let scanner = scan::def_scanner();
        let parser = parse::def_parser();

        //exercise
        let tokens = scanner.scan(&chars[..], &cdfa).unwrap();
        let tree = parser.parse(tokens, &grammar).unwrap();

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
        let (cdfa, grammar, formatter) = generate_spec(&parse).unwrap();

        let input = "abaa".to_string();
        let chars: Vec<char> = input.chars().collect();

        let scanner = scan::def_scanner();
        let parser = parse::def_parser();

        //exercise
        let tokens = scanner.scan(&chars[..], &cdfa).unwrap();
        let tree = parser.parse(tokens, &grammar).unwrap();
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

        let scanner = scan::def_scanner();
        let tree = lang::parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, _, _) = generate_spec(&parse).unwrap();

        //exercise
        let tokens = scanner.scan(&chars[..], &cdfa).unwrap();

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
    fn context_sensitive_scanner() {
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

        let scanner = scan::def_scanner();
        let tree = lang::parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, _, _) = generate_spec(&parse).unwrap();

        //exercise
        let tokens = scanner.scan(&chars[..], &cdfa).unwrap();

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
