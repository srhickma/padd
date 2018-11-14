use std::error;
use std::fmt;

use core::data::Data;
use core::parse::grammar::Grammar;
use core::scan::Token;

mod earley;
mod leo;
pub mod grammar;

pub trait Parser: 'static + Send + Sync {
    fn parse(&self, scan: Vec<Token<String>>, grammar: &Grammar) -> Result<Tree, Error>;
}

pub fn def_parser() -> Box<Parser> {
    Box::new(earley::EarleyParser)
    //Box::new(leo::LeoParser)
}

#[derive(Clone, PartialEq, Debug)]
pub struct Tree {
    pub lhs: Token<String>,
    pub children: Vec<Tree>,
}

impl Tree {
    pub fn get_child(&self, i: usize) -> &Tree {
        self.children.get(i).unwrap()
    }

    pub fn is_leaf(&self) -> bool {
        self.children.len() == 0
    }

    pub fn is_empty(&self) -> bool {
        self.children.len() == 1 && self.get_child(0).is_null()
    }

    pub fn is_null(&self) -> bool {
        self.lhs.kind == ""
    }

    pub fn null() -> Tree {
        Tree {
            lhs: Token {
                kind: "".to_string(),
                lexeme: "NULL".to_string(),
            },
            children: vec![],
        }
    }

    fn to_string_internal(&self, prefix: String, is_tail: bool) -> String {
        if self.children.len() == 0 {
            format!("{}{}{}", prefix, if is_tail { "└── " } else { "├── " }, self.lhs.to_string())
        } else {
            let mut s = format!("{}{}{}", prefix, if is_tail { "└── " } else { "├── " }, self.lhs.kind);
            let mut i = 0;
            let len = self.children.len();
            for child in &self.children {
                if i == len - 1 {
                    s = format!("{}\n{}", s, child.to_string_internal(format!("{}{}", prefix, if is_tail { "    " } else { "│   " }), true));
                } else {
                    s = format!("{}\n{}", s, child.to_string_internal(format!("{}{}", prefix, if is_tail { "    " } else { "│   " }), false));
                }
                i += 1;
            }
            s
        }
    }

    pub fn production(&self) -> String {
        let vec: Vec<String> = self.children.iter()
            .map(|s| s.lhs.kind.clone())
            .collect();
        format!("{} {}", self.lhs.kind, (&vec[..]).join(" "))
    }
}

impl Data for Tree {
    fn to_string(&self) -> String {
        self.to_string_internal("".to_string(), true)
    }
}

#[derive(Debug)]
pub struct Error {
    pub message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl error::Error for Error {
    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

#[derive(PartialEq, Clone, Debug)]
pub struct Production {
    pub lhs: String,
    pub rhs: Vec<String>,
}

impl Data for Production {
    fn to_string(&self) -> String {
        format!("{} {}", self.lhs, (&self.rhs[..]).join(" "))
    }
}

pub fn build_prods<'a>(strings: &'a [&'a str]) -> Vec<Production> {
    let mut productions: Vec<Production> = vec![];
    for string in strings {
        productions.push(prod_from_string(string));
    }
    productions
}

fn prod_from_string(string: &str) -> Production {
    let mut i = 0;
    let mut lhs = String::new();
    let mut rhs: Vec<String> = vec![];

    for s in string.split_whitespace() {
        if i == 0 {
            lhs = s.to_string();
        } else {
            rhs.push(s.to_string());
        }
        i += 1;
    }

    Production {
        lhs,
        rhs,
    }
}

#[cfg(test)]
mod tests {
    use core::parse::grammar::GrammarBuilder;

    use super::*;

    #[test]
    fn parse_example() {
        //setup
        let mut grammar_builder = GrammarBuilder::new();
        grammar_builder.add_productions(build_prods(&[
            "Sentence Noun Verb",
            "Noun mary",
            "Verb runs"
        ]));
        grammar_builder.try_mark_start("Sentence");
        let grammar = grammar_builder.build();

        let scan = vec![
            Token {
                kind: "mary".to_string(),
                lexeme: "Hello".to_string(),
            },
            Token {
                kind: "runs".to_string(),
                lexeme: "World!".to_string(),
            }
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(scan, &grammar);

        //verify
        assert_eq!(tree.unwrap().to_string(),
                   "└── Sentence
    ├── Noun
    │   └── mary <- 'Hello'
    └── Verb
        └── runs <- 'World!'"
        );
    }

    #[test]
    fn parse_simple() {
        //setup
        let mut grammar_builder = GrammarBuilder::new();
        grammar_builder.add_productions(build_prods(&[
            "S BOF A EOF",
            "A x",
            "A A x"
        ]));
        grammar_builder.try_mark_start("S");
        let grammar = grammar_builder.build();

        let scan = vec![
            Token {
                kind: "BOF".to_string(),
                lexeme: "a".to_string(),
            },
            Token {
                kind: "x".to_string(),
                lexeme: "b".to_string(),
            },
            Token {
                kind: "EOF".to_string(),
                lexeme: "c".to_string(),
            }
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(scan, &grammar);

        //verify
        assert_eq!(tree.unwrap().to_string(),
                   "└── S
    ├── BOF <- 'a'
    ├── A
    │   └── x <- 'b'
    └── EOF <- 'c'"
        );
    }

    #[test]
    fn parse_expressions() {
        //setup
        let mut grammar_builder = GrammarBuilder::new();
        grammar_builder.add_productions(build_prods(&[
            "S expr",
            "S S OP expr",
            "expr ( S )",
            "expr ID",
        ]));
        grammar_builder.try_mark_start("S");
        let grammar = grammar_builder.build();

        let scan = "( ID OP ID ) OP ID OP ( ID )".split_whitespace()
            .map(|kind| Token {
                kind: kind.to_string(),
                lexeme: "xy".to_string(),
            }, ).collect();

        let parser = def_parser();

        //exercise
        let tree = parser.parse(scan, &grammar);

        //verify
        assert_eq!(tree.unwrap().to_string(),
                   "└── S
    ├── S
    │   ├── S
    │   │   └── expr
    │   │       ├── ( <- 'xy'
    │   │       ├── S
    │   │       │   ├── S
    │   │       │   │   └── expr
    │   │       │   │       └── ID <- 'xy'
    │   │       │   ├── OP <- 'xy'
    │   │       │   └── expr
    │   │       │       └── ID <- 'xy'
    │   │       └── ) <- 'xy'
    │   ├── OP <- 'xy'
    │   └── expr
    │       └── ID <- 'xy'
    ├── OP <- 'xy'
    └── expr
        ├── ( <- 'xy'
        ├── S
        │   └── expr
        │       └── ID <- 'xy'
        └── ) <- 'xy'"
        );
    }

    #[test]
    fn parse_lacs_math() {
        //setup
        let mut grammar_builder = GrammarBuilder::new();
        grammar_builder.add_productions(build_prods(&[
            "Sum Sum AS Product",
            "Sum Product",
            "Product Product MD Factor",
            "Product Factor",
            "Factor LPAREN Sum RPAREN",
            "Factor Number",
            "Number NUM",
        ]));
        grammar_builder.try_mark_start("Sum");
        let grammar = grammar_builder.build();

        let scan = "NUM AS LPAREN NUM MD NUM AS NUM RPAREN".split_whitespace()
            .map(|kind| Token {
                kind: kind.to_string(),
                lexeme: "xy".to_string(),
            }, ).collect();

        let parser = def_parser();

        //exercise
        let tree = parser.parse(scan, &grammar);

        //verify
        assert_eq!(tree.unwrap().to_string(),
                   "└── Sum
    ├── Sum
    │   └── Product
    │       └── Factor
    │           └── Number
    │               └── NUM <- 'xy'
    ├── AS <- 'xy'
    └── Product
        └── Factor
            ├── LPAREN <- 'xy'
            ├── Sum
            │   ├── Sum
            │   │   └── Product
            │   │       ├── Product
            │   │       │   └── Factor
            │   │       │       └── Number
            │   │       │           └── NUM <- 'xy'
            │   │       ├── MD <- 'xy'
            │   │       └── Factor
            │   │           └── Number
            │   │               └── NUM <- 'xy'
            │   ├── AS <- 'xy'
            │   └── Product
            │       └── Factor
            │           └── Number
            │               └── NUM <- 'xy'
            └── RPAREN <- 'xy'"
        );
    }

    #[test]
    fn parse_brackets() {
        //setup
        let mut grammar_builder = GrammarBuilder::new();
        grammar_builder.add_productions(build_prods(&[
            "s s b",
            "s ",
            "b LBRACKET s RBRACKET",
            "b w",
            "w WHITESPACE",
        ]));
        grammar_builder.try_mark_start("s");
        let grammar = grammar_builder.build();

        let scan = "WHITESPACE LBRACKET WHITESPACE LBRACKET WHITESPACE RBRACKET WHITESPACE RBRACKET LBRACKET RBRACKET WHITESPACE".split_whitespace()
            .map(|kind| Token {
                kind: kind.to_string(),
                lexeme: "xy".to_string(),
            }, ).collect();

        let parser = def_parser();

        //exercise
        let tree = parser.parse(scan, &grammar);

        //verify
        assert_eq!(tree.unwrap().to_string(),
                   "└── s
    ├── s
    │   ├── s
    │   │   ├── s
    │   │   │   ├── s
    │   │   │   │   └──  <- 'NULL'
    │   │   │   └── b
    │   │   │       └── w
    │   │   │           └── WHITESPACE <- 'xy'
    │   │   └── b
    │   │       ├── LBRACKET <- 'xy'
    │   │       ├── s
    │   │       │   ├── s
    │   │       │   │   ├── s
    │   │       │   │   │   ├── s
    │   │       │   │   │   │   └──  <- 'NULL'
    │   │       │   │   │   └── b
    │   │       │   │   │       └── w
    │   │       │   │   │           └── WHITESPACE <- 'xy'
    │   │       │   │   └── b
    │   │       │   │       ├── LBRACKET <- 'xy'
    │   │       │   │       ├── s
    │   │       │   │       │   ├── s
    │   │       │   │       │   │   └──  <- 'NULL'
    │   │       │   │       │   └── b
    │   │       │   │       │       └── w
    │   │       │   │       │           └── WHITESPACE <- 'xy'
    │   │       │   │       └── RBRACKET <- 'xy'
    │   │       │   └── b
    │   │       │       └── w
    │   │       │           └── WHITESPACE <- 'xy'
    │   │       └── RBRACKET <- 'xy'
    │   └── b
    │       ├── LBRACKET <- 'xy'
    │       ├── s
    │       │   └──  <- 'NULL'
    │       └── RBRACKET <- 'xy'
    └── b
        └── w
            └── WHITESPACE <- 'xy'"
        );
    }

    #[test]
    fn parse_deep_epsilon() {
        //setup
        let mut grammar_builder = GrammarBuilder::new();
        grammar_builder.add_productions(build_prods(&[
            "s w w w w",
            "w WHITESPACE",
            "w ",
        ]));
        grammar_builder.try_mark_start("s");
        let grammar = grammar_builder.build();

        let scan = "WHITESPACE".split_whitespace()
            .map(|kind| Token {
                kind: kind.to_string(),
                lexeme: "xy".to_string(),
            }, ).collect();

        let parser = def_parser();

        //exercise
        let tree = parser.parse(scan, &grammar);

        //verify
        assert_eq!(tree.unwrap().to_string(),
                   "└── s
    ├── w
    │   └──  <- 'NULL'
    ├── w
    │   └──  <- 'NULL'
    ├── w
    │   └──  <- 'NULL'
    └── w
        └── WHITESPACE <- 'xy'"
        );
    }

    #[test]
    fn advanced_parse_build() {
        //setup
        let mut grammar_builder = GrammarBuilder::new();
        grammar_builder.add_productions(build_prods(&[
            "sum sum PM prod",
            "sum prod",
            "prod prod MD fac",
            "prod fac",
            "fac LPAREN sum RPAREN",
            "fac num",
            "num DIGIT num",
            "num DIGIT",
        ]));
        grammar_builder.try_mark_start("sum");
        let grammar = grammar_builder.build();

        let scan = vec![
            Token { kind: "DIGIT".to_string(), lexeme: "1".to_string() },
            Token { kind: "PM".to_string(), lexeme: "+".to_string() },
            Token { kind: "LPAREN".to_string(), lexeme: "(".to_string() },
            Token { kind: "DIGIT".to_string(), lexeme: "2".to_string() },
            Token { kind: "MD".to_string(), lexeme: "*".to_string() },
            Token { kind: "DIGIT".to_string(), lexeme: "3".to_string() },
            Token { kind: "PM".to_string(), lexeme: "-".to_string() },
            Token { kind: "DIGIT".to_string(), lexeme: "4".to_string() },
            Token { kind: "RPAREN".to_string(), lexeme: ")".to_string() },
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(scan, &grammar);

        //verify
        assert_eq!(tree.unwrap().to_string(),
                   "└── sum
    ├── sum
    │   └── prod
    │       └── fac
    │           └── num
    │               └── DIGIT <- '1'
    ├── PM <- '+'
    └── prod
        └── fac
            ├── LPAREN <- '('
            ├── sum
            │   ├── sum
            │   │   └── prod
            │   │       ├── prod
            │   │       │   └── fac
            │   │       │       └── num
            │   │       │           └── DIGIT <- '2'
            │   │       ├── MD <- '*'
            │   │       └── fac
            │   │           └── num
            │   │               └── DIGIT <- '3'
            │   ├── PM <- '-'
            │   └── prod
            │       └── fac
            │           └── num
            │               └── DIGIT <- '4'
            └── RPAREN <- ')'"
        );
    }

    #[test]
    fn parse_must_consume() {
        //setup
        let mut grammar_builder = GrammarBuilder::new();
        grammar_builder.add_productions(build_prods(&["s "]));
        grammar_builder.try_mark_start("s");
        let grammar = grammar_builder.build();

        let scan = vec![Token {
            kind: "kind".to_string(),
            lexeme: "lexeme".to_string(),
        }];

        let parser = def_parser();

        //exercise
        let res = parser.parse(scan, &grammar);

        //verify
        assert!(res.is_err());
        assert_eq!(format!("{}", res.err().unwrap()), "Largest parse did not consume all tokens: 0 of 1")
    }
}
