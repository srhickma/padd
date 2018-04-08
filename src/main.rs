#[macro_use]
extern crate lazy_static;

use std::io;
use std::collections::HashMap;
use core::scan::def_scanner;
use core::scan::DFA;
use core::scan::Token;
use core::scan::Kind;
use core::scan::State;
use core::parse::def_parser;
use core::parse::build_prods;
use core::parse::Grammar;
use core::parse::Production;
use core::parse::Tree;

mod cli;
mod core;

fn main() {
    //let input = "  {{} \t {{{{{{{{{{\n}}}}\n}}}}}{}}  {}{   }\t { } {\t}{ }}  {}  {}{   } { } {}{ }\n";
    let input = "{{}}{}";
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

    //reconstruct
    println!("Reconstructed:\n{}", recon(&tree, |prod, cur, next| {
//        return match (&mk[..], &ck[..]) {
//            (_, "LBRACKET") => format!("{}{}\n", cur, next),
//            (_, "RBRACKET") => format!("{}{}\n", cur, next),
//            (_, _) => format!("{}{}", cur, next),
//        }
//        return match &prod[..] {
//            "b -> LBRACKET s RBRACKET" => "{}\n{}{}\n",
//            (_, "RBRACKET") => format!("{}{}\n", cur, next),
//            (_, _) => format!("{}{}", cur, next),
//        }
        return String::new();
    }));
}

fn recon(tree: &Tree, formatter: fn(&String, &String, String) -> String) -> String {
    if tree.children.len() == 0 {
        if tree.lhs.kind == "" {
            return String::new();
        }
        return tree.lhs.lexeme.clone();
    }
    let mut res = String::new();
    for child in &tree.children {
        println!("{}", tree.production());
        //res = formatter(&tree.lhs.kind, &child.lhs.kind, &res, recon(child, formatter)); //This is where we add custom formatting
    }
    return res;
}