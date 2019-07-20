#[macro_use]
extern crate lazy_static;
extern crate stopwatch;

use {
    core::{
        fmt::Formatter,
        lex::{self, Lexer, CDFA},
        parse::{
            self,
            grammar::{EncodedGrammarBuilder, Grammar},
            Parser,
        },
        spec,
    },
    std::{error, fmt},
};

mod core;

pub struct FormatJob {
    text: String,
}

impl FormatJob {
    pub fn from_text(text: String) -> Self {
        FormatJob { text }
    }
}

type StateType = usize;
type SymbolType = usize;

pub struct FormatJobRunner {
    cdfa: Box<CDFA<StateType, SymbolType>>,
    grammar: Box<Grammar<SymbolType>>,
    formatter: Formatter<SymbolType>,
    lexer: Box<Lexer<StateType, SymbolType>>,
    parser: Box<Parser<SymbolType>>,
}

impl FormatJobRunner {
    pub fn build(spec: &str) -> Result<FormatJobRunner, BuildError> {
        let parse = spec::parse_spec(spec)?;
        let grammar_builder = EncodedGrammarBuilder::new();
        let (cdfa, grammar, formatter) = spec::generate_spec(&parse, grammar_builder)?;
        Ok(FormatJobRunner {
            cdfa,
            grammar,
            formatter,
            lexer: lex::def_lexer(),
            parser: parse::def_parser(),
        })
    }

    pub fn format(&self, job: FormatJob) -> Result<String, FormatError> {
        let chars: Vec<char> = job.text.chars().collect();

        let tokens = self.lexer.lex(&chars[..], &*self.cdfa)?;
        let parse = self.parser.parse(tokens, &*self.grammar)?;
        Ok(self.formatter.format(&parse))
    }
}

#[derive(Debug)]
pub enum BuildError {
    SpecParseErr(spec::ParseError),
    SpecGenErr(spec::GenError),
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            BuildError::SpecParseErr(ref err) => {
                write!(f, "Failed to parse specification: {}", err)
            }
            BuildError::SpecGenErr(ref err) => {
                write!(f, "Failed to generate specification: {}", err)
            }
        }
    }
}

impl error::Error for BuildError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            BuildError::SpecParseErr(ref err) => Some(err),
            BuildError::SpecGenErr(ref err) => Some(err),
        }
    }
}

impl From<spec::ParseError> for BuildError {
    fn from(err: spec::ParseError) -> BuildError {
        BuildError::SpecParseErr(err)
    }
}

impl From<spec::GenError> for BuildError {
    fn from(err: spec::GenError) -> BuildError {
        BuildError::SpecGenErr(err)
    }
}

#[derive(Debug)]
pub enum FormatError {
    LexErr(lex::Error),
    ParseErr(parse::Error),
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FormatError::LexErr(ref err) => write!(f, "Failed to lex input: {}", err),
            FormatError::ParseErr(ref err) => write!(f, "Failed to parse input: {}", err),
        }
    }
}

impl error::Error for FormatError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            FormatError::LexErr(ref err) => Some(err),
            FormatError::ParseErr(ref err) => Some(err),
        }
    }
}

impl From<lex::Error> for FormatError {
    fn from(err: lex::Error) -> FormatError {
        FormatError::LexErr(err)
    }
}

impl From<parse::Error> for FormatError {
    fn from(err: parse::Error) -> FormatError {
        FormatError::ParseErr(err)
    }
}

#[cfg(test)]
mod tests {
    use {core::parse::grammar::SimpleGrammarBuilder, std::error::Error};

    use super::*;

    #[test]
    fn failed_lex_input() {
        //setup
        let spec = "
alphabet 'ab'

cdfa {
    start 'a' -> ^ACC;
}

grammar {
    s | ACC;
}
        "
        .to_string();

        let fjr = FormatJobRunner::build(&spec).unwrap();

        //exercise
        let res = fjr.format(FormatJob::from_text("b".to_string()));

        //verify
        assert!(res.is_err());

        let mut err: &Error = &res.err().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to lex input: No accepting tokens after (1,1): b..."
        );

        err = err.source().unwrap();
        assert_eq!(format!("{}", err), "No accepting tokens after (1,1): b...");

        assert!(err.source().is_none());
    }

    #[test]
    fn failed_lex_alphabet() {
        //setup
        let spec = "
alphabet 'a'

cdfa {
    start _ -> ^_;
}

grammar {
    s |;
}
        "
        .to_string();

        let fjr = FormatJobRunner::build(&spec).unwrap();

        //exercise
        let res = fjr.format(FormatJob::from_text("b".to_string()));

        //verify
        assert!(res.is_err());

        let mut err: &Error = &res.err().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to lex input: Consuming character outside lexer alphabet: 'b'"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Consuming character outside lexer alphabet: 'b'"
        );

        assert!(err.source().is_none());
    }

    #[test]
    fn failed_parse_input() {
        //setup
        let spec = "
alphabet 'ab'

cdfa {
    start
        'a' -> ^ACC
        'b' -> ^B;
}

grammar {
    s | B;
}
        "
        .to_string();

        let fjr = FormatJobRunner::build(&spec).unwrap();

        //exercise
        let res = fjr.format(FormatJob::from_text("a".to_string()));

        //verify
        assert!(res.is_err());

        let mut err: &Error = &res.err().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to parse input: Recognition failed at token 1: ACC <- 'a'"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Recognition failed at token 1: ACC <- 'a'"
        );

        assert!(err.source().is_none());
    }

    #[test]
    fn failed_lex_spec() {
        //setup
        let spec = "
alphabet 'ab'~

cdfa {
    start 'a' -> ^ACC;
}

grammar {
    s | ACC;
}
        "
        .to_string();

        //exercise
        let res = FormatJobRunner::build(&spec);

        //verify
        assert!(res.is_err());

        let mut err: &Error = &res.err().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to parse specification: Lex error: No accepting tokens after (2,14): \
             ~\n\ncdfa {\n..."
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Lex error: No accepting tokens after (2,14): ~\n\ncdfa {\n..."
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "No accepting tokens after (2,14): ~\n\ncdfa {\n..."
        );

        assert!(err.source().is_none());
    }

    #[test]
    fn failed_parse_spec() {
        //setup
        let spec = "
alphabet 'a'

cdfa {
    start 'a' -> ^ACC SOMETHING;
}

grammar {
    s | B;
}
        "
        .to_string();

        //exercise
        let res = FormatJobRunner::build(&spec);

        //verify
        assert!(res.is_err());

        let mut err: &Error = &res.err().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to parse specification: Parse error: Recognition failed at token 10: \
             TId <- 'SOMETHING'"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Parse error: Recognition failed at token 10: TId <- 'SOMETHING'"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Recognition failed at token 10: TId <- 'SOMETHING'"
        );

        assert!(err.source().is_none());
    }

    #[test]
    fn failed_empty_lex() {
        //setup
        let spec = "".to_string();

        //exercise
        let res = FormatJobRunner::build(&spec);

        //verify
        assert!(res.is_err());

        let mut err: &Error = &res.err().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to parse specification: Parse error: No symbols tokenized"
        );

        err = err.source().unwrap();
        assert_eq!(format!("{}", err), "Parse error: No symbols tokenized");

        err = err.source().unwrap();
        assert_eq!(format!("{}", err), "No symbols tokenized");

        assert!(err.source().is_none());
    }

    #[test]
    fn failed_cdfa_multiple_def_matchers() {
        //setup
        let spec = "
alphabet ''

cdfa {
    start
        _ -> ^A
        _ -> ^B;
}

grammar {
    s |;
}
        "
        .to_string();

        //exercise
        let res = FormatJobRunner::build(&spec);

        //verify
        assert!(res.is_err());

        let mut err: &Error = &res.err().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to generate specification: ECDFA generation error: Failed to build CDFA: \
             Default matcher used twice"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "ECDFA generation error: Failed to build CDFA: Default matcher used twice"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to build CDFA: Default matcher used twice"
        );

        assert!(err.source().is_none());
    }

    #[test]
    fn failed_cdfa_non_prefix_free() {
        //setup
        let spec = "
alphabet ''

cdfa {
    start
        'a' -> ^A
        'ab' -> ^B;
}

grammar {
    s |;
}
        "
        .to_string();

        //exercise
        let res = FormatJobRunner::build(&spec);

        //verify
        assert!(res.is_err());

        let mut err: &Error = &res.err().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to generate specification: ECDFA generation error: Failed to build CDFA: \
             Transition trie is not prefix free on character 'a'"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "ECDFA generation error: Failed to build CDFA: \
             Transition trie is not prefix free on character 'a'"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to build CDFA: Transition trie is not prefix free on character 'a'"
        );

        assert!(err.source().is_none());
    }

    #[test]
    fn failed_range_based_matchers_overlap() {
        //setup
        let spec = "
alphabet 'abcdefghijklmnopqrstuvwxyz'

cdfa {
    start
        'a' .. 'l' -> ^FIRST
        'l' .. 'z' -> ^LAST;
}

grammar {
    s |;
}
        "
        .to_string();

        //exercise
        let res = FormatJobRunner::build(&spec);

        //verify
        assert!(res.is_err());

        let mut err: &Error = &res.err().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to generate specification: ECDFA generation error: Failed to build CDFA: \
             Range matcher error: Intervals overlap"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "ECDFA generation error: Failed to build CDFA: Range matcher error: Intervals overlap"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to build CDFA: Range matcher error: Intervals overlap"
        );

        assert!(err.source().is_none());
    }

    #[test]
    fn failed_range_based_matchers_invalid_start() {
        //setup
        let spec = "
alphabet ''

cdfa {
    start
        'aa'..'b' -> ^A;
}

grammar {
    s |;
}
        "
        .to_string();

        //exercise
        let res = FormatJobRunner::build(&spec);

        //verify
        assert!(res.is_err());

        let mut err: &Error = &res.err().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to generate specification: Matcher definition error: \
             Range start must be one character, but was 'aa'"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Matcher definition error: Range start must be one character, but was 'aa'"
        );

        assert!(err.source().is_none());
    }

    #[test]
    fn failed_range_based_matchers_invalid_end() {
        //setup
        let spec = "
alphabet ''

cdfa {
    start
        'a'..'cd' -> ^A;
}

grammar {
    s |;
}
        "
        .to_string();

        //exercise
        let res = FormatJobRunner::build(&spec);

        //verify
        assert!(res.is_err());

        let mut err: &Error = &res.err().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to generate specification: Matcher definition error: \
             Range end must be one character, but was 'cd'"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Matcher definition error: Range end must be one character, but was 'cd'"
        );

        assert!(err.source().is_none());
    }

    #[test]
    fn failed_orphaned_terminal() {
        //setup
        let spec = "
alphabet 'ab'

cdfa {
    start
        'a' -> ^A
        'b' -> ^B;
}

grammar {
    s | ORPHANED;
}
        "
        .to_string();

        //exercise
        let res = FormatJobRunner::build(&spec);

        //verify
        assert!(res.is_err());

        let mut err: &Error = &res.err().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to generate specification: ECDFA to grammar mapping error: \
             Orphaned terminal 'ORPHANED' is not tokenized by the ECDFA"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "ECDFA to grammar mapping error: \
             Orphaned terminal 'ORPHANED' is not tokenized by the ECDFA"
        );

        assert!(err.source().is_none());
    }

    #[test]
    fn failed_missing_required_region() {
        //setup
        let spec = "
alphabet ''

grammar {
    s |;
}
        "
        .to_string();

        //exercise
        let res = FormatJobRunner::build(&spec);

        //verify
        assert!(res.is_err());

        let mut err: &Error = &res.err().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to generate specification: Region error: Missing required region: 'CDFA'"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Region error: Missing required region: 'CDFA'"
        );

        err = err.source().unwrap();
        assert_eq!(format!("{}", err), "Missing required region: 'CDFA'");

        assert!(err.source().is_none());
    }

    #[test]
    fn failed_pattern_lex_error() {
        //setup
        let spec = "
alphabet ''

cdfa {
    start ;
}

grammar {
    s | `\\\\`;
}
        "
        .to_string();

        //exercise
        let res = FormatJobRunner::build(&spec);

        //verify
        assert!(res.is_err());

        let mut err: &Error = &res.err().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to generate specification: Formatter build error: Pattern build error: \
             Pattern lex error: No accepting tokens after (1,1): \\..."
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Formatter build error: Pattern build error: \
             Pattern lex error: No accepting tokens after (1,1): \\..."
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Pattern build error: Pattern lex error: No accepting tokens after (1,1): \\..."
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Pattern lex error: No accepting tokens after (1,1): \\..."
        );

        err = err.source().unwrap();
        assert_eq!(format!("{}", err), "No accepting tokens after (1,1): \\...");

        assert!(err.source().is_none());
    }

    #[test]
    fn failed_pattern_parse_error() {
        //setup
        let spec = "
alphabet ''

cdfa {
    start ;
}

grammar {
    s | `{`;
}
        "
        .to_string();

        //exercise
        let res = FormatJobRunner::build(&spec);

        //verify
        assert!(res.is_err());

        let mut err: &Error = &res.err().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to generate specification: Formatter build error: Pattern build error: \
             Pattern parse error: Recognition failed after consuming all tokens"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Formatter build error: Pattern build error: Pattern parse error: \
             Recognition failed after consuming all tokens"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Pattern build error: Pattern parse error: Recognition failed after consuming all tokens"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Pattern parse error: Recognition failed after consuming all tokens"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Recognition failed after consuming all tokens"
        );

        assert!(err.source().is_none());
    }

    #[test]
    fn failed_pattern_capture_error() {
        //setup
        let spec = "
alphabet ''

cdfa {
    start ;
}

grammar {
    s | `{4}`;
}
        "
        .to_string();

        //exercise
        let res = FormatJobRunner::build(&spec);

        //verify
        assert!(res.is_err());

        let mut err: &Error = &res.err().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to generate specification: Formatter build error: Pattern build error: \
             Pattern capture error: Capture index 4 out of bounds for production \'s\' with 0 \
             children"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Formatter build error: Pattern build error: Pattern capture error: \
             Capture index 4 out of bounds for production \'s\' with 0 children"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Pattern build error: Pattern capture error: \
             Capture index 4 out of bounds for production \'s\' with 0 children"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Pattern capture error: \
             Capture index 4 out of bounds for production \'s\' with 0 children"
        );

        assert!(err.source().is_none());
    }

    #[test]
    fn failed_ignorable_non_terminal_error() {
        //setup
        let spec = "
alphabet 's'

cdfa {
    start
        's' -> S;
}

ignore s

grammar {
    s | S;
}
        "
        .to_string();

        //exercise
        let res = FormatJobRunner::build(&spec);

        //verify
        assert!(res.is_err());

        let mut err: &Error = &res.err().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to generate specification: Grammar build error: Ignored symbol 's' is \
             non-terminal"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Grammar build error: Ignored symbol 's' is non-terminal"
        );

        err = err.source().unwrap();
        assert_eq!(format!("{}", err), "Ignored symbol 's' is non-terminal");

        assert!(err.source().is_none());
    }

    #[test]
    fn failed_injected_non_terminal_error() {
        //setup
        let spec = "
alphabet 's'

cdfa {
    start
        's' -> S;
}

inject left s

grammar {
    s | S;
}
        "
        .to_string();

        //exercise
        let res = FormatJobRunner::build(&spec);

        //verify
        assert!(res.is_err());

        let mut err: &Error = &res.err().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to generate specification: Grammar build error: Injected symbol 's' is \
             non-terminal"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Grammar build error: Injected symbol 's' is non-terminal"
        );

        err = err.source().unwrap();
        assert_eq!(format!("{}", err), "Injected symbol 's' is non-terminal");

        assert!(err.source().is_none());
    }

    #[test]
    fn failed_duplicate_injection() {
        //setup
        let spec = "
alphabet 's'

cdfa {
    start
        's' -> S;
}

inject left S
inject right S `pattern`

grammar {
    s | S;
}
        ";

        //exercise
        let parse = spec::parse_spec(spec).unwrap();
        let grammar_builder = SimpleGrammarBuilder::new();
        let res = spec::generate_spec(&parse, grammar_builder);

        //verify
        assert!(res.is_err());

        let mut err: &Error = &res.err().unwrap();
        assert_eq!(
            format!("{}", err),
            "Formatter build error: Injection specified multiple times for symbol \'S\'"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Injection specified multiple times for symbol \'S\'"
        );

        assert!(err.source().is_none());
    }

    #[test]
    fn failed_injected_and_ignored_error() {
        //setup
        let spec = "
alphabet 's'

cdfa {
    start
        's' -> S;
}

ignore S
inject left S

grammar {
    s | S;
}
        "
        .to_string();

        //exercise
        let res = FormatJobRunner::build(&spec);

        //verify
        assert!(res.is_err());

        let mut err: &Error = &res.err().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to generate specification: Grammar build error: \
             Symbol 'S' is both ignored and injected"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Grammar build error: Symbol 'S' is both ignored and injected"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Symbol 'S' is both ignored and injected"
        );

        assert!(err.source().is_none());
    }
}
