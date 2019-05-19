use {
    core::{
        data::Data,
        parse::grammar::{Grammar, GrammarSymbol},
        scan::Token
    },
    std::{error, fmt},
};

mod earley;
pub mod grammar;

pub trait Parser<Symbol: GrammarSymbol>: 'static + Send + Sync {
    fn parse(
        &self,
        scan: Vec<Token<Symbol>>,
        grammar: &Grammar<Symbol>,
    ) -> Result<Tree<Symbol>, Error>;
}

pub fn def_parser<Symbol: GrammarSymbol>() -> Box<Parser<Symbol>> {
    Box::new(earley::EarleyParser)
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct Tree<Symbol: GrammarSymbol> {
    pub lhs: Token<Symbol>,
    pub children: Vec<Tree<Symbol>>,
}

impl<Symbol: GrammarSymbol> Tree<Symbol> {
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
                self.lhs.kind().to_string(),
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

impl<Symbol: GrammarSymbol> Data for Tree<Symbol> {
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
pub struct Production<Symbol: GrammarSymbol> {
    pub lhs: Symbol,
    pub rhs: Vec<Symbol>,
}

impl<Symbol: GrammarSymbol> Production<Symbol> {
    pub fn from(lhs: Symbol, rhs: Vec<Symbol>) -> Self {
        Production { lhs, rhs }
    }

    pub fn epsilon(lhs: Symbol) -> Self {
        Production::from(lhs, Vec::new())
    }
}

impl<Symbol: GrammarSymbol> Data for Production<Symbol> {
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
    use core::parse::grammar::{GrammarBuilder, SimpleGrammarBuilder};

    use super::*;

    #[test]
    fn parse_example() {
        //setup
        let mut grammar_builder = SimpleGrammarBuilder::new();
        add_productions(&[
            "Sentence Noun Verb",
            "Noun mary",
            "Verb runs",
        ], &mut grammar_builder);
        grammar_builder.try_mark_start(&"Sentence".to_string());
        let grammar = grammar_builder.build().unwrap();

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
        let mut grammar_builder = SimpleGrammarBuilder::new();
        add_productions(&["S BOF A EOF", "A x", "A A x"], &mut grammar_builder);
        grammar_builder.try_mark_start(&"S".to_string());
        let grammar = grammar_builder.build().unwrap();

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
        let mut grammar_builder = SimpleGrammarBuilder::new();
        add_productions(&[
            "S expr",
            "S S OP expr",
            "expr ( S )",
            "expr ID",
        ], &mut grammar_builder);
        grammar_builder.try_mark_start(&"S".to_string());
        let grammar = grammar_builder.build().unwrap();

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
        let mut grammar_builder = SimpleGrammarBuilder::new();
        add_productions(&[
            "Sum Sum AS Product",
            "Sum Product",
            "Product Product MD Factor",
            "Product Factor",
            "Factor LPAREN Sum RPAREN",
            "Factor Number",
            "Number NUM",
        ], &mut grammar_builder);
        grammar_builder.try_mark_start(&"Sum".to_string());
        let grammar = grammar_builder.build().unwrap();

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
        let mut grammar_builder = SimpleGrammarBuilder::new();
        add_productions(&[
            "s s b",
            "s ",
            "b LBRACKET s RBRACKET",
            "b w",
            "w WHITESPACE",
        ], &mut grammar_builder);
        grammar_builder.try_mark_start(&"s".to_string());
        let grammar = grammar_builder.build().unwrap();

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
        let mut grammar_builder = SimpleGrammarBuilder::new();
        add_productions(&[
            "s w w w w",
            "w WHITESPACE",
            "w ",
        ], &mut grammar_builder);
        grammar_builder.try_mark_start(&"s".to_string());
        let grammar = grammar_builder.build().unwrap();

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
        let mut grammar_builder = SimpleGrammarBuilder::new();
        add_productions(&[
            "sum sum PM prod",
            "sum prod",
            "prod prod MD fac",
            "prod fac",
            "fac LPAREN sum RPAREN",
            "fac num",
            "num DIGIT num",
            "num DIGIT",
        ], &mut grammar_builder);
        grammar_builder.try_mark_start(&"sum".to_string());
        let grammar = grammar_builder.build().unwrap();

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
        let mut grammar_builder = SimpleGrammarBuilder::new();
        add_productions(&["s "], &mut grammar_builder);
        grammar_builder.try_mark_start(&"s".to_string());
        let grammar = grammar_builder.build().unwrap();

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

    #[test]
    fn ignorable_terminal() {
        //setup
        let mut grammar_builder = SimpleGrammarBuilder::new();
        add_productions(&["s A s B", "s C"], &mut grammar_builder);
        grammar_builder.try_mark_start(&"s".to_string());
        grammar_builder.mark_ignorable(&"C".to_string());
        let grammar = grammar_builder.build().unwrap();

        let scan = vec![
            Token::leaf("A".to_string(), "a".to_string()),
            Token::leaf("C".to_string(), "c".to_string()),
            Token::leaf("C".to_string(), "c".to_string()),
            Token::leaf("B".to_string(), "b".to_string()),
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(scan, &grammar);

        //verify
        assert_eq!(
            tree.unwrap().to_string(),
            "└── s
    ├── A <- 'a'
    ├── s
    │   └── C <- 'c'
    └── B <- 'b'"
        );
    }

    #[test]
    fn ignorable_terminal_only() {
        //setup
        let mut grammar_builder = SimpleGrammarBuilder::new();
        add_productions(&["s "], &mut grammar_builder);
        grammar_builder.try_mark_start(&"s".to_string());
        grammar_builder.mark_ignorable(&"C".to_string());
        let grammar = grammar_builder.build().unwrap();

        let scan = vec![
            Token::leaf("C".to_string(), "c".to_string()),
            Token::leaf("C".to_string(), "c".to_string()),
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(scan, &grammar);

        //verify
        assert_eq!(
            tree.unwrap().to_string(),
            "└── s\n    └──  <- 'NULL'"
        );
    }

    #[test]
    fn favour_non_ignored_terminals() {
        //setup
        let mut grammar_builder = SimpleGrammarBuilder::new();
        add_productions(&["s A s A", "s C", "s "], &mut grammar_builder);
        grammar_builder.try_mark_start(&"s".to_string());
        grammar_builder.mark_ignorable(&"C".to_string());
        let grammar = grammar_builder.build().unwrap();

        let scan = vec![
            Token::leaf("A".to_string(), "a".to_string()),
            Token::leaf("C".to_string(), "c".to_string()),
            Token::leaf("A".to_string(), "a".to_string()),
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(scan, &grammar);

        //verify
        assert_eq!(
            tree.unwrap().to_string(),
            "└── s
    ├── A <- 'a'
    ├── s
    │   └── C <- 'c'
    └── A <- 'a'"
        );
    }

    pub fn add_productions<'scope>(
        strings: &'scope [&'scope str],
        grammar_builder: &mut SimpleGrammarBuilder<String>
    ) {
        for string in strings {
            grammar_builder.add_production(production_from_string(string));
        }
    }

    fn production_from_string(string: &str) -> Production<String> {
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
