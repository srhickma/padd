use std::collections::HashSet;
use core::scan::Token;

mod earley;

pub trait Parser {
    fn parse(&self, scan: Vec<Token>, grammar: &Grammar) -> Option<Tree>;
}

pub fn def_parser() -> Box<Parser> {
    return Box::new(earley::EarleyParser);
}

#[derive(Clone)]
pub struct Tree {
    pub lhs: Token,
    pub children: Vec<Tree>,
}

impl Tree {
    pub fn get_child(&self, i: usize) -> &Tree {
        return self.children.get(i).unwrap();
    }
    pub fn is_leaf(&self) -> bool {
        return self.children.len() == 0;
    }
    pub fn is_empty(&self) -> bool {
        return self.children.len() == 1 && self.get_child(0).is_null();
    }
    pub fn is_null(&self) -> bool {
        return self.lhs.kind == "";
    }
    pub fn null() -> Tree {
        return Tree{
            lhs: Token{
                kind: "".to_string(),
                lexeme: "NULL".to_string(),
            },
            children: vec![],
        }
    }
    #[allow(dead_code)]
    pub fn print(&self){
        println!("{}", self.to_string());
    }
    #[allow(dead_code)]
    pub fn to_string(&self) -> String {
        return self.to_string_internal("".to_string(), true)
    }
    #[allow(dead_code)]
    fn to_string_internal(&self, prefix: String, is_tail: bool) -> String {
        if self.children.len() == 0 {
            return format!("{}{}{}", prefix, if is_tail {"└── "} else {"├── "}, self.lhs.to_string());
        }
        else {
            let mut s = format!("{}{}{}", prefix, if is_tail {"└── "} else {"├── "}, self.lhs.kind);
            let mut i = 0;
            let len = self.children.len();
            for child in &self.children {
                if i == len - 1{
                    s = format!("{}\n{}", s, child.to_string_internal(format!("{}{}", prefix, if is_tail {"    "} else {"│   "}), true));
                } else {
                    s = format!("{}\n{}", s, child.to_string_internal(format!("{}{}", prefix, if is_tail {"    "} else {"│   "}), false));
                }
                i += 1;
            }
            return s;
        }
    }
    pub fn production(&self) -> String {
        let vec: Vec<String> = self.children.iter().map(|s| s.lhs.kind.clone()).collect();
        return format!("{} {}", self.lhs.kind, (&vec[..]).join(" "));
    }
}

pub struct Grammar {
    pub productions: Vec<Production>,
    #[allow(dead_code)]
    non_terminals: HashSet<String>,
    terminals: HashSet<String>,
    #[allow(dead_code)]
    symbols: HashSet<String>,
    start: String,
}

impl Grammar {
    pub fn from(productions: Vec<Production>) -> Grammar {
        let non_terminals: HashSet<String> = productions.iter().cloned()
            .map(|prod| prod.lhs)
            .collect();
        let mut symbols: HashSet<String> = productions.iter()
            .flat_map(|prod| prod.rhs.iter())
            .map(|x| x.clone())
            .collect();
        for non_terminal in &non_terminals {
            symbols.insert(non_terminal.clone());
        }
        let terminals = symbols.difference(&non_terminals)
            .map(|x| x.clone())
            .collect();

        let start = productions[0].lhs.clone();

        return Grammar {
            productions,
            non_terminals,
            terminals,
            symbols,
            start,
        };
    }
}

#[derive(PartialEq, Clone)]
pub struct Production {
    pub lhs: String,
    pub rhs: Vec<String>,
}

impl Production {
    #[allow(dead_code)]
    pub fn to_string(&self) -> String {
        format!("{} {}", self.lhs, (&self.rhs[..]).join(" "))
    }
}

pub fn build_prods<'a>(strings: &'a[&'a str]) -> Vec<Production> {
    let mut productions: Vec<Production> = vec![];
    for string in strings {
        productions.push(prod_from_string(string));
    }
    return productions;
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

    return Production{
        lhs,
        rhs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_example() {
        //setup
        let productions = build_prods(&[
            "Sentence Noun Verb",
            "Noun mary",
            "Verb runs"
        ]);
        let grammar = Grammar::from(productions);

        let scan = vec![
            Token{
                kind: "mary".to_string(),
                lexeme: "Hello".to_string(),
            },
            Token{
                kind: "runs".to_string(),
                lexeme: "World!".to_string(),
            }
        ];

        let parser = def_parser();

        //execute
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
        let productions = build_prods(&[
            "S BOF A EOF",
            "A x",
            "A A x"
        ]);
        let grammar = Grammar::from(productions);

        let scan = vec![
            Token{
                kind: "BOF".to_string(),
                lexeme: "a".to_string(),
            },
            Token{
                kind: "x".to_string(),
                lexeme: "b".to_string(),
            },
            Token{
                kind: "EOF".to_string(),
                lexeme: "c".to_string(),
            }
        ];

        let parser = def_parser();

        //execute
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
        let productions = build_prods(&[
            "S expr",
            "expr ( expr )",
            "expr expr OP expr",
            "expr ID",
        ]);
        let grammar = Grammar::from(productions);

        let scan = "( ID OP ID ) OP ID OP ( ID )".split_whitespace()
            .map(|kind| Token{
                kind: kind.to_string(),
                lexeme: "xy".to_string(),
            },).collect();

        let parser = def_parser();

        //execute
        let tree = parser.parse(scan, &grammar);

        //verify
        assert_eq!(tree.unwrap().to_string(),
"└── S
    └── expr
        ├── expr
        │   ├── expr
        │   │   ├── ( <- 'xy'
        │   │   ├── expr
        │   │   │   ├── expr
        │   │   │   │   └── ID <- 'xy'
        │   │   │   ├── OP <- 'xy'
        │   │   │   └── expr
        │   │   │       └── ID <- 'xy'
        │   │   └── ) <- 'xy'
        │   ├── OP <- 'xy'
        │   └── expr
        │       └── ID <- 'xy'
        ├── OP <- 'xy'
        └── expr
            ├── ( <- 'xy'
            ├── expr
            │   └── ID <- 'xy'
            └── ) <- 'xy'"
        );
    }

    #[test]
    fn parse_lacs_math() {
        //setup
        let productions = build_prods(&[
            "Sum Sum AS Product",
            "Sum Product",
            "Product Product MD Factor",
            "Product Factor",
            "Factor LPAREN Sum RPAREN",
            "Factor Number",
            "Number NUM",
        ]);
        let grammar = Grammar::from(productions);

        let scan = "NUM AS LPAREN NUM MD NUM AS NUM RPAREN".split_whitespace()
            .map(|kind| Token{
                kind: kind.to_string(),
                lexeme: "xy".to_string(),
            },).collect();

        let parser = def_parser();

        //execute
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
        let productions = build_prods(&[
            "s s b",
            "s ",
            "b LBRACKET s RBRACKET",
            "b w",
            "w WHITESPACE",
        ]);
        let grammar = Grammar::from(productions);

        let scan = "WHITESPACE LBRACKET WHITESPACE LBRACKET WHITESPACE RBRACKET WHITESPACE RBRACKET LBRACKET RBRACKET WHITESPACE".split_whitespace()
            .map(|kind| Token{
                kind: kind.to_string(),
                lexeme: "xy".to_string(),
            },).collect();

        let parser = def_parser();

        //execute
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
        let productions = build_prods(&[
            "s w w w w",
            "w WHITESPACE",
            "w ",
        ]);
        let grammar = Grammar::from(productions);

        let scan = "WHITESPACE".split_whitespace()
            .map(|kind| Token{
                kind: kind.to_string(),
                lexeme: "xy".to_string(),
            },).collect();

        let parser = def_parser();

        //execute
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
}