#[macro_use]
extern crate lazy_static;

use core::scan::def_scanner;
use core::parse::def_parser;
use core::parse::build_prods;
use core::scan::DFA;
use core::scan::State;
use core::parse::Grammar;
use core::parse::Tree;
use core::fmt::PatternPair;
use core::fmt::FormatJob;

use std::collections::HashMap;

mod cli;
mod core;

fn main() {
    //let input = "  {{} \t {{{{{{{{{{\n}}}}\n}}}}}{}}  {}{   }\t { } {\t}{ }}  {}  {}{   } { } {}{ }\n";
    let input = "  {  {    }  }   {  }  ";
    //let input = "{}";

    //setup dfa
    let alphabet = "{} \t\n";
    let states: [State; 4] = ["start", "lbr", "rbr", "ws"];
    let start: State = "start";
    let accepting: [State; 3] = ["lbr", "rbr", "ws"];
    let delta: fn(State, char) -> State = |state, c| match (state, c) {
        ("start", ' ') => "ws",
        ("start", '\t') => "ws",
        ("start", '\n') => "ws",
        ("start", '{') => "lbr",
        ("start", '}') => "rbr",
        ("ws", ' ') => "ws",
        ("ws", '\t') => "ws",
        ("ws", '\n') => "ws",
        (&_, _) => "",
    };
    let tokenizer: fn(State) -> &str = |state| match state {
        "lbr" => "LBRACKET",
        "rbr" => "RBRACKET",
        "ws" => "WHITESPACE",
        _ => "",
    };

    let dfa = DFA{
        alphabet: &alphabet,
        states: &states,
        start,
        accepting: &accepting,
        delta,
        tokenizer
    };

    //setup grammar
    let productions = build_prods(&[
        "s s b",
        "s ",
        "b LBRACKET s RBRACKET",
        "b w",
        "w WHITESPACE",
    ]);
    let grammar = Grammar::from(&productions[..]);

    //scan and parse
    let scanner = def_scanner();
    let parser = def_parser();

    let tokens = scanner.scan(&input, &dfa);
    let res = parser.parse(tokens, &grammar);

    if res.is_none() {
        panic!("Failed Parse");
    }
    let tree = res.unwrap();

    //print
    println!("Parsed Tree:");
    tree.print();

    //patterns
    let patterns: &[PatternPair] = &[
        PatternPair {
            production: "w WHITESPACE".to_string(),
            pattern: "",
        },
        PatternPair{
            production: "b LBRACKET s RBRACKET".to_string(),
            pattern: "{0}\n{1}\n{2}\n",
        }
    ];

    let fmt_job = FormatJob::create(&tree, patterns);
    let result = fmt_job.run();
    println!("{}\n-------------FORMATTED AS-------------\n{}", input, result);
}