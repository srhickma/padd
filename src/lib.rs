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
            ecdfa::EncodedCDFA,
            Kind,
            Scanner,
        },
        spec,
    },
    std::{error, fmt},
};

mod core;

pub struct FormatJob {
    text: String
}

impl FormatJob {
    pub fn from_text(text: String) -> Self {
        FormatJob {
            text
        }
    }
}

pub struct FormatJobRunner {
    cdfa: EncodedCDFA<Kind>,
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
            scanner: scan::def_scanner(),
            parser: parse::def_parser(),
        })
    }

    pub fn format(&self, job: FormatJob) -> Result<String, FormatError> {
        let mut iter = job.text.chars();
        let mut getter = || iter.next();
        let mut source = StreamSource::observe(&mut getter);
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
            BuildError::SpecParseErr(ref err) => write!(
                f, "Failed to parse specification: {}", err
            ),
            BuildError::SpecGenErr(ref err) => write!(
                f, "Failed to generate specification: {}", err
            ),
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
'ab'

start
    'a' -> ^ACC
    'b' -> ^B;

s -> B;
    ".to_string();

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
            "Failed to generate specification: ECDFA generation error: Failed to build CDFA: \
            Transition trie is not prefix free on character 'a'"
        );

        err = err.cause().unwrap();
        assert_eq!(
            format!("{}", err),
            "ECDFA generation error: Failed to build CDFA: \
            Transition trie is not prefix free on character 'a'"
        );

        err = err.cause().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to build CDFA: Transition trie is not prefix free on character 'a'"
        );

        assert!(err.cause().is_none());
    }

    #[test]
    fn failed_range_based_matchers_overlap() {
        //setup
        let spec = "
'abcdefghijklmnopqrstuvwxyz'

start
    'a' .. 'l' -> ^FIRST
    'l' .. 'z' -> ^LAST;

s -> ;
    ".to_string();

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

        err = err.cause().unwrap();
        assert_eq!(
            format!("{}", err),
            "ECDFA generation error: Failed to build CDFA: \
            Transition trie is not prefix free on character 'l'"
        );

        err = err.cause().unwrap();
        assert_eq!(
            format!("{}", err),
            "Failed to build CDFA: Transition trie is not prefix free on character 'l'"
        );

        assert!(err.cause().is_none());
    }

    #[test]
    fn failed_range_based_matchers_invalid_start() {
        //setup
        let spec = "
''

start
    'aa'..'b' -> ^A;

s ->;
    ".to_string();

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

        err = err.cause().unwrap();
        assert_eq!(
            format!("{}", err),
            "Matcher definition error: Range start must be one character, but was 'aa'"
        );

        assert!(err.cause().is_none());
    }

    #[test]
    fn failed_range_based_matchers_invalid_end() {
        //setup
        let spec = "
''

start
    'a'..'cd' -> ^A;

s ->;
    ".to_string();

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

        err = err.cause().unwrap();
        assert_eq!(
            format!("{}", err),
            "Matcher definition error: Range end must be one character, but was 'cd'"
        );

        assert!(err.cause().is_none());
    }

    #[test]
    fn failed_orphaned_terminal() {
        //setup
        let spec = "
'ab'

start
    'a' -> ^A
    'b' -> ^B;

s -> ORPHANED;
    ".to_string();

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

        err = err.cause().unwrap();
        assert_eq!(
            format!("{}", err),
            "ECDFA to grammar mapping error: \
            Orphaned terminal 'ORPHANED' is not tokenized by the ECDFA"
        );

        assert!(err.cause().is_none());
    }

    //TODO test failures for re-mapping acceptances
}
