#[macro_use]
extern crate lazy_static;
extern crate stopwatch;

use core::scan::DFA;
use core::parse::Grammar;
use core::parse::Tree;
use core::fmt::Formatter;
use core::spec;

mod core;

//struct FormatJobRunner<'a> {
//    dfa: DFA,
//    grammar: Grammar<'a>,
//    formatter: Formatter,
//}
//
//impl<'a> FormatJobRunner<'a> {
//    pub fn build(spec: &String) -> FormatJobRunner {
//        let tree = spec::parse_spec(spec);
//        let parse = tree.unwrap();
//        let (dfa, grammar, formatter) = spec::generate_spec(&parse);
//        FormatJobRunner{
//            dfa,
//            grammar: grammar.clone(),
//            formatter,
//        }
//    }
//}

pub fn test() {

}