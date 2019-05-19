#[macro_use]
extern crate lazy_static;
extern crate stopwatch;

use {
    core::{
        fmt::Formatter,
        parse::{
            self,
            grammar::{EncodedGrammarBuilder, Grammar},
            Parser,
        },
        scan::{self, Scanner, CDFA},
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

//TODO(shane) do we really need all these types??
pub struct FormatJobRunner {
    cdfa: Box<CDFA<usize, usize>>,
    grammar: Box<Grammar<usize>>,
    formatter: Formatter<usize>,
    scanner: Box<Scanner<usize, usize>>,
    parser: Box<Parser<usize>>,
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
            scanner: scan::def_scanner(),
            parser: parse::def_parser(),
        })
    }

    pub fn format(&self, job: FormatJob) -> Result<String, FormatError> {
        let chars: Vec<char> = job.text.chars().collect();

        let tokens = self.scanner.scan(&chars[..], &*self.cdfa)?;
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
    ScanErr(scan::Error),
    ParseErr(parse::Error),
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FormatError::ScanErr(ref err) => write!(f, "Failed to scan input: {}", err),
            FormatError::ParseErr(ref err) => write!(f, "Failed to parse input: {}", err),
        }
    }
}

impl error::Error for FormatError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            FormatError::ScanErr(ref err) => Some(err),
            FormatError::ParseErr(ref err) => Some(err),
        }
    }
}

impl From<scan::Error> for FormatError {
    fn from(err: scan::Error) -> FormatError {
        FormatError::ScanErr(err)
    }
}

impl From<parse::Error> for FormatError {
    fn from(err: parse::Error) -> FormatError {
        FormatError::ParseErr(err)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    #[test]
    fn failed_scan_input() {
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
            "Failed to scan input: No accepting scans after (1,1): b..."
        );

        err = err.source().unwrap();
        assert_eq!(format!("{}", err), "No accepting scans after (1,1): b...");

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
    fn failed_scan_spec() {
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
            "Failed to parse specification: Scan error: No accepting scans after (2,14): \
             ~\n\ncdfa {\n..."
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Scan error: No accepting scans after (2,14): ~\n\ncdfa {\n..."
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "No accepting scans after (2,14): ~\n\ncdfa {\n..."
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
    fn failed_empty_scan() {
        //setup
        let spec = "".to_string();

        //exercise
        let res = FormatJobRunner::build(&spec);

        //verify
        assert!(res.is_err());

        let mut err: &Error = &res.err().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to parse specification: Parse error: No tokens scanned"
        );

        err = err.source().unwrap();
        assert_eq!(format!("{}", err), "Parse error: No tokens scanned");

        err = err.source().unwrap();
        assert_eq!(format!("{}", err), "No tokens scanned");

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
             Transition trie is not prefix free on character 'l'"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "ECDFA generation error: Failed to build CDFA: \
             Transition trie is not prefix free on character 'l'"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to build CDFA: Transition trie is not prefix free on character 'l'"
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
    fn failed_cdfa_different_destinations() {
        //setup
        let spec = "
alphabet 'a'

cdfa {
    start
        'a' -> ^A -> x
        _ -> ^A -> y;
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
             State \"A\" is accepted multiple times with different destinations"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "ECDFA generation error: Failed to build CDFA: \
             State \"A\" is accepted multiple times with different destinations"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to build CDFA: \
             State \"A\" is accepted multiple times with different destinations"
        );

        assert!(err.source().is_none());
    }

    #[test]
    fn failed_cdfa_existing_acceptor_destination() {
        //setup
        let spec = "
alphabet ''

cdfa {
    start
        _ -> ^A -> x;

    A   ^A -> y;
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
             State \"A\" already has an acceptance destination from all incoming states"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "ECDFA generation error: Failed to build CDFA: \
             State \"A\" already has an acceptance destination from all incoming states"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to build CDFA: \
             State \"A\" already has an acceptance destination from all incoming states"
        );

        assert!(err.source().is_none());
    }

    #[test]
    fn failed_cdfa_existing_acceptor_destination_from_all() {
        //setup
        let spec = "
alphabet ''

cdfa {
    A   ^A -> y;

    start
        _ -> ^A -> x;
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
             State \"A\" already has an acceptance destination from a specific state"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "ECDFA generation error: Failed to build CDFA: \
             State \"A\" already has an acceptance destination from a specific state"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to build CDFA: \
             State \"A\" already has an acceptance destination from a specific state"
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
            "Failed to generate specification: Formatter build error: Pattern parse error: \
             Recognition failed after consuming all tokens"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Formatter build error: Pattern parse error: \
             Recognition failed after consuming all tokens"
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
            "Failed to generate specification: Formatter build error: Pattern capture error: \
             Capture index 4 out of bounds for production \'s\' with 0 children"
        );

        err = err.source().unwrap();
        assert_eq!(
            format!("{}", err),
            "Formatter build error: Pattern capture error: \
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
    fn failed_ignorable_terminal_error() {
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
}
