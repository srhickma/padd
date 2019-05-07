use {
    core::{
        data::Data,
        parse::{
            self,
            grammar::{Grammar, GrammarBuilder},
            Production, Tree,
        },
        scan::{
            self,
            ecdfa::{EncodedCDFA, EncodedCDFABuilder},
            CDFABuilder,
        },
        util::string_utils,
    },
    std::{error, fmt},
};

static PATTERN_ALPHABET: &'static str =
    "{}[];=0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ \n\t\r`~!@#$%^&*()_-+:'\"<>,.?/\\|";

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
enum S {
    Start,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Semi,
    Equals,
    Zero,
    Number,
    Alpha,
    Filler,
    Escape,
    Fail,
}

thread_local! {
    static PATTERN_ECDFA: EncodedCDFA<Symbol> = build_pattern_ecdfa().unwrap();
}

fn build_pattern_ecdfa() -> Result<EncodedCDFA<Symbol>, scan::CDFAError> {
    let mut builder: EncodedCDFABuilder<S, Symbol> = EncodedCDFABuilder::new();

    builder
        .set_alphabet(PATTERN_ALPHABET.chars())
        .mark_start(&S::Start);

    builder
        .state(&S::Start)
        .mark_trans(&S::LeftBrace, '{')?
        .mark_trans(&S::RightBrace, '}')?
        .mark_trans(&S::LeftBracket, '[')?
        .mark_trans(&S::RightBracket, ']')?
        .mark_trans(&S::Semi, ';')?
        .mark_trans(&S::Equals, '=')?
        .mark_trans(&S::Escape, '\\')?
        .mark_trans(&S::Zero, '0')?
        .mark_range(&S::Number, '1', '9')?
        .mark_range(&S::Alpha, 'a', 'Z')?
        .default_to(&S::Filler)?;

    builder
        .state(&S::Filler)
        .mark_trans(&S::Escape, '\\')?
        .mark_trans(&S::Fail, '{')?
        .mark_trans(&S::Fail, '}')?
        .mark_trans(&S::Fail, '[')?
        .mark_trans(&S::Fail, ';')?
        .mark_trans(&S::Fail, '=')?
        .default_to(&S::Filler)?
        .accept()
        .tokenize(&Symbol::TFiller);

    builder.default_to(&S::Escape, &S::Filler)?;

    builder
        .state(&S::Number)
        .mark_range(&S::Number, '0', '9')?
        .accept()
        .tokenize(&Symbol::TNumber);

    builder
        .state(&S::Alpha)
        .mark_range(&S::Alpha, 'a', 'Z')?
        .accept()
        .tokenize(&Symbol::TAlpha);

    builder
        .accept(&S::Semi)
        .accept(&S::Equals)
        .accept(&S::LeftBrace)
        .accept(&S::RightBrace)
        .accept(&S::LeftBracket)
        .accept(&S::RightBracket)
        .accept(&S::Zero);

    builder
        .tokenize(&S::Semi, &Symbol::TSemi)
        .tokenize(&S::Equals, &Symbol::TEquals)
        .tokenize(&S::LeftBrace, &Symbol::TLeftBrace)
        .tokenize(&S::RightBrace, &Symbol::TRightBrace)
        .tokenize(&S::LeftBracket, &Symbol::TLeftBracket)
        .tokenize(&S::RightBracket, &Symbol::TRightBracket)
        .tokenize(&S::Zero, &Symbol::TNumber);

    builder.build()
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum Symbol {
    Pattern,
    Segments,
    Segment,
    Filler,
    Substitution,
    Capture,
    CaptureDescriptor,
    CaptureIndex,
    Declarations,
    Declaration,
    Value,
    TFiller,
    TAlpha,
    TNumber,
    TLeftBracket,
    TRightBracket,
    TLeftBrace,
    TRightBrace,
    TSemi,
    TEquals,
}

impl Default for Symbol {
    fn default() -> Symbol {
        Symbol::Pattern
    }
}

impl Data for Symbol {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

lazy_static! {
    static ref PATTERN_GRAMMAR: Grammar<Symbol> = build_pattern_grammar();
}

fn build_pattern_grammar() -> Grammar<Symbol> {
    //TODO optimize for left recursion

    let mut builder = GrammarBuilder::new();
    builder.try_mark_start(&Symbol::Pattern);

    builder.from(Symbol::Pattern).to(vec![Symbol::Segments]);

    builder
        .from(Symbol::Segments)
        .to(vec![Symbol::Segment, Symbol::Segments])
        .epsilon();

    builder
        .from(Symbol::Segment)
        .to(vec![Symbol::Filler])
        .to(vec![Symbol::Substitution])
        .to(vec![Symbol::Capture]);

    builder
        .from(Symbol::Filler)
        .to(vec![Symbol::TFiller])
        .to(vec![Symbol::TAlpha])
        .to(vec![Symbol::TNumber]);

    builder.from(Symbol::Substitution).to(vec![
        Symbol::TLeftBracket,
        Symbol::TAlpha,
        Symbol::TRightBracket,
    ]);

    builder.from(Symbol::Capture).to(vec![
        Symbol::TLeftBrace,
        Symbol::CaptureDescriptor,
        Symbol::TRightBrace,
    ]);

    builder
        .from(Symbol::CaptureDescriptor)
        .to(vec![Symbol::CaptureIndex])
        .to(vec![
            Symbol::CaptureIndex,
            Symbol::TSemi,
            Symbol::Declarations,
        ]);

    builder
        .from(Symbol::CaptureIndex)
        .to(vec![Symbol::TNumber])
        .epsilon();

    builder
        .from(Symbol::Declarations)
        .to(vec![
            Symbol::Declarations,
            Symbol::TSemi,
            Symbol::Declaration,
        ])
        .to(vec![Symbol::Declaration]);

    builder
        .from(Symbol::Declaration)
        .to(vec![Symbol::TAlpha, Symbol::TEquals, Symbol::Value]);

    builder
        .from(Symbol::Value)
        .to(vec![Symbol::Pattern])
        .epsilon();

    builder.build()
}

#[derive(Clone)]
pub struct Pattern {
    pub segments: Vec<Segment>,
}

#[derive(Clone)]
pub enum Segment {
    Filler(String),
    Substitution(String),
    Capture(Capture),
}

#[derive(Clone)]
pub struct Capture {
    pub child_index: usize,
    pub declarations: Vec<Declaration>,
}

#[derive(Clone)]
pub struct Declaration {
    pub key: String,
    pub value: Option<Pattern>,
}

pub fn generate_pattern<SpecSymbol: Data + Default>(
    input: &str,
    prod: &Production<SpecSymbol>,
) -> Result<Pattern, BuildError> {
    let parse = parse_pattern(input)?;
    generate_pattern_internal(&parse, prod)
}

pub fn generate_pattern_internal<SpecSymbol: Data + Default>(
    root: &Tree<Symbol>,
    prod: &Production<SpecSymbol>,
) -> Result<Pattern, BuildError> {
    let mut segments: Vec<Segment> = vec![];
    generate_pattern_recursive(&root, &mut segments, prod, 0)?;
    Ok(Pattern { segments })
}

fn generate_pattern_recursive<'scope, SpecSymbol: Data + Default>(
    node: &'scope Tree<Symbol>,
    accumulator: &'scope mut Vec<Segment>,
    prod: &Production<SpecSymbol>,
    captures: usize,
) -> Result<usize, BuildError> {
    if node.lhs.is_null() {
        return Ok(captures);
    }

    match node.lhs.kind() {
        Symbol::TFiller | Symbol::TAlpha | Symbol::TNumber => {
            let name = string_utils::replace_escapes(&node.lhs.lexeme()[..]);
            accumulator.push(Segment::Filler(name));
        }
        Symbol::Substitution => {
            accumulator.push(Segment::Substitution(
                node.get_child(1).lhs.lexeme().clone(),
            ));
        }
        Symbol::CaptureDescriptor => {
            let mut declarations: Vec<Declaration> = Vec::new();
            if node.children.len() == 3 {
                parse_decls(&node.get_child(2), &mut declarations, prod)?
            }

            let cap_index = node.get_child(0);
            let child_index = if cap_index.is_empty() {
                captures
            } else {
                cap_index
                    .get_child(0)
                    .lhs
                    .lexeme()
                    .parse::<usize>()
                    .unwrap()
            };

            if child_index >= prod.rhs.len() {
                return Err(BuildError::CaptureErr(format!(
                    "Capture index {} out of bounds for production '{}' with {} children",
                    child_index,
                    prod.to_string(),
                    prod.rhs.len()
                )));
            }

            accumulator.push(Segment::Capture(Capture {
                child_index,
                declarations,
            }));
            return Ok(captures + 1);
        }
        _ => {
            let mut new_captures = captures;
            for child in &node.children {
                new_captures = generate_pattern_recursive(child, accumulator, prod, new_captures)?;
            }
            return Ok(new_captures);
        }
    }
    Ok(captures)
}

fn parse_decls<'scope, SpecSymbol: Data + Default>(
    decls_node: &'scope Tree<Symbol>,
    accumulator: &'scope mut Vec<Declaration>,
    prod: &Production<SpecSymbol>,
) -> Result<(), BuildError> {
    accumulator.push(parse_decl(decls_node.children.last().unwrap(), prod)?);
    if decls_node.children.len() == 3 {
        parse_decls(decls_node.get_child(0), accumulator, prod)?;
    }
    Ok(())
}

fn parse_decl<SpecSymbol: Data + Default>(
    decl: &Tree<Symbol>,
    prod: &Production<SpecSymbol>,
) -> Result<Declaration, BuildError> {
    let val_node = decl.get_child(2).get_child(0);
    Ok(Declaration {
        key: decl.get_child(0).lhs.lexeme().clone(),
        value: if val_node.is_null() {
            None
        } else {
            Some(generate_pattern_internal(val_node.get_child(0), prod)?)
        },
    })
}

fn parse_pattern(input: &str) -> Result<Tree<Symbol>, BuildError> {
    PATTERN_ECDFA.with(|cdfa| -> Result<Tree<Symbol>, BuildError> {
        let chars: Vec<char> = input.chars().collect();

        let tokens = scan::def_scanner().scan(&chars[..], cdfa)?;
        let parse = parse::def_parser().parse(tokens, &PATTERN_GRAMMAR)?;
        Ok(parse)
    })
}

#[derive(Debug)]
pub enum BuildError {
    ScanErr(scan::Error),
    ParseErr(parse::Error),
    CaptureErr(String),
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            BuildError::ScanErr(ref err) => write!(f, "Pattern scan error: {}", err),
            BuildError::ParseErr(ref err) => write!(f, "Pattern parse error: {}", err),
            BuildError::CaptureErr(ref err) => write!(f, "Pattern capture error: {}", err),
        }
    }
}

impl error::Error for BuildError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            BuildError::ScanErr(ref err) => Some(err),
            BuildError::ParseErr(ref err) => Some(err),
            BuildError::CaptureErr(_) => None,
        }
    }
}

impl From<scan::Error> for BuildError {
    fn from(err: scan::Error) -> BuildError {
        BuildError::ScanErr(err)
    }
}

impl From<parse::Error> for BuildError {
    fn from(err: parse::Error) -> BuildError {
        BuildError::ParseErr(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pattern_simple() {
        //setup
        let input = "\t \n\n\n\r\n{1}  {2}  {45;something=\n\n \t} {46;somethinelse=\n\n \t;some=}";

        //exercise
        let tree = parse_pattern(input);

        //verify
        assert_eq!(tree.unwrap().to_string(),
                   "└── Pattern
    └── Segments
        ├── Segment
        │   └── Filler
        │       └── TFiller <- '\\t \\n\\n\\n\\r\\n'
        └── Segments
            ├── Segment
            │   └── Capture
            │       ├── TLeftBrace <- '{'
            │       ├── CaptureDescriptor
            │       │   └── CaptureIndex
            │       │       └── TNumber <- '1'
            │       └── TRightBrace <- '}'
            └── Segments
                ├── Segment
                │   └── Filler
                │       └── TFiller <- '  '
                └── Segments
                    ├── Segment
                    │   └── Capture
                    │       ├── TLeftBrace <- '{'
                    │       ├── CaptureDescriptor
                    │       │   └── CaptureIndex
                    │       │       └── TNumber <- '2'
                    │       └── TRightBrace <- '}'
                    └── Segments
                        ├── Segment
                        │   └── Filler
                        │       └── TFiller <- '  '
                        └── Segments
                            ├── Segment
                            │   └── Capture
                            │       ├── TLeftBrace <- '{'
                            │       ├── CaptureDescriptor
                            │       │   ├── CaptureIndex
                            │       │   │   └── TNumber <- '45'
                            │       │   ├── TSemi <- ';'
                            │       │   └── Declarations
                            │       │       └── Declaration
                            │       │           ├── TAlpha <- 'something'
                            │       │           ├── TEquals <- '='
                            │       │           └── Value
                            │       │               └── Pattern
                            │       │                   └── Segments
                            │       │                       ├── Segment
                            │       │                       │   └── Filler
                            │       │                       │       └── TFiller <- '\\n\\n \\t'
                            │       │                       └── Segments
                            │       │                           └──  <- 'NULL'
                            │       └── TRightBrace <- '}'
                            └── Segments
                                ├── Segment
                                │   └── Filler
                                │       └── TFiller <- ' '
                                └── Segments
                                    ├── Segment
                                    │   └── Capture
                                    │       ├── TLeftBrace <- '{'
                                    │       ├── CaptureDescriptor
                                    │       │   ├── CaptureIndex
                                    │       │   │   └── TNumber <- '46'
                                    │       │   ├── TSemi <- ';'
                                    │       │   └── Declarations
                                    │       │       ├── Declarations
                                    │       │       │   └── Declaration
                                    │       │       │       ├── TAlpha <- 'somethinelse'
                                    │       │       │       ├── TEquals <- '='
                                    │       │       │       └── Value
                                    │       │       │           └── Pattern
                                    │       │       │               └── Segments
                                    │       │       │                   ├── Segment
                                    │       │       │                   │   └── Filler
                                    │       │       │                   │       └── TFiller <- '\\n\\n \\t'
                                    │       │       │                   └── Segments
                                    │       │       │                       └──  <- 'NULL'
                                    │       │       ├── TSemi <- ';'
                                    │       │       └── Declaration
                                    │       │           ├── TAlpha <- 'some'
                                    │       │           ├── TEquals <- '='
                                    │       │           └── Value
                                    │       │               └──  <- 'NULL'
                                    │       └── TRightBrace <- '}'
                                    └── Segments
                                        └──  <- 'NULL'"
        );
    }

    #[test]
    fn generate_pattern_simple() {
        //setup
        let input = "\t \n\n\n\n{1}  {2}  {4;something=\n\n \t} {3;somethinelse=\n\n \t;some=}";
        let prod = Production {
            lhs: Symbol::Pattern,
            rhs: vec![
                Symbol::Pattern,
                Symbol::Pattern,
                Symbol::Pattern,
                Symbol::Pattern,
                Symbol::Pattern,
            ],
        };

        //exercise
        let pattern = generate_pattern(input, &prod).unwrap();

        //verify
        assert_eq!(pattern.segments.len(), 8);
        assert!(match pattern.segments.get(0).unwrap() {
            &Segment::Filler(ref s) => "\t \n\n\n\n" == *s,
            &Segment::Substitution(_) => false,
            &Segment::Capture(_) => false,
        });
        assert!(match pattern.segments.get(1).unwrap() {
            &Segment::Filler(_) => false,
            &Segment::Substitution(_) => false,
            &Segment::Capture(ref c) => c.child_index == 1 && c.declarations.len() == 0,
        });
        assert!(match pattern.segments.get(2).unwrap() {
            &Segment::Filler(ref s) => "  " == *s,
            &Segment::Substitution(_) => false,
            &Segment::Capture(_) => false,
        });
        assert!(match pattern.segments.get(3).unwrap() {
            &Segment::Filler(_) => false,
            &Segment::Substitution(_) => false,
            &Segment::Capture(ref c) => c.child_index == 2 && c.declarations.len() == 0,
        });
        assert!(match pattern.segments.get(4).unwrap() {
            &Segment::Filler(ref s) => "  " == *s,
            &Segment::Substitution(_) => false,
            &Segment::Capture(_) => false,
        });
        assert!(match pattern.segments.get(5).unwrap() {
            &Segment::Filler(_) => false,
            &Segment::Substitution(_) => false,
            &Segment::Capture(ref c) => c.child_index == 4 && c.declarations.len() == 1,
        });
        assert!(match pattern.segments.get(4).unwrap() {
            &Segment::Filler(ref s) => "  " == *s,
            &Segment::Substitution(_) => false,
            &Segment::Capture(_) => false,
        });
        assert!(match pattern.segments.get(7).unwrap() {
            &Segment::Filler(_) => false,
            &Segment::Substitution(_) => false,
            &Segment::Capture(ref c) => c.child_index == 3 && c.declarations.len() == 2,
        });
    }

    #[test]
    fn generate_auto_indexed_pattern_simple() {
        //setup
        let input = "\t \n\n\n\n{1}  {}  {;something=\n\n \t} {;somethinelse=\n\n \t;some=}";
        let prod = Production {
            lhs: Symbol::Pattern,
            rhs: vec![
                Symbol::Pattern,
                Symbol::Pattern,
                Symbol::Pattern,
                Symbol::Pattern,
                Symbol::Pattern,
            ],
        };

        //exercise
        let pattern = generate_pattern(input, &prod).unwrap();

        //verify
        assert_eq!(pattern.segments.len(), 8);
        assert!(match pattern.segments.get(0).unwrap() {
            &Segment::Filler(ref s) => "\t \n\n\n\n" == *s,
            &Segment::Substitution(_) => false,
            &Segment::Capture(_) => false,
        });
        assert!(match pattern.segments.get(1).unwrap() {
            &Segment::Filler(_) => false,
            &Segment::Substitution(_) => false,
            &Segment::Capture(ref c) => c.child_index == 1 && c.declarations.len() == 0,
        });
        assert!(match pattern.segments.get(2).unwrap() {
            &Segment::Filler(ref s) => "  " == *s,
            &Segment::Substitution(_) => false,
            &Segment::Capture(_) => false,
        });
        assert!(match pattern.segments.get(3).unwrap() {
            &Segment::Filler(_) => false,
            &Segment::Substitution(_) => false,
            &Segment::Capture(ref c) => c.child_index == 1 && c.declarations.len() == 0,
        });
        assert!(match pattern.segments.get(4).unwrap() {
            &Segment::Filler(ref s) => "  " == *s,
            &Segment::Substitution(_) => false,
            &Segment::Capture(_) => false,
        });
        assert!(match pattern.segments.get(5).unwrap() {
            &Segment::Filler(_) => false,
            &Segment::Substitution(_) => false,
            &Segment::Capture(ref c) => c.child_index == 2 && c.declarations.len() == 1,
        });
        assert!(match pattern.segments.get(4).unwrap() {
            &Segment::Filler(ref s) => "  " == *s,
            &Segment::Substitution(_) => false,
            &Segment::Capture(_) => false,
        });
        assert!(match pattern.segments.get(7).unwrap() {
            &Segment::Filler(_) => false,
            &Segment::Substitution(_) => false,
            &Segment::Capture(ref c) => c.child_index == 3 && c.declarations.len() == 2,
        });
    }

    #[test]
    fn generate_pattern_substitutions() {
        //setup
        let input = "\t \n\r[a]{1}  {} [prefix] ";
        let prod = Production {
            lhs: Symbol::Pattern,
            rhs: vec![Symbol::Pattern, Symbol::Pattern],
        };

        //exercise
        let pattern = generate_pattern(input, &prod).unwrap();

        //verify
        assert_eq!(pattern.segments.len(), 8);
        assert!(match pattern.segments.get(0).unwrap() {
            &Segment::Filler(ref s) => "\t \n\r" == *s,
            &Segment::Substitution(_) => false,
            &Segment::Capture(_) => false,
        });
        assert!(match pattern.segments.get(1).unwrap() {
            &Segment::Filler(_) => false,
            &Segment::Substitution(ref s) => "a" == *s,
            &Segment::Capture(_) => false,
        });
        assert!(match pattern.segments.get(2).unwrap() {
            &Segment::Filler(_) => false,
            &Segment::Substitution(_) => false,
            &Segment::Capture(ref c) => c.child_index == 1 && c.declarations.len() == 0,
        });
        assert!(match pattern.segments.get(3).unwrap() {
            &Segment::Filler(ref s) => "  " == *s,
            &Segment::Substitution(_) => false,
            &Segment::Capture(_) => false,
        });
        assert!(match pattern.segments.get(4).unwrap() {
            &Segment::Filler(_) => false,
            &Segment::Substitution(_) => false,
            &Segment::Capture(ref c) => c.child_index == 1 && c.declarations.len() == 0,
        });
        assert!(match pattern.segments.get(5).unwrap() {
            &Segment::Filler(ref s) => " " == *s,
            &Segment::Substitution(_) => false,
            &Segment::Capture(_) => false,
        });
        assert!(match pattern.segments.get(6).unwrap() {
            &Segment::Filler(_) => false,
            &Segment::Substitution(ref s) => "prefix" == *s,
            &Segment::Capture(_) => false,
        });
        assert!(match pattern.segments.get(7).unwrap() {
            &Segment::Filler(ref s) => " " == *s,
            &Segment::Substitution(_) => false,
            &Segment::Capture(_) => false,
        });
    }

    #[test]
    fn generate_pattern_escaped_filler() {
        //setup
        let input = "1234567890abcdefghijklmnopqrstuvwxyz \n\t`~!@#$%^&*()_-+:'\"<>,.?/|{}\\{\\}\\[\\]\\;\\=\\\\";
        let prod = Production {
            lhs: Symbol::Pattern,
            rhs: vec![Symbol::Pattern],
        };

        //exercise
        let pattern = generate_pattern(input, &prod).unwrap();

        //verify
        assert_eq!(pattern.segments.len(), 5);
        assert!(match pattern.segments.get(0).unwrap() {
            &Segment::Filler(ref s) => "1234567890" == *s,
            &Segment::Substitution(_) => false,
            &Segment::Capture(_) => false,
        });
        assert!(match pattern.segments.get(1).unwrap() {
            &Segment::Filler(ref s) => "abcdefghijklmnopqrstuvwxyz" == *s,
            &Segment::Substitution(_) => false,
            &Segment::Capture(_) => false,
        });
        assert!(match pattern.segments.get(2).unwrap() {
            &Segment::Filler(ref s) => " \n\t`~!@#$%^&*()_-+:'\"<>,.?/|" == *s,
            &Segment::Substitution(_) => false,
            &Segment::Capture(_) => false,
        });
        assert!(match pattern.segments.get(3).unwrap() {
            &Segment::Filler(_) => false,
            &Segment::Substitution(_) => false,
            &Segment::Capture(ref c) => c.child_index == 0 && c.declarations.len() == 0,
        });
        assert!(match pattern.segments.get(4).unwrap() {
            &Segment::Filler(ref s) => "{}[];=\\" == *s,
            &Segment::Substitution(_) => false,
            &Segment::Capture(_) => false,
        });
    }

    #[test]
    fn pattern_scan_error() {
        //setup
        let input = "\\";
        let prod = Production {
            lhs: Symbol::Pattern,
            rhs: vec![],
        };

        //exercise
        let res = generate_pattern(input, &prod);

        //verify
        assert!(res.is_err());
        assert_eq!(
            format!("{}", res.err().unwrap()),
            "Pattern scan error: No accepting scans after (1,1): \\..."
        );
    }

    #[test]
    fn pattern_parse_error() {
        //setup
        let input = "{";
        let prod = Production {
            lhs: Symbol::Pattern,
            rhs: vec![],
        };

        //exercise
        let res = generate_pattern(input, &prod);

        //verify
        assert!(res.is_err());
        assert_eq!(
            format!("{}", res.err().unwrap()),
            "Pattern parse error: Recognition failed after consuming all tokens"
        );
    }

    #[test]
    fn pattern_capture_error() {
        //setup
        let input = "{1}";
        let prod = Production {
            lhs: Symbol::Pattern,
            rhs: vec![Symbol::TSemi],
        };

        //exercise
        let res = generate_pattern(input, &prod);

        //verify
        assert!(res.is_err());
        assert_eq!(
            format!("{}", res.err().unwrap()),
            "Pattern capture error: \
             Capture index 1 out of bounds for production 'Pattern TSemi' with 1 children"
        );
    }
}
