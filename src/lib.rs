#[macro_use]
extern crate lazy_static;

extern crate stopwatch;

use core::scan::def_scanner;
use core::parse::def_parser;
use core::parse::build_prods;
use core::scan::DFA;
use core::scan::CompileTransitionDelta;
use core::scan::State;
use core::parse::Grammar;
use core::fmt::PatternPair;
use core::fmt::FormatJob;
use stopwatch::{Stopwatch};

mod core;

pub fn test() {
//    let input = "  {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }    {  {  {{{\t}}}\n {} }  }   { {}\n }  ";
//
//    let sw = Stopwatch::start_new();
//
//    //setup dfa
//    let alphabet = "{} \t\n".to_string();
//    let start: State = "start".to_string();
//    let delta: fn(&str, char) -> &String = |state, c| match (state, c) {
//        ("start", ' ') => &"ws".to_string(),
//        ("start", '\t') => &"ws".to_string(),
//        ("start", '\n') => &"ws".to_string(),
//        ("start", '{') => &"lbr".to_string(),
//        ("start", '}') => &"rbr".to_string(),
//        ("ws", ' ') => &"ws".to_string(),
//        ("ws", '\t') => &"ws".to_string(),
//        ("ws", '\n') => &"ws".to_string(),
//        (&_, _) => &"".to_string(),
//    };
//    let tokenizer: fn(&str) -> &'static str = |state| match state {
//        "lbr" => "LBRACKET",
//        "rbr" => "RBRACKET",
//        "ws" => "WHITESPACE",
//        _ => "",
//    };
//
//    let dfa = DFA{
//        alphabet,
//        start,
//        td: Box::new(CompileTransitionDelta{
//            delta,
//            tokenizer,
//        }),
//    };
//
//    //setup grammar
//    let productions = build_prods(&[
//        "s s b",
//        "s ",
//        "b LBRACKET s RBRACKET",
//        "b w",
//        "w WHITESPACE",
//    ]);
//    let grammar = Grammar::from(&productions[..]);
//
//    //scan and parse
//    let scanner = def_scanner();
//    let parser = def_parser();
//
//    println!("STARTING SCAN {}ms", sw.elapsed_ms());
//    let tokens = scanner.scan(&input, &(Box::new(dfa) as Box<DFA>));
//    println!("FINISHED SCAN {}ms", sw.elapsed_ms());
//    let res = parser.parse(tokens, &grammar);
//    println!("FINISHED PARSE {}ms", sw.elapsed_ms());
//
//    if res.is_none() {
//        panic!("Failed Parse");
//    }
//    let tree = res.unwrap();
//
//    //print
//    println!("Parsed Tree:");
//    //tree.print();
//
//    //patterns
//    let patterns: &[PatternPair] = &[
//        PatternPair {
//            production: "w WHITESPACE".to_string(),
//            pattern: "",
//        },
//        PatternPair{
//            production: "b LBRACKET s RBRACKET".to_string(),
//            pattern: "[prefix]{0}\n\n{1;prefix=[prefix]\t}[prefix]{2}\n\n",
//        }
//    ];
//
//    let fmt_job = FormatJob::create(&tree, patterns);
//    let result = fmt_job.run();
//    println!("{}\n-------------FORMATTED AS-------------\n{}", input, result);
//
//    println!("Parses took {}ms", sw.elapsed_ms());
}