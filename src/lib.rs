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

mod core;

pub struct FormatJobRunner {
    dfa: DFA,
    grammar: Grammar,
    formatter: Formatter,
    scanner: Box<Scanner>,
    parser: Box<Parser>,
}

impl FormatJobRunner {
    pub fn build(spec: &String) -> FormatJobRunner {
        let tree = spec::parse_spec(spec);
        let parse = tree.unwrap();
        let (dfa, grammar, formatter) = spec::generate_spec(&parse);
        FormatJobRunner{
            dfa,
            grammar,
            formatter,
            scanner: scan::def_scanner(),
            parser: parse::def_parser(),
        }
    }

    pub fn format(&self, input: &String) -> Result<String, &str> {
        let tokens = self.scanner.scan(input, &self.dfa);
        if tokens.is_none() {
            return Err("Failed to scan input");
        }
        let tree = self.parser.parse(tokens.unwrap(), &self.grammar);
        match tree {
            Some(parse) => Ok(self.formatter.format(&parse)),
            None => Err("Failed to parse input"),
        }
    }
}