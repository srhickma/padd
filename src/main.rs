use std::io;
use core::scan::def_scanner;
use core::scan::DFA;
use core::scan::Token;
use core::scan::State;
use core::parse::def_parser;
use core::parse::Grammar;
use core::parse::Production;

mod cli;
mod core;

fn main() {
    let grammar = Grammar::from(&[
        Production{
            lhs: "Sum",
            rhs: &["Sum", "AS", "Product"],
        },
        Production{
            lhs: "Sum",
            rhs: &["Product"],
        },
        Production{
            lhs: "Product",
            rhs: &["Product", "MD", "Factor"],
        },
        Production{
            lhs: "Product",
            rhs: &["Factor"],
        },
        Production{
            lhs: "Factor",
            rhs: &["LPAREN", "Sum", "RPAREN"],
        },
        Production{
            lhs: "Factor",
            rhs: &["Number"],
        },
        Production{
            lhs: "Number",
            rhs: &["NUM"],
        }
    ]);

    let scan = "NUM AS LPAREN NUM MD NUM AS NUM RPAREN".split_whitespace()
        .map(|kind| Token{
            kind: kind.to_string(),
            lexeme: "".to_string(),
        },).collect();

//    let mut grammar = Grammar::from(&[
//        Production{
//            lhs: "S",
//            rhs: &["expr"],
//        },
//        Production{
//            lhs: "expr",
//            rhs: &["(", "expr", ")"],
//        },
//        Production{
//            lhs: "expr",
//            rhs: &["expr", "OP", "expr"],
//        },
//        Production{
//            lhs: "expr",
//            rhs: &["ID"],
//        }
//    ]);
//
//    let scan = "( ID OP ID ) OP ID OP ( ID )".split_whitespace()
//        .map(|kind| Token{
//            kind: kind.to_string(),
//            lexeme: "".to_string(),
//        },).collect();

//    let mut grammar = Grammar::from(&[
//        Production{
//            lhs: "S",
//            rhs: &["BOF", "A", "EOF"],
//        },
//        Production{
//            lhs: "A",
//            rhs: &["x"],
//        },
//        Production{
//            lhs: "A",
//            rhs: &["A", "x"],
//        }
//    ]);
//
//    let scan = vec![
//        Token{
//            kind: "BOF".to_string(),
//            lexeme: "".to_string(),
//        },
//        Token{
//            kind: "x".to_string(),
//            lexeme: "".to_string(),
//        },
//        Token{
//            kind: "EOF".to_string(),
//            lexeme: "".to_string(),
//        }
//    ];

//    let grammar = Grammar::from(&[
//        Production{
//            lhs: "Sentence",
//            rhs: &["Noun", "Verb"],
//        },
//        Production{
//            lhs: "Noun",
//            rhs: &["mary"],
//        },
//        Production{
//            lhs: "Verb",
//            rhs: &["runs"],
//        }
//    ]);
//
//    let scan = vec![
//        Token{
//            kind: "mary".to_string(),
//            lexeme: "Hello".to_string(),
//        },
//        Token{
//            kind: "runs".to_string(),
//            lexeme: "World!".to_string(),
//        }
//    ];

    let parser = def_parser();

    let res = parser.parse(scan, &grammar);

    res.unwrap().print();

    let alphabet = "01";
    let states: [State; 3] = ["start", "0", "not0"];
    let start: State = "start";
    let accepting: [State; 2] = ["0", "not0"];
    let delta: fn(State, char) -> State = |state, c| match (state, c) {
        ("start", '0') => "0",
        ("start", '1') => "not0",
        ("not0", _) => "not0",
        (&_, _) => "",
    };
    let tokenizer: fn(State) -> &str = |state| match state {
        "0" => "ZERO",
        "not0" => "NZ",
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

    loop {
        println!("Input some string");

        let mut input = String::new();

        io::stdin().read_line(&mut input)
            .expect("Failed to read line");

        input.pop(); //Remove trailing newline

        let scanner = def_scanner();

        let tokens = scanner.scan(&input, &dfa);

        println!("Scanned Tokens: {}", tokens.len());

        for token in tokens {
            println!("kind={} lexeme={}", token.kind, token.lexeme)
        }
    }
}
