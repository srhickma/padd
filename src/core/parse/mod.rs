use {
    core::{data::Data, parse::grammar::Grammar, scan::Token},
    std::{error, fmt},
};

mod earley;
pub mod grammar;

pub trait Parser<Symbol: Data + Default>: 'static + Send + Sync {
    fn parse(
        &self,
        scan: Vec<Token<Symbol>>,
        grammar: &Grammar<Symbol>,
    ) -> Result<Tree<Symbol>, Error>;
}

pub fn def_parser<Symbol: Data + Default>() -> Box<Parser<Symbol>> {
    Box::new(earley::EarleyParser)
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct Tree<Symbol: Data + Default> {
    pub lhs: Token<Symbol>,
    pub children: Vec<Tree<Symbol>>,
}

impl<Symbol: Data + Default> Tree<Symbol> {
    pub fn get_child(&self, i: usize) -> &Tree<Symbol> {
        &self.children[i]
    }

    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    pub fn is_empty(&self) -> bool {
        self.children.len() == 1 && self.get_child(0).is_null()
    }

    pub fn is_null(&self) -> bool {
        self.lhs.is_null()
    }

    pub fn null() -> Tree<Symbol> {
        Tree {
            lhs: Token::null(),
            children: vec![],
        }
    }

    fn to_string_internal(&self, prefix: String, is_tail: bool) -> String {
        if self.children.is_empty() {
            format!(
                "{}{}{}",
                prefix,
                if is_tail { "└── " } else { "├── " },
                self.lhs.to_string()
            )
        } else {
            let mut builder = format!(
                "{}{}{}",
                prefix,
                if is_tail { "└── " } else { "├── " },
                self.lhs.kind().to_string()
            );
            let len = self.children.len();
            for (i, child) in self.children.iter().enumerate() {
                let margin = format!("{}{}", prefix, if is_tail { "    " } else { "│   " });
                let child_string = child.to_string_internal(margin, i == len - 1);
                builder = format!("{}\n{}", builder, child_string);
            }
            builder
        }
    }

    pub fn production(&self) -> Production<Symbol> {
        let mut rhs: Vec<Symbol> = Vec::new();

        for child in &self.children {
            if !child.is_null() {
                rhs.push(child.lhs.kind().clone())
            }
        }

        Production {
            lhs: self.lhs.kind().clone(),
            rhs,
        }
    }
}

impl<Symbol: Data + Default> Data for Tree<Symbol> {
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
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct Production<Symbol: Data + Default> {
    pub lhs: Symbol,
    pub rhs: Vec<Symbol>,
}

impl<Symbol: Data + Default> Production<Symbol> {
    pub fn from(lhs: Symbol, rhs: Vec<Symbol>) -> Self {
        Production { lhs, rhs }
    }

    pub fn epsilon(lhs: Symbol) -> Self {
        Production::from(lhs, Vec::new())
    }
}

impl<Symbol: Data + Default> Data for Production<Symbol> {
    fn to_string(&self) -> String {
        let mut res_string = self.lhs.to_string();

        for symbol in &self.rhs {
            res_string = format!("{} {}", res_string, symbol.to_string());
        }

        res_string
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
        grammar_builder.add_productions(build_prods_from_strings(&[
            "Sentence Noun Verb",
            "Noun mary",
            "Verb runs",
        ]));
        grammar_builder.try_mark_start(&"Sentence".to_string());
        let grammar = grammar_builder.build();

        let scan = vec![
            Token::leaf("mary".to_string(), "Hello".to_string()),
            Token::leaf("runs".to_string(), "World!".to_string()),
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(scan, &grammar);

        //verify
        assert_eq!(
            tree.unwrap().to_string(),
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
        grammar_builder.add_productions(build_prods_from_strings(&["S BOF A EOF", "A x", "A A x"]));
        grammar_builder.try_mark_start(&"S".to_string());
        let grammar = grammar_builder.build();

        let scan = vec![
            Token::leaf("BOF".to_string(), "a".to_string()),
            Token::leaf("x".to_string(), "b".to_string()),
            Token::leaf("EOF".to_string(), "c".to_string()),
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(scan, &grammar);

        //verify
        assert_eq!(
            tree.unwrap().to_string(),
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
        grammar_builder.add_productions(build_prods_from_strings(&[
            "S expr",
            "S S OP expr",
            "expr ( S )",
            "expr ID",
        ]));
        grammar_builder.try_mark_start(&"S".to_string());
        let grammar = grammar_builder.build();

        let scan = "( ID OP ID ) OP ID OP ( ID )"
            .split_whitespace()
            .map(|kind| Token::leaf(kind.to_string(), "xy".to_string()))
            .collect();

        let parser = def_parser();

        //exercise
        let tree = parser.parse(scan, &grammar);

        //verify
        assert_eq!(
            tree.unwrap().to_string(),
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
        grammar_builder.add_productions(build_prods_from_strings(&[
            "Sum Sum AS Product",
            "Sum Product",
            "Product Product MD Factor",
            "Product Factor",
            "Factor LPAREN Sum RPAREN",
            "Factor Number",
            "Number NUM",
        ]));
        grammar_builder.try_mark_start(&"Sum".to_string());
        let grammar = grammar_builder.build();

        let scan = "NUM AS LPAREN NUM MD NUM AS NUM RPAREN"
            .split_whitespace()
            .map(|kind| Token::leaf(kind.to_string(), "xy".to_string()))
            .collect();

        let parser = def_parser();

        //exercise
        let tree = parser.parse(scan, &grammar);

        //verify
        assert_eq!(
            tree.unwrap().to_string(),
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
        grammar_builder.add_productions(build_prods_from_strings(&[
            "s s b",
            "s ",
            "b LBRACKET s RBRACKET",
            "b w",
            "w WHITESPACE",
        ]));
        grammar_builder.try_mark_start(&"s".to_string());
        let grammar = grammar_builder.build();

        let scan = "WHITESPACE LBRACKET WHITESPACE LBRACKET WHITESPACE RBRACKET WHITESPACE RBRACKET LBRACKET RBRACKET WHITESPACE".split_whitespace()
            .map(|kind| Token::leaf(kind.to_string(), "xy".to_string()))
            .collect();

        let parser = def_parser();

        //exercise
        let tree = parser.parse(scan, &grammar);

        //verify
        assert_eq!(
            tree.unwrap().to_string(),
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
        grammar_builder.add_productions(build_prods_from_strings(&[
            "s w w w w",
            "w WHITESPACE",
            "w ",
        ]));
        grammar_builder.try_mark_start(&"s".to_string());
        let grammar = grammar_builder.build();

        let scan = "WHITESPACE"
            .split_whitespace()
            .map(|kind| Token::leaf(kind.to_string(), "xy".to_string()))
            .collect();

        let parser = def_parser();

        //exercise
        let tree = parser.parse(scan, &grammar);

        //verify
        assert_eq!(
            tree.unwrap().to_string(),
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
        grammar_builder.add_productions(build_prods_from_strings(&[
            "sum sum PM prod",
            "sum prod",
            "prod prod MD fac",
            "prod fac",
            "fac LPAREN sum RPAREN",
            "fac num",
            "num DIGIT num",
            "num DIGIT",
        ]));
        grammar_builder.try_mark_start(&"sum".to_string());
        let grammar = grammar_builder.build();

        let scan = vec![
            Token::leaf("DIGIT".to_string(), "1".to_string()),
            Token::leaf("PM".to_string(), "+".to_string()),
            Token::leaf("LPAREN".to_string(), "(".to_string()),
            Token::leaf("DIGIT".to_string(), "2".to_string()),
            Token::leaf("MD".to_string(), "*".to_string()),
            Token::leaf("DIGIT".to_string(), "3".to_string()),
            Token::leaf("PM".to_string(), "-".to_string()),
            Token::leaf("DIGIT".to_string(), "4".to_string()),
            Token::leaf("RPAREN".to_string(), ")".to_string()),
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(scan, &grammar);

        //verify
        assert_eq!(
            tree.unwrap().to_string(),
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
        grammar_builder.add_productions(build_prods_from_strings(&["s "]));
        grammar_builder.try_mark_start(&"s".to_string());
        let grammar = grammar_builder.build();

        let scan = vec![Token::leaf("kind".to_string(), "lexeme".to_string())];

        let parser = def_parser();

        //exercise
        let res = parser.parse(scan, &grammar);

        //verify
        assert!(res.is_err());
        assert_eq!(
            format!("{}", res.err().unwrap()),
            "Largest parse did not consume all tokens: 0 of 1"
        )
    }

    pub fn build_prods_from_strings<'scope>(
        strings: &'scope [&'scope str],
    ) -> Vec<Production<String>> {
        let mut productions: Vec<Production<String>> = vec![];
        for string in strings {
            productions.push(prod_from_string(string));
        }
        productions
    }

    fn prod_from_string(string: &str) -> Production<String> {
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

        Production { lhs, rhs }
    }
}
