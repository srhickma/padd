#[macro_use]
extern crate lazy_static;
extern crate stopwatch;

use std::error;
use std::fmt;
use core::data::stream::StreamSource;
use core::scan;
use core::scan::runtime;
use core::scan::runtime::Scanner;
use core::scan::runtime::ecdfa::EncodedCDFA;
use core::parse;
use core::parse::Grammar;
use core::parse::Parser;
use core::fmt::Formatter;
use core::spec;

mod core;

pub struct FormatJobRunner {
    cdfa: EncodedCDFA,
    grammar: Grammar,
    formatter: Formatter,
    scanner: Box<Scanner<usize, String>>,
    parser: Box<Parser>,
}

impl FormatJobRunner {
    pub fn build(spec: &String) -> Result<FormatJobRunner, BuildError> {
        let parse = spec::parse_spec(spec)?;
        let (cdfa, grammar, formatter) = spec::generate_spec(&parse)?;
        Ok(FormatJobRunner {
            cdfa,
            grammar,
            formatter,
            scanner: runtime::def_scanner(),
            parser: parse::def_parser(),
        })
    }

    pub fn format(&self, input: &String) -> Result<String, FormatError> {
        //TODO take a stream as input here?

        let mut iter = input.chars();
        let mut getter = || {
            iter.next()
        };
        let mut stream: StreamSource<char> = StreamSource::observe(&mut getter);

        let tokens = self.scanner.scan(&mut stream, &self.cdfa)?;
        let parse = self.parser.parse(tokens, &self.grammar)?;
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
            BuildError::SpecParseErr(ref err) => write!(f, "Failed to parse specification: {}", err),
            BuildError::SpecGenErr(ref err) => write!(f, "Failed to generate specification: {}", err),
        }
    }
}

impl error::Error for BuildError {
    fn cause(&self) -> Option<&error::Error> {
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
    fn cause(&self) -> Option<&error::Error> {
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
    use super::*;

    #[test]
    fn test_failed_scan_input() {
        //setup
        let spec = "
'ab'

start 'a' -> ^ACC;

s -> ACC;
    ".to_string();

        let fjr = FormatJobRunner::build(&spec).unwrap();

        //exercise
        let res = fjr.format(&"b".to_string());

        //verify
        assert!(res.is_err());
        assert_eq!(
            format!("{}", res.err().unwrap()),
            "Failed to scan input: No accepting scans after (1,1): b..."
        );
    }

    #[test]
    fn test_failed_parse_input() {
        //setup
        let spec = "
'a'

start 'a' -> ^ACC;

s -> B;
    ".to_string();

        let fjr = FormatJobRunner::build(&spec).unwrap();

        //exercise
        let res = fjr.format(&"a".to_string());

        //verify
        assert!(res.is_err());
        assert_eq!(
            format!("{}", res.err().unwrap()),
            "Failed to parse input: Recognition failed at token 1: ACC <- 'a'"
        );
    }

    #[test]
    fn test_failed_scan_spec() {
        //setup
        let spec = "
'ab'~

start 'a' -> ^ACC;

s -> ACC;
    ".to_string();

        //exercise
        let res = FormatJobRunner::build(&spec);

        //verify
        assert!(res.is_err());
        assert_eq!(
            format!("{}", res.err().unwrap()),
            "Failed to parse specification: Scan error: No accepting scans after (2,5): ~\n\nstart \'..."
        );
    }

    #[test]
    fn test_failed_parse_spec() {
        //setup
        let spec = "
start 'a' -> ^ACC;

s -> B;
    ".to_string();

        //exercise
        let res = FormatJobRunner::build(&spec);

        //verify
        assert!(res.is_err());
        assert_eq!(
            format!("{}", res.err().unwrap()),
            "Failed to parse specification: Parse error: Recognition failed at token 1: ID <- 'start'"
        );
    }

    #[test]
    fn test_failed_cdfa_multiple_def_matchers() {
        //setup
        let spec = "
''

start
    _ -> ^A
    _ -> ^B;

s ->;
    ".to_string();

        //exercise
        let res = FormatJobRunner::build(&spec);

        //verify
        assert!(res.is_err());
        assert_eq!(
            format!("{}", res.err().unwrap()),
            "Failed to generate specification: ECDFA generation error: Failed to build CDFA: Default matcher used twice"
        );
    }

    #[test]
    fn test_failed_cdfa_non_prefix_free() {
        //setup
        let spec = "
''

start
    'a' -> ^A
    'ab' -> ^B;

s ->;
    ".to_string();

        //exercise
        let res = FormatJobRunner::build(&spec);

        //verify
        assert!(res.is_err());
        assert_eq!(
            format!("{}", res.err().unwrap()),
            "Failed to generate specification: ECDFA generation error: Failed to build CDFA: Transition trie is not prefix free"
        );
    }
}
