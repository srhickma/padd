use std::collections::HashSet;
use std::collections::HashMap;
use std::error;
use std::fmt;
use core::scan::Token;

mod earley;

pub trait Parser {
    fn parse(&self, scan: Vec<Token>, grammar: &Grammar) -> Result<Tree, Error>;
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

pub struct Grammar {
    pub productions: Vec<Production>,
    nss: HashSet<String>,
    #[allow(dead_code)]
    non_terminals: HashSet<String>,
    terminals: HashSet<String>,
    #[allow(dead_code)]
    symbols: HashSet<String>,
    start: String,
}

impl Grammar {
    pub fn nullable(&self, prod: &Production) -> bool {
        self.nss.contains(&prod.lhs)
    }

    pub fn from(productions: Vec<Production>) -> Grammar {
        let nss = Grammar::build_nss(&productions);
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
            nss,
            non_terminals,
            terminals,
            symbols,
            start,
        };
    }

    fn build_nss(productions: &Vec<Production>) ->  HashSet<String> {
        let mut nss: HashSet<String> = HashSet::new();
        let mut prods_by_rhs: HashMap<&String, Vec<&Production>> = HashMap::new();
        let mut work_stack: Vec<&String> = Vec::new();

        for prod in productions {
            for s in &prod.rhs {
                prods_by_rhs.entry(s)
                    .or_insert(Vec::new())
                    .push(prod);
            }

            if prod.rhs.is_empty() {
                nss.insert(prod.lhs.clone()); //TODO can we avoid cloning here
                work_stack.push(&prod.lhs);
            }
        }

        loop {
            match work_stack.pop() {
                None => break,
                Some(work_symbol) => {
                    match prods_by_rhs.get(work_symbol) {
                        None => {},
                        Some(prods) => {
                            for prod in prods {
                                if !nss.contains(&prod.lhs)
                                    && prod.rhs.iter().all(|sym| nss.contains(sym)) {
                                    nss.insert(prod.lhs.clone());
                                    work_stack.push(&prod.lhs);
                                }
                            }
                        }
                    }
                }
            };
        }

        nss
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
            "S S OP expr",
            "expr ( S )",
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

    #[test]
    fn advanced_parse_build(){
        //setup
        let productions = build_prods(&[
            "sum sum PM prod",
            "sum prod",
            "prod prod MD fac",
            "prod fac",
            "fac LPAREN sum RPAREN",
            "fac num",
            "num DIGIT num",
            "num DIGIT"
        ]);
        let grammar = Grammar::from(productions);

        let scan = vec![
            Token{ kind: "DIGIT".to_string(), lexeme: "1".to_string() },
            Token{ kind: "PM".to_string(), lexeme: "+".to_string() },
            Token{ kind: "LPAREN".to_string(), lexeme: "(".to_string() },
            Token{ kind: "DIGIT".to_string(), lexeme: "2".to_string() },
            Token{ kind: "MD".to_string(), lexeme: "*".to_string() },
            Token{ kind: "DIGIT".to_string(), lexeme: "3".to_string() },
            Token{ kind: "PM".to_string(), lexeme: "-".to_string() },
            Token{ kind: "DIGIT".to_string(), lexeme: "4".to_string() },
            Token{ kind: "RPAREN".to_string(), lexeme: ")".to_string() },
        ];

        let parser = def_parser();

        //execute
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
        let productions = build_prods(&["s "]);
        let grammar = Grammar::from(productions);

        let scan = vec![Token{
            kind: "kind".to_string(),
            lexeme: "lexeme".to_string(),
        }];

        let parser = def_parser();

        //execute
        let res = parser.parse(scan, &grammar);

        //verify
        assert!(res.is_err());
        assert_eq!(format!("{}", res.err().unwrap()), "Largest parse did not consume all tokens: 0 of 1")
    }
}