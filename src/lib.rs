#[macro_use]
extern crate lazy_static;
extern crate stopwatch;

use core::scan;
use core::scan::DFA;
use core::scan::Scanner;
use core::parse;
use core::parse::Grammar;
use core::parse::Parser;
use core::fmt::Formatter;
use core::spec;
use core::Error;

mod core;

pub struct FormatJobRunner {
    dfa: DFA,
    grammar: Grammar,
    formatter: Formatter,
    scanner: Box<Scanner>,
    parser: Box<Parser>,
}

impl FormatJobRunner {
    pub fn build(spec: &String) -> Result<FormatJobRunner, String> {
        match spec::parse_spec(spec) {
            Ok(parse) => {
                match spec::generate_spec(&parse) {
                    Ok((dfa, grammar, formatter)) => Ok(FormatJobRunner {
                        dfa,
                        grammar,
                        formatter,
                        scanner: scan::def_scanner(),
                        parser: parse::def_parser(),
                    }),
                    Err(e) => Err(e.to_string())
                }
            },
            Err(e) => Err(e.to_string())
        }
    }

    pub fn format(&self, input: &String) -> Result<String, String> {
        let res = self.scanner.scan(input, &self.dfa);
        match res {
            Ok(tokens) => {
                let tree = self.parser.parse(tokens, &self.grammar);
                match tree {
                    Some(parse) => Ok(self.formatter.format(&parse)),
                    None => Err(Error::ParseErr().to_string()),
                }
            },
            Err(se) => Err(Error::ScanErr(se).to_string()),
        }
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
        assert_eq!(res.err().unwrap(), "Failed to scan input: No accepting scans after (1,1): b...");
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
        assert_eq!(res.err().unwrap(), "Failed to parse input");
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
        assert_eq!(res.err().unwrap(), "Failed to scan input: No accepting scans after (2,5): ~\n\nstart \'...");
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
        assert_eq!(res.err().unwrap(), "Failed to parse input");
    }
}