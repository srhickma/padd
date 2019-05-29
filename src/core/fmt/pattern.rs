use {
    core::{
        data::Data,
        parse::{
            self,
            grammar::{self, GrammarBuilder, GrammarSymbol, SimpleGrammar, SimpleGrammarBuilder},
            Production, Tree,
        },
        scan::{
            self,
            ecdfa::{EncodedCDFA, EncodedCDFABuilder},
            CDFABuilder, ConsumerStrategy,
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

impl Data for S {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

thread_local! {
    static PATTERN_ECDFA: EncodedCDFA<PatternSymbol> = build_pattern_ecdfa().unwrap();
}

fn build_pattern_ecdfa() -> Result<EncodedCDFA<PatternSymbol>, scan::CDFAError> {
    let mut builder: EncodedCDFABuilder<S, PatternSymbol> = EncodedCDFABuilder::new();

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
        .tokenize(&PatternSymbol::TFiller);

    builder.default_to(&S::Escape, &S::Filler, ConsumerStrategy::All)?;

    builder
        .state(&S::Number)
        .mark_range(&S::Number, '0', '9')?
        .accept()
        .tokenize(&PatternSymbol::TNumber);

    builder
        .state(&S::Alpha)
        .mark_range(&S::Alpha, 'a', 'Z')?
        .accept()
        .tokenize(&PatternSymbol::TAlpha);

    builder
        .accept(&S::Semi)
        .accept(&S::Equals)
        .accept(&S::LeftBrace)
        .accept(&S::RightBrace)
        .accept(&S::LeftBracket)
        .accept(&S::RightBracket)
        .accept(&S::Zero);

    builder
        .tokenize(&S::Semi, &PatternSymbol::TSemi)
        .tokenize(&S::Equals, &PatternSymbol::TEquals)
        .tokenize(&S::LeftBrace, &PatternSymbol::TLeftBrace)
        .tokenize(&S::RightBrace, &PatternSymbol::TRightBrace)
        .tokenize(&S::LeftBracket, &PatternSymbol::TLeftBracket)
        .tokenize(&S::RightBracket, &PatternSymbol::TRightBracket)
        .tokenize(&S::Zero, &PatternSymbol::TNumber);

    builder.build()
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum PatternSymbol {
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

impl Default for PatternSymbol {
    fn default() -> PatternSymbol {
        PatternSymbol::Pattern
    }
}

impl Data for PatternSymbol {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

impl GrammarSymbol for PatternSymbol {}

lazy_static! {
    static ref PATTERN_GRAMMAR: SimpleGrammar<PatternSymbol> = build_pattern_grammar().unwrap();
}

fn build_pattern_grammar() -> Result<SimpleGrammar<PatternSymbol>, grammar::BuildError> {
    //TODO optimize for left recursion

    let mut builder = SimpleGrammarBuilder::new();
    builder.try_mark_start(&PatternSymbol::Pattern);

    builder
        .from(PatternSymbol::Pattern)
        .to(vec![PatternSymbol::Segments]);

    builder
        .from(PatternSymbol::Segments)
        .to(vec![PatternSymbol::Segment, PatternSymbol::Segments])
        .epsilon();

    builder
        .from(PatternSymbol::Segment)
        .to(vec![PatternSymbol::Filler])
        .to(vec![PatternSymbol::Substitution])
        .to(vec![PatternSymbol::Capture]);

    builder
        .from(PatternSymbol::Filler)
        .to(vec![PatternSymbol::TFiller])
        .to(vec![PatternSymbol::TAlpha])
        .to(vec![PatternSymbol::TNumber]);

    builder.from(PatternSymbol::Substitution).to(vec![
        PatternSymbol::TLeftBracket,
        PatternSymbol::TAlpha,
        PatternSymbol::TRightBracket,
    ]);

    builder.from(PatternSymbol::Capture).to(vec![
        PatternSymbol::TLeftBrace,
        PatternSymbol::CaptureDescriptor,
        PatternSymbol::TRightBrace,
    ]);

    builder
        .from(PatternSymbol::CaptureDescriptor)
        .to(vec![PatternSymbol::CaptureIndex])
        .to(vec![
            PatternSymbol::CaptureIndex,
            PatternSymbol::TSemi,
            PatternSymbol::Declarations,
        ]);

    builder
        .from(PatternSymbol::CaptureIndex)
        .to(vec![PatternSymbol::TNumber])
        .epsilon();

    builder
        .from(PatternSymbol::Declarations)
        .to(vec![
            PatternSymbol::Declarations,
            PatternSymbol::TSemi,
            PatternSymbol::Declaration,
        ])
        .to(vec![PatternSymbol::Declaration]);

    builder.from(PatternSymbol::Declaration).to(vec![
        PatternSymbol::TAlpha,
        PatternSymbol::TEquals,
        PatternSymbol::Value,
    ]);

    builder
        .from(PatternSymbol::Value)
        .to(vec![PatternSymbol::Pattern])
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

pub fn generate_pattern<Symbol: GrammarSymbol>(
    input: &str,
    prod: &Production<Symbol>,
    string_prod: &Production<String>,
) -> Result<Pattern, BuildError> {
    let parse = parse_pattern(input)?;
    generate_pattern_internal(&parse, prod, string_prod)
}

pub fn generate_pattern_internal<Symbol: GrammarSymbol>(
    root: &Tree<PatternSymbol>,
    prod: &Production<Symbol>,
    string_prod: &Production<String>,
) -> Result<Pattern, BuildError> {
    let mut segments: Vec<Segment> = vec![];
    generate_pattern_recursive(&root, &mut segments, prod, string_prod, 0)?;
    Ok(Pattern { segments })
}

fn generate_pattern_recursive<'scope, Symbol: GrammarSymbol>(
    node: &'scope Tree<PatternSymbol>,
    accumulator: &'scope mut Vec<Segment>,
    prod: &Production<Symbol>,
    string_prod: &Production<String>,
    captures: usize,
) -> Result<usize, BuildError> {
    if node.lhs.is_null() {
        return Ok(captures);
    }

    match node.lhs.kind() {
        PatternSymbol::TFiller | PatternSymbol::TAlpha | PatternSymbol::TNumber => {
            let name = string_utils::replace_escapes(&node.lhs.lexeme()[..]);
            accumulator.push(Segment::Filler(name));
        }
        PatternSymbol::Substitution => {
            accumulator.push(Segment::Substitution(
                node.get_child(1).lhs.lexeme().clone(),
            ));
        }
        PatternSymbol::CaptureDescriptor => {
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
                    string_prod.to_string(),
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
                new_captures = generate_pattern_recursive(
                    child,
                    accumulator,
                    prod,
                    string_prod,
                    new_captures,
                )?;
            }
            return Ok(new_captures);
        }
    }
    Ok(captures)
}

fn parse_decls<'scope, Symbol: GrammarSymbol>(
    decls_node: &'scope Tree<PatternSymbol>,
    accumulator: &'scope mut Vec<Declaration>,
    prod: &Production<Symbol>,
) -> Result<(), BuildError> {
    accumulator.push(parse_decl(decls_node.children.last().unwrap(), prod)?);
    if decls_node.children.len() == 3 {
        parse_decls(decls_node.get_child(0), accumulator, prod)?;
    }
    Ok(())
}

fn parse_decl<Symbol: GrammarSymbol>(
    decl: &Tree<PatternSymbol>,
    prod: &Production<Symbol>,
) -> Result<Declaration, BuildError> {
    let val_node = decl.get_child(2).get_child(0);
    Ok(Declaration {
        key: decl.get_child(0).lhs.lexeme().clone(),
        value: if val_node.is_null() {
            None
        } else {
            Some(generate_pattern_internal(
                val_node.get_child(0),
                prod,
                &prod.string_production(),
            )?)
        },
    })
}

fn parse_pattern(input: &str) -> Result<Tree<PatternSymbol>, BuildError> {
    PATTERN_ECDFA.with(|cdfa| -> Result<Tree<PatternSymbol>, BuildError> {
        let chars: Vec<char> = input.chars().collect();

        let tokens = scan::def_scanner().scan(&chars[..], cdfa)?;
        let parse = parse::def_parser().parse(tokens, &*PATTERN_GRAMMAR)?;
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
            lhs: PatternSymbol::Pattern,
            rhs: vec![
                PatternSymbol::Pattern,
                PatternSymbol::Pattern,
                PatternSymbol::Pattern,
                PatternSymbol::Pattern,
                PatternSymbol::Pattern,
            ],
        };

        //exercise
        let pattern = generate_pattern(input, &prod, &prod.string_production()).unwrap();

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
            lhs: PatternSymbol::Pattern,
            rhs: vec![
                PatternSymbol::Pattern,
                PatternSymbol::Pattern,
                PatternSymbol::Pattern,
                PatternSymbol::Pattern,
                PatternSymbol::Pattern,
            ],
        };

        //exercise
        let pattern = generate_pattern(input, &prod, &prod.string_production()).unwrap();

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
            lhs: PatternSymbol::Pattern,
            rhs: vec![PatternSymbol::Pattern, PatternSymbol::Pattern],
        };

        //exercise
        let pattern = generate_pattern(input, &prod, &prod.string_production()).unwrap();

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
            lhs: PatternSymbol::Pattern,
            rhs: vec![PatternSymbol::Pattern],
        };

        //exercise
        let pattern = generate_pattern(input, &prod, &prod.string_production()).unwrap();

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
            lhs: PatternSymbol::Pattern,
            rhs: vec![],
        };

        //exercise
        let res = generate_pattern(input, &prod, &prod.string_production());

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
            lhs: PatternSymbol::Pattern,
            rhs: vec![],
        };

        //exercise
        let res = generate_pattern(input, &prod, &prod.string_production());

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
            lhs: PatternSymbol::Pattern,
            rhs: vec![PatternSymbol::TSemi],
        };

        //exercise
        let res = generate_pattern(input, &prod, &prod.string_production());

        //verify
        assert!(res.is_err());
        assert_eq!(
            format!("{}", res.err().unwrap()),
            "Pattern capture error: \
             Capture index 1 out of bounds for production 'Pattern TSemi' with 1 children"
        );
    }
}
