use {
    core::{
        data::Data,
        lex::Token,
        parse::grammar::{Grammar, GrammarSymbol},
    },
    std::{error, fmt},
};

mod earley;
pub mod grammar;

pub trait Parser<Symbol: GrammarSymbol>: 'static + Send + Sync {
    fn parse(
        &self,
        lex: Vec<Token<Symbol>>,
        grammar: &dyn Grammar<Symbol>,
    ) -> Result<Tree<Symbol>, Error>;
}

pub fn def_parser<Symbol: GrammarSymbol>() -> Box<dyn Parser<Symbol>> {
    Box::new(earley::EarleyParser)
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum SymbolParseMethod {
    Standard,
    Ignored,
    Injected,
    Repeated,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct Tree<Symbol: GrammarSymbol> {
    pub lhs: Token<Symbol>,
    pub children: Vec<Tree<Symbol>>,
    pub production: Option<Production<Symbol>>,
    pub spm: SymbolParseMethod,
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
            production: None,
            spm: SymbolParseMethod::Standard,
        }
    }

    #[allow(dead_code)]
    pub fn decode(&self, grammar: &dyn Grammar<Symbol>) -> Tree<String> {
        let lhs = match self.lhs.kind_opt() {
            Some(ref symbol) => {
                Token::leaf(grammar.symbol_string(symbol), self.lhs.lexeme().clone())
            }
            None => Token::null(),
        };

        let children: Vec<Tree<String>> = self
            .children
            .iter()
            .map(|tree| tree.decode(grammar))
            .collect();

        let production = match self.production {
            Some(ref prod) => Some(prod.decode(grammar)),
            None => None,
        };

        Tree {
            lhs,
            children,
            production,
            spm: self.spm.clone(),
        }
    }

    fn to_string_internal(&self, prefix: String, is_tail: bool) -> String {
        if self.children.is_empty() {
            format!(
                "{}{}{}{}",
                prefix,
                if is_tail { "└── " } else { "├── " },
                if self.spm == SymbolParseMethod::Injected {
                    "<< "
                } else {
                    ""
                },
                self.lhs.to_string()
            )
        } else {
            let kind_string = match self.lhs.kind_opt() {
                None => String::from("?"),
                Some(kind) => kind.to_string(),
            };
            let mut builder = format!(
                "{}{}{}",
                prefix,
                if is_tail { "└── " } else { "├── " },
                kind_string,
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
pub struct ProductionSymbol<Symbol: GrammarSymbol> {
    pub symbol: Symbol,
    pub is_list: bool,
}

impl<Symbol: GrammarSymbol> ProductionSymbol<Symbol> {
    pub fn symbol(symbol: Symbol) -> Self {
        ProductionSymbol {
            symbol,
            is_list: false,
        }
    }

    pub fn symbol_list(symbol: Symbol) -> Self {
        ProductionSymbol {
            symbol,
            is_list: true,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct Production<Symbol: GrammarSymbol> {
    pub lhs: Symbol,
    pub rhs: Vec<ProductionSymbol<Symbol>>,
}

impl<Symbol: GrammarSymbol> Production<Symbol> {
    pub fn from(lhs: Symbol, rhs: Vec<ProductionSymbol<Symbol>>) -> Self {
        Production { lhs, rhs }
    }

    pub fn epsilon(lhs: Symbol) -> Self {
        Production::from(lhs, Vec::new())
    }

    pub fn string_production(&self) -> Production<String> {
        Production {
            lhs: self.lhs.to_string(),
            rhs: self
                .rhs
                .iter()
                .map(|sym| ProductionSymbol {
                    symbol: sym.symbol.to_string(),
                    is_list: sym.is_list,
                })
                .collect(),
        }
    }

    pub fn decode(&self, grammar: &dyn Grammar<Symbol>) -> Production<String> {
        Production {
            lhs: self.lhs.to_string(),
            rhs: self
                .rhs
                .iter()
                .map(|sym| ProductionSymbol {
                    symbol: grammar.symbol_string(&sym.symbol),
                    is_list: sym.is_list,
                })
                .collect(),
        }
    }
}

impl<Symbol: GrammarSymbol> Data for Production<Symbol> {
    fn to_string(&self) -> String {
        let mut res_string = self.lhs.to_string();

        for sym in &self.rhs {
            res_string = format!("{} {}", res_string, sym.symbol.to_string());
        }

        res_string
    }
}

#[cfg(test)]
mod tests {
    use core::{
        fmt::InjectionAffinity,
        parse::grammar::{GrammarBuilder, SimpleGrammarBuilder},
    };

    use super::*;

    #[test]
    fn parse_example() {
        //setup
        let mut grammar_builder = SimpleGrammarBuilder::new();
        add_productions(
            &["Sentence Noun Verb", "Noun mary", "Verb runs"],
            &mut grammar_builder,
        );
        grammar_builder.try_mark_start(&"Sentence".to_string());
        let grammar = grammar_builder.build().unwrap();

        let lex = vec![
            Token::leaf("mary".to_string(), "Hello".to_string()),
            Token::leaf("runs".to_string(), "World!".to_string()),
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(lex, &grammar);

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

        let lex = vec![
            Token::leaf("BOF".to_string(), "a".to_string()),
            Token::leaf("x".to_string(), "b".to_string()),
            Token::leaf("EOF".to_string(), "c".to_string()),
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(lex, &grammar);

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
        add_productions(
            &["S expr", "S S OP expr", "expr ( S )", "expr ID"],
            &mut grammar_builder,
        );
        grammar_builder.try_mark_start(&"S".to_string());
        let grammar = grammar_builder.build().unwrap();

        let lex = "( ID OP ID ) OP ID OP ( ID )"
            .split_whitespace()
            .map(|kind| Token::leaf(kind.to_string(), "xy".to_string()))
            .collect();

        let parser = def_parser();

        //exercise
        let tree = parser.parse(lex, &grammar);

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
        add_productions(
            &[
                "Sum Sum AS Product",
                "Sum Product",
                "Product Product MD Factor",
                "Product Factor",
                "Factor LPAREN Sum RPAREN",
                "Factor Number",
                "Number NUM",
            ],
            &mut grammar_builder,
        );
        grammar_builder.try_mark_start(&"Sum".to_string());
        let grammar = grammar_builder.build().unwrap();

        let lex = "NUM AS LPAREN NUM MD NUM AS NUM RPAREN"
            .split_whitespace()
            .map(|kind| Token::leaf(kind.to_string(), "xy".to_string()))
            .collect();

        let parser = def_parser();

        //exercise
        let tree = parser.parse(lex, &grammar);

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
        add_productions(
            &[
                "s s b",
                "s ",
                "b LBRACKET s RBRACKET",
                "b w",
                "w WHITESPACE",
            ],
            &mut grammar_builder,
        );
        grammar_builder.try_mark_start(&"s".to_string());
        let grammar = grammar_builder.build().unwrap();

        let lex = "WHITESPACE LBRACKET WHITESPACE LBRACKET WHITESPACE RBRACKET WHITESPACE RBRACKET LBRACKET RBRACKET WHITESPACE".split_whitespace()
            .map(|kind| Token::leaf(kind.to_string(), "xy".to_string()))
            .collect();

        let parser = def_parser();

        //exercise
        let tree = parser.parse(lex, &grammar);

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
        add_productions(&["s w w w w", "w WHITESPACE", "w "], &mut grammar_builder);
        grammar_builder.try_mark_start(&"s".to_string());
        let grammar = grammar_builder.build().unwrap();

        let lex = "WHITESPACE"
            .split_whitespace()
            .map(|kind| Token::leaf(kind.to_string(), "xy".to_string()))
            .collect();

        let parser = def_parser();

        //exercise
        let tree = parser.parse(lex, &grammar);

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
        add_productions(
            &[
                "sum sum PM prod",
                "sum prod",
                "prod prod MD fac",
                "prod fac",
                "fac LPAREN sum RPAREN",
                "fac num",
                "num DIGIT num",
                "num DIGIT",
            ],
            &mut grammar_builder,
        );
        grammar_builder.try_mark_start(&"sum".to_string());
        let grammar = grammar_builder.build().unwrap();

        let lex = vec![
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
        let tree = parser.parse(lex, &grammar);

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

        let lex = vec![Token::leaf("kind".to_string(), "lexeme".to_string())];

        let parser = def_parser();

        //exercise
        let res = parser.parse(lex, &grammar);

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

        let lex = vec![
            Token::leaf("A".to_string(), "a".to_string()),
            Token::leaf("C".to_string(), "c".to_string()),
            Token::leaf("C".to_string(), "c".to_string()),
            Token::leaf("B".to_string(), "b".to_string()),
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(lex, &grammar);

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

        let lex = vec![
            Token::leaf("C".to_string(), "c".to_string()),
            Token::leaf("C".to_string(), "c".to_string()),
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(lex, &grammar);

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

        let lex = vec![
            Token::leaf("A".to_string(), "a".to_string()),
            Token::leaf("C".to_string(), "c".to_string()),
            Token::leaf("A".to_string(), "a".to_string()),
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(lex, &grammar);

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

    #[test]
    fn injectable_terminal_left_affinity() {
        //setup
        let mut grammar_builder = SimpleGrammarBuilder::new();
        add_productions(&["s A s B", "s "], &mut grammar_builder);
        grammar_builder.try_mark_start(&"s".to_string());
        grammar_builder.mark_injectable(&"C".to_string(), InjectionAffinity::Left);
        let grammar = grammar_builder.build().unwrap();

        let lex = vec![
            Token::leaf("A".to_string(), "a".to_string()),
            Token::leaf("C".to_string(), "c".to_string()),
            Token::leaf("B".to_string(), "b".to_string()),
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(lex, &grammar);

        //verify
        assert_eq!(
            tree.unwrap().to_string(),
            "└── s
    ├── A <- 'a'
    ├── << C <- 'c'
    ├── s
    │   └──  <- 'NULL'
    └── B <- 'b'"
        );
    }

    #[test]
    fn injectable_terminal_right_affinity() {
        //setup
        let mut grammar_builder = SimpleGrammarBuilder::new();
        add_productions(&["s A s B", "s "], &mut grammar_builder);
        grammar_builder.try_mark_start(&"s".to_string());
        grammar_builder.mark_injectable(&"C".to_string(), InjectionAffinity::Right);
        let grammar = grammar_builder.build().unwrap();

        let lex = vec![
            Token::leaf("A".to_string(), "a".to_string()),
            Token::leaf("C".to_string(), "c".to_string()),
            Token::leaf("B".to_string(), "b".to_string()),
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(lex, &grammar);

        //verify
        assert_eq!(
            tree.unwrap().to_string(),
            "└── s
    ├── A <- 'a'
    ├── s
    │   └──  <- 'NULL'
    ├── << C <- 'c'
    └── B <- 'b'"
        );
    }

    #[test]
    fn injectable_terminal_last() {
        //setup
        let mut grammar_builder = SimpleGrammarBuilder::new();
        add_productions(&["s A s B", "s "], &mut grammar_builder);
        grammar_builder.try_mark_start(&"s".to_string());
        grammar_builder.mark_injectable(&"C".to_string(), InjectionAffinity::Left);
        let grammar = grammar_builder.build().unwrap();

        let lex = vec![
            Token::leaf("A".to_string(), "a".to_string()),
            Token::leaf("B".to_string(), "b".to_string()),
            Token::leaf("C".to_string(), "c".to_string()),
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(lex, &grammar);

        //verify
        assert_eq!(
            tree.unwrap().to_string(),
            "└── s
    ├── A <- 'a'
    ├── s
    │   └──  <- 'NULL'
    ├── B <- 'b'
    └── << C <- 'c'"
        );
    }

    #[test]
    fn favour_non_injected_terminals() {
        //setup
        let mut grammar_builder = SimpleGrammarBuilder::new();
        add_productions(&["s A s A", "s C", "s "], &mut grammar_builder);
        grammar_builder.try_mark_start(&"s".to_string());
        grammar_builder.mark_injectable(&"C".to_string(), InjectionAffinity::Left);
        let grammar = grammar_builder.build().unwrap();

        let lex = vec![
            Token::leaf("A".to_string(), "a".to_string()),
            Token::leaf("C".to_string(), "c".to_string()),
            Token::leaf("A".to_string(), "a".to_string()),
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(lex, &grammar);

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

    #[test]
    fn injection_into_single_terminal() {
        //setup
        let mut grammar_builder = SimpleGrammarBuilder::new();
        add_productions(&["s A"], &mut grammar_builder);
        grammar_builder.try_mark_start(&"s".to_string());
        grammar_builder.mark_injectable(&"B".to_string(), InjectionAffinity::Right);
        let grammar = grammar_builder.build().unwrap();

        let lex = vec![
            Token::leaf("B".to_string(), "b".to_string()),
            Token::leaf("A".to_string(), "a".to_string()),
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(lex, &grammar);

        //verify
        assert_eq!(
            tree.unwrap().to_string(),
            "└── s
    ├── << B <- 'b'
    └── A <- 'a'"
        );
    }

    #[test]
    fn inline_list() {
        //setup
        let mut grammar_builder = SimpleGrammarBuilder::new();

        grammar_builder.add_production(Production::from(
            "s".to_string(),
            vec![ProductionSymbol::symbol_list("T".to_string())],
        ));

        grammar_builder.try_mark_start(&"s".to_string());

        let grammar = grammar_builder.build().unwrap();

        let lex = vec![
            Token::leaf("T".to_string(), "a".to_string()),
            Token::leaf("T".to_string(), "b".to_string()),
            Token::leaf("T".to_string(), "c".to_string()),
            Token::leaf("T".to_string(), "d".to_string()),
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(lex, &grammar).unwrap();

        //verify
        assert_eq!(
            tree.to_string(),
            "└── s
    └── ?
        ├── T <- 'a'
        ├── T <- 'b'
        ├── T <- 'c'
        └── T <- 'd'"
        );
    }

    #[test]
    fn pinned_inline_list() {
        //setup
        let mut grammar_builder = SimpleGrammarBuilder::new();

        grammar_builder.add_production(Production::from(
            "s".to_string(),
            vec![
                ProductionSymbol::symbol("A".to_string()),
                ProductionSymbol::symbol_list("T".to_string()),
                ProductionSymbol::symbol("A".to_string()),
            ],
        ));

        grammar_builder.try_mark_start(&"s".to_string());

        let grammar = grammar_builder.build().unwrap();

        let lex = vec![
            Token::leaf("A".to_string(), "1".to_string()),
            Token::leaf("T".to_string(), "a".to_string()),
            Token::leaf("T".to_string(), "b".to_string()),
            Token::leaf("T".to_string(), "c".to_string()),
            Token::leaf("T".to_string(), "d".to_string()),
            Token::leaf("A".to_string(), "2".to_string()),
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(lex, &grammar).unwrap();

        //verify
        assert_eq!(
            tree.to_string(),
            "└── s
    ├── A <- '1'
    ├── ?
    │   ├── T <- 'a'
    │   ├── T <- 'b'
    │   ├── T <- 'c'
    │   └── T <- 'd'
    └── A <- '2'"
        );
    }

    #[test]
    fn non_terminal_inline_list() {
        //setup
        let mut grammar_builder = SimpleGrammarBuilder::new();

        grammar_builder.add_production(Production::from(
            "s".to_string(),
            vec![ProductionSymbol::symbol_list("t".to_string())],
        ));

        grammar_builder.add_production(Production::from(
            "t".to_string(),
            vec![ProductionSymbol::symbol("T".to_string())],
        ));

        grammar_builder.try_mark_start(&"s".to_string());

        let grammar = grammar_builder.build().unwrap();

        let lex = vec![
            Token::leaf("T".to_string(), "a".to_string()),
            Token::leaf("T".to_string(), "b".to_string()),
            Token::leaf("T".to_string(), "c".to_string()),
            Token::leaf("T".to_string(), "d".to_string()),
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(lex, &grammar).unwrap();

        //verify
        assert_eq!(
            tree.to_string(),
            "└── s
    └── ?
        ├── t
        │   └── T <- 'a'
        ├── t
        │   └── T <- 'b'
        ├── t
        │   └── T <- 'c'
        └── t
            └── T <- 'd'"
        );
    }

    #[test]
    fn pinned_non_terminal_inline_list() {
        //setup
        let mut grammar_builder = SimpleGrammarBuilder::new();

        grammar_builder.add_production(Production::from(
            "s".to_string(),
            vec![
                ProductionSymbol::symbol("A".to_string()),
                ProductionSymbol::symbol_list("t".to_string()),
                ProductionSymbol::symbol("A".to_string()),
            ],
        ));

        grammar_builder.add_production(Production::from(
            "t".to_string(),
            vec![ProductionSymbol::symbol("T".to_string())],
        ));

        grammar_builder.try_mark_start(&"s".to_string());

        let grammar = grammar_builder.build().unwrap();

        let lex = vec![
            Token::leaf("A".to_string(), "1".to_string()),
            Token::leaf("T".to_string(), "a".to_string()),
            Token::leaf("T".to_string(), "b".to_string()),
            Token::leaf("T".to_string(), "c".to_string()),
            Token::leaf("T".to_string(), "d".to_string()),
            Token::leaf("A".to_string(), "2".to_string()),
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(lex, &grammar).unwrap();

        //verify
        assert_eq!(
            tree.to_string(),
            "└── s
    ├── A <- '1'
    ├── ?
    │   ├── t
    │   │   └── T <- 'a'
    │   ├── t
    │   │   └── T <- 'b'
    │   ├── t
    │   │   └── T <- 'c'
    │   └── t
    │       └── T <- 'd'
    └── A <- '2'"
        );
    }

    #[test]
    fn push_down_nested_inline_lists() {
        //setup
        let mut grammar_builder = SimpleGrammarBuilder::new();

        grammar_builder.add_production(Production::from(
            "s".to_string(),
            vec![ProductionSymbol::symbol_list("t".to_string())],
        ));

        grammar_builder.add_production(Production::from(
            "t".to_string(),
            vec![ProductionSymbol::symbol_list("T".to_string())],
        ));

        grammar_builder.try_mark_start(&"s".to_string());

        let grammar = grammar_builder.build().unwrap();

        let lex = vec![
            Token::leaf("T".to_string(), "a".to_string()),
            Token::leaf("T".to_string(), "b".to_string()),
            Token::leaf("T".to_string(), "c".to_string()),
            Token::leaf("T".to_string(), "d".to_string()),
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(lex, &grammar).unwrap();

        //verify
        assert_eq!(
            tree.to_string(),
            "└── s
    └── t
        └── ?
            ├── T <- 'a'
            ├── T <- 'b'
            ├── T <- 'c'
            └── T <- 'd'"
        );
    }

    #[test]
    fn inline_lists_non_empty() {
        //setup
        let mut grammar_builder = SimpleGrammarBuilder::new();

        grammar_builder.add_production(Production::from(
            "s".to_string(),
            vec![
                ProductionSymbol::symbol_list("X".to_string()),
                ProductionSymbol::symbol_list("Y".to_string()),
                ProductionSymbol::symbol_list("Z".to_string()),
            ],
        ));

        grammar_builder.try_mark_start(&"s".to_string());

        let grammar = grammar_builder.build().unwrap();

        let parser = def_parser();

        //exercise/verify
        let res = parser.parse(
            vec![
                Token::leaf("Y".to_string(), "2".to_string()),
                Token::leaf("Z".to_string(), "3".to_string()),
            ],
            &grammar,
        );

        assert!(res.is_err());
        assert_eq!(
            format!("{}", res.err().unwrap()),
            "Recognition failed at token 1: Y <- '2'"
        );

        let res = parser.parse(
            vec![
                Token::leaf("X".to_string(), "1".to_string()),
                Token::leaf("Z".to_string(), "3".to_string()),
            ],
            &grammar,
        );

        assert!(res.is_err());
        assert_eq!(
            format!("{}", res.err().unwrap()),
            "Recognition failed at token 2: Z <- '3'"
        );

        let res = parser.parse(
            vec![
                Token::leaf("X".to_string(), "1".to_string()),
                Token::leaf("Y".to_string(), "2".to_string()),
            ],
            &grammar,
        );

        assert!(res.is_err());
        assert_eq!(
            format!("{}", res.err().unwrap()),
            "Recognition failed after consuming all tokens"
        );
    }

    #[test]
    fn repeated_inline_lists() {
        //setup
        let mut grammar_builder = SimpleGrammarBuilder::new();

        grammar_builder.add_production(Production::from(
            "s".to_string(),
            vec![
                ProductionSymbol::symbol_list("X".to_string()),
                ProductionSymbol::symbol_list("Y".to_string()),
                ProductionSymbol::symbol_list("Z".to_string()),
            ],
        ));

        grammar_builder.try_mark_start(&"s".to_string());

        let grammar = grammar_builder.build().unwrap();

        let lex = vec![
            Token::leaf("X".to_string(), "z".to_string()),
            Token::leaf("Y".to_string(), "a".to_string()),
            Token::leaf("Y".to_string(), "b".to_string()),
            Token::leaf("Z".to_string(), "1".to_string()),
            Token::leaf("Z".to_string(), "2".to_string()),
            Token::leaf("Z".to_string(), "3".to_string()),
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(lex, &grammar).unwrap();

        //verify
        assert_eq!(
            tree.to_string(),
            "└── s
    ├── X <- 'z'
    ├── ?
    │   ├── Y <- 'a'
    │   └── Y <- 'b'
    └── ?
        ├── Z <- '1'
        ├── Z <- '2'
        └── Z <- '3'"
        );
    }

    #[test]
    fn inline_list_before_optional_list() {
        //setup
        let mut grammar_builder = SimpleGrammarBuilder::new();

        grammar_builder.add_production(Production::from(
            "s".to_string(),
            vec![
                ProductionSymbol::symbol_list("X".to_string()),
                ProductionSymbol::symbol("t_opt".to_string()),
            ],
        ));

        grammar_builder.add_production(Production::from(
            "t_opt".to_string(),
            vec![ProductionSymbol::symbol_list("T".to_string())],
        ));
        grammar_builder.add_production(Production::from("t_opt".to_string(), Vec::new()));

        grammar_builder.try_mark_start(&"s".to_string());

        let grammar = grammar_builder.build().unwrap();

        let parser = def_parser();

        //exercise/verify
        let lex = vec![
            Token::leaf("X".to_string(), "a".to_string()),
            Token::leaf("X".to_string(), "b".to_string()),
            Token::leaf("T".to_string(), "1".to_string()),
            Token::leaf("T".to_string(), "2".to_string()),
        ];

        let tree = parser.parse(lex, &grammar).unwrap();

        assert_eq!(
            tree.to_string(),
            "└── s
    ├── ?
    │   ├── X <- 'a'
    │   └── X <- 'b'
    └── t_opt
        └── ?
            ├── T <- '1'
            └── T <- '2'"
        );

        let lex = vec![
            Token::leaf("X".to_string(), "a".to_string()),
            Token::leaf("X".to_string(), "b".to_string()),
        ];

        let tree = parser.parse(lex, &grammar).unwrap();

        assert_eq!(
            tree.to_string(),
            "└── s
    ├── ?
    │   ├── X <- 'a'
    │   └── X <- 'b'
    └── t_opt
        └──  <- 'NULL'"
        );
    }

    #[test]
    fn inline_list_after_optional_list() {
        //setup
        let mut grammar_builder = SimpleGrammarBuilder::new();

        grammar_builder.add_production(Production::from(
            "s".to_string(),
            vec![
                ProductionSymbol::symbol("t_opt".to_string()),
                ProductionSymbol::symbol_list("X".to_string()),
            ],
        ));

        grammar_builder.add_production(Production::from(
            "t_opt".to_string(),
            vec![ProductionSymbol::symbol_list("T".to_string())],
        ));
        grammar_builder.add_production(Production::from("t_opt".to_string(), Vec::new()));

        grammar_builder.try_mark_start(&"s".to_string());

        let grammar = grammar_builder.build().unwrap();

        let parser = def_parser();

        //exercise/verify
        let lex = vec![
            Token::leaf("T".to_string(), "1".to_string()),
            Token::leaf("T".to_string(), "2".to_string()),
            Token::leaf("X".to_string(), "a".to_string()),
            Token::leaf("X".to_string(), "b".to_string()),
        ];

        let tree = parser.parse(lex, &grammar).unwrap();

        assert_eq!(
            tree.to_string(),
            "└── s
    ├── t_opt
    │   └── ?
    │       ├── T <- '1'
    │       └── T <- '2'
    └── ?
        ├── X <- 'a'
        └── X <- 'b'"
        );

        let lex = vec![
            Token::leaf("X".to_string(), "a".to_string()),
            Token::leaf("X".to_string(), "b".to_string()),
        ];

        let tree = parser.parse(lex, &grammar).unwrap();

        assert_eq!(
            tree.to_string(),
            "└── s
    ├── t_opt
    │   └──  <- 'NULL'
    └── ?
        ├── X <- 'a'
        └── X <- 'b'"
        );
    }

    #[test]
    fn inline_list_of_injectable_terminals() {
        //setup
        let mut grammar_builder = SimpleGrammarBuilder::new();

        grammar_builder.add_production(Production::from(
            "s".to_string(),
            vec![
                ProductionSymbol::symbol("A".to_string()),
                ProductionSymbol::symbol_list("T".to_string()),
                ProductionSymbol::symbol("A".to_string()),
            ],
        ));

        grammar_builder.mark_injectable(&"T".to_string(), InjectionAffinity::Right);

        grammar_builder.try_mark_start(&"s".to_string());

        let grammar = grammar_builder.build().unwrap();

        let lex = vec![
            Token::leaf("T".to_string(), "a".to_string()),
            Token::leaf("A".to_string(), "1".to_string()),
            Token::leaf("T".to_string(), "b".to_string()),
            Token::leaf("T".to_string(), "c".to_string()),
            Token::leaf("A".to_string(), "2".to_string()),
            Token::leaf("T".to_string(), "d".to_string()),
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(lex, &grammar).unwrap();

        //verify
        assert_eq!(
            tree.to_string(),
            "└── s
    ├── << T <- 'a'
    ├── A <- '1'
    ├── ?
    │   ├── T <- 'b'
    │   └── T <- 'c'
    ├── A <- '2'
    └── << T <- 'd'"
        );
    }

    #[test]
    fn inline_list_between_injectable_terminals() {
        //setup
        let mut grammar_builder = SimpleGrammarBuilder::new();

        grammar_builder.add_production(Production::from(
            "s".to_string(),
            vec![ProductionSymbol::symbol_list("T".to_string())],
        ));

        grammar_builder.mark_injectable(&"A".to_string(), InjectionAffinity::Right);

        grammar_builder.try_mark_start(&"s".to_string());

        let grammar = grammar_builder.build().unwrap();

        let lex = vec![
            Token::leaf("A".to_string(), "1".to_string()),
            Token::leaf("T".to_string(), "a".to_string()),
            Token::leaf("T".to_string(), "b".to_string()),
            Token::leaf("A".to_string(), "2".to_string()),
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(lex, &grammar).unwrap();

        //verify
        assert_eq!(
            tree.to_string(),
            "└── s
    ├── << A <- '1'
    ├── ?
    │   ├── T <- 'a'
    │   └── T <- 'b'
    └── << A <- '2'"
        );
    }

    #[test]
    fn injections_intersecting_inline_list() {
        //setup
        let mut grammar_builder = SimpleGrammarBuilder::new();

        grammar_builder.add_production(Production::from(
            "s".to_string(),
            vec![ProductionSymbol::symbol_list("T".to_string())],
        ));

        grammar_builder.mark_injectable(&"A".to_string(), InjectionAffinity::Right);

        grammar_builder.try_mark_start(&"s".to_string());

        let grammar = grammar_builder.build().unwrap();

        let lex = vec![
            Token::leaf("A".to_string(), "1".to_string()),
            Token::leaf("T".to_string(), "a".to_string()),
            Token::leaf("A".to_string(), "2".to_string()),
            Token::leaf("A".to_string(), "3".to_string()),
            Token::leaf("T".to_string(), "b".to_string()),
            Token::leaf("T".to_string(), "c".to_string()),
            Token::leaf("T".to_string(), "d".to_string()),
            Token::leaf("A".to_string(), "4".to_string()),
            Token::leaf("A".to_string(), "5".to_string()),
            Token::leaf("T".to_string(), "e".to_string()),
        ];

        let parser = def_parser();

        //exercise
        let tree = parser.parse(lex, &grammar).unwrap();

        //verify
        assert_eq!(
            tree.to_string(),
            "└── s
    ├── << A <- '1'
    └── ?
        ├── T <- 'a'
        ├── A <- '2'
        ├── A <- '3'
        ├── T <- 'b'
        ├── T <- 'c'
        ├── T <- 'd'
        ├── A <- '4'
        ├── A <- '5'
        └── T <- 'e'"
        );
    }

    pub fn add_productions<'scope>(
        strings: &'scope [&'scope str],
        grammar_builder: &mut SimpleGrammarBuilder<String>,
    ) {
        for string in strings {
            grammar_builder.add_production(production_from_string(string));
        }
    }

    fn production_from_string(string: &str) -> Production<String> {
        let mut i = 0;
        let mut lhs = String::new();
        let mut rhs: Vec<ProductionSymbol<String>> = vec![];

        for s in string.split_whitespace() {
            if i == 0 {
                lhs = s.to_string();
            } else {
                rhs.push(ProductionSymbol {
                    symbol: s.to_string(),
                    is_list: false,
                });
            }
            i += 1;
        }

        Production { lhs, rhs }
    }
}
