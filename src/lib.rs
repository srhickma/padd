#[macro_use]
extern crate lazy_static;
extern crate stopwatch;

use {
    core::{
        data::stream::StreamSource,
        fmt::Formatter,
        parse::{
            self,
            grammar::Grammar,
            Parser,
        },
        scan::{
            self,
            runtime::{
                self,
                ecdfa::EncodedCDFA,
                Scanner,
            },
        },
        spec,
    },
    std::{error, fmt},
};

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

    pub fn format(&self, stream: &mut Stream<char>) -> Result<String, FormatError> {
        let mut source = StreamSource::observe(stream.getter);
        let tokens = self.scanner.scan(&mut source, &self.cdfa)?;
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

pub struct Stream<'g, T: 'g + Clone> {
    getter: &'g mut FnMut() -> Option<T>,
}

impl<'g, T: 'g + Clone> Stream<'g, T> {
    pub fn from(getter: &'g mut FnMut() -> Option<T>) -> Stream<'g, T> {
        Stream { getter }
    }
}

pub type ThreadPool<Payload> = core::util::thread_pool::ThreadPool<Payload>;

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    #[test]
    fn failed_scan_input() {
        //setup
        let spec = "
'ab'

start 'a' -> ^ACC;

s -> ACC;
    ".to_string();

        let input = "b".to_string();
        let mut iter = input.chars();
        let mut getter = || iter.next();
        let mut stream = Stream::from(&mut getter);

        let fjr = FormatJobRunner::build(&spec).unwrap();

        //exercise
        let res = fjr.format(&mut stream);

        //verify
        assert!(res.is_err());

        let mut err: &Error = &res.err().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to scan input: No accepting scans after (1,1): b..."
        );

        err = err.cause().unwrap();
        assert_eq!(
            format!("{}", err),
            "No accepting scans after (1,1): b..."
        );

        assert!(err.cause().is_none());
    }

    #[test]
    fn failed_parse_input() {
        //setup
        let spec = "
'a'

start 'a' -> ^ACC;

s -> B;
    ".to_string();

        let input = "a".to_string();
        let mut iter = input.chars();
        let mut getter = || iter.next();
        let mut stream = Stream::from(&mut getter);

        let fjr = FormatJobRunner::build(&spec).unwrap();

        //exercise
        let res = fjr.format(&mut stream);

        //verify
        assert!(res.is_err());

        let mut err: &Error = &res.err().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to parse input: Recognition failed at token 1: ACC <- 'a'"
        );

        err = err.cause().unwrap();
        assert_eq!(
            format!("{}", err),
            "Recognition failed at token 1: ACC <- 'a'"
        );

        assert!(err.cause().is_none());
    }

    #[test]
    fn failed_scan_spec() {
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

        let mut err: &Error = &res.err().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to parse specification: Scan error: No accepting scans after (2,5): ~\n\nstart \'..."
        );

        err = err.cause().unwrap();
        assert_eq!(
            format!("{}", err),
            "Scan error: No accepting scans after (2,5): ~\n\nstart \'..."
        );

        err = err.cause().unwrap();
        assert_eq!(
            format!("{}", err),
            "No accepting scans after (2,5): ~\n\nstart \'..."
        );

        assert!(err.cause().is_none());
    }

    #[test]
    fn failed_parse_spec() {
        //setup
        let spec = "
start 'a' -> ^ACC;

s -> B;
    ".to_string();

        //exercise
        let res = FormatJobRunner::build(&spec);

        //verify
        assert!(res.is_err());

        let mut err: &Error = &res.err().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to parse specification: Parse error: Recognition failed at token 1: ID <- 'start'"
        );

        err = err.cause().unwrap();
        assert_eq!(
            format!("{}", err),
            "Parse error: Recognition failed at token 1: ID <- 'start'"
        );

        err = err.cause().unwrap();
        assert_eq!(
            format!("{}", err),
            "Recognition failed at token 1: ID <- 'start'"
        );

        assert!(err.cause().is_none());
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

        err = err.cause().unwrap();
        assert_eq!(
            format!("{}", err),
            "Parse error: No tokens scanned"
        );

        err = err.cause().unwrap();
        assert_eq!(
            format!("{}", err),
            "No tokens scanned"
        );

        assert!(err.cause().is_none());
    }

    #[test]
    fn failed_cdfa_multiple_def_matchers() {
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

        let mut err: &Error = &res.err().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to generate specification: ECDFA generation error: Failed to build CDFA: Default matcher used twice"
        );

        err = err.cause().unwrap();
        assert_eq!(
            format!("{}", err),
            "ECDFA generation error: Failed to build CDFA: Default matcher used twice"
        );

        err = err.cause().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to build CDFA: Default matcher used twice"
        );

        assert!(err.cause().is_none());
    }

    #[test]
    fn failed_cdfa_non_prefix_free() {
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

        let mut err: &Error = &res.err().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to generate specification: ECDFA generation error: Failed to build CDFA: Transition trie is not prefix free on character 'a'"
        );

        err = err.cause().unwrap();
        assert_eq!(
            format!("{}", err),
            "ECDFA generation error: Failed to build CDFA: Transition trie is not prefix free on character 'a'"
        );

        err = err.cause().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to build CDFA: Transition trie is not prefix free on character 'a'"
        );

        assert!(err.cause().is_none());
    }
}
