#[macro_use]
extern crate lazy_static;

extern crate stopwatch;

use core::scan::def_scanner;
use core::parse::def_parser;
use core::parse::build_prods;
use core::scan::DFA;
use core::scan::State;
use core::parse::Grammar;
use core::fmt::PatternPair;
use core::fmt::FormatJob;
use stopwatch::{Stopwatch};

mod core;

pub fn test() {
    let input = "  {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }  ";

    let sw = Stopwatch::start_new();

    //setup dfa
    let alphabet = "{} \t\n";
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

    println!("STARTING SCAN {}ms", sw.elapsed_ms());
    let tokens = scanner.scan(&input, &dfa);
    println!("FINISHED SCAN {}ms", sw.elapsed_ms());
    let res = parser.parse(tokens, &grammar);
    println!("FINISHED PARSE {}ms", sw.elapsed_ms());

    if res.is_none() {
        panic!("Failed Parse");
    }
    let tree = res.unwrap();

    //print
    println!("Parsed Tree:");
    //tree.print();

    //patterns
    let patterns: &[PatternPair] = &[
        PatternPair {
            production: "w WHITESPACE".to_string(),
            pattern: "",
        },
        PatternPair{
            production: "b LBRACKET s RBRACKET".to_string(),
            pattern: "[prefix]{0}\n\n{1;prefix=[prefix]\t}[prefix]{2}\n\n",
        }
    ];

    let fmt_job = FormatJob::create(&tree, patterns);
    let result = fmt_job.run();
    println!("{}\n-------------FORMATTED AS-------------\n{}", input, result);

    println!("Parses took {}ms", sw.elapsed_ms());
}