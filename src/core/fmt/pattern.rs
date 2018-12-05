use {
    core::{
        data::Data,
        parse::{
            self,
            grammar::{Grammar, GrammarBuilder},
            Production,
            Tree,
        },
        scan::{
            self,
            compile::{self, CompileTransitionDelta, DFA},
        },
        util::string_utils,
    },
    std::{error, fmt},
};

static PATTERN_ALPHABET: &'static str =
    "{}[];=1234567890abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ \n\t`~!@#$%^&*()_-+:'\"<>,.?/\\|";

#[derive(PartialEq, Clone)]
enum S {
    START,
    LBRACE,
    RBRACE,
    LBRACKET,
    RBRACKET,
    SEMI,
    EQ,
    ZERO,
    NUM,
    ALPHA,
    FILLER,
    ESC,
    FAIL,
}

thread_local! {
    static PATTERN_DFA: DFA<S> = {
        let delta: fn(S, char) -> S = |state, c| match (state, c) {
            (S::START, '{') => S::LBRACE,
            (S::START, '}') => S::RBRACE,
            (S::START, '[') => S::LBRACKET,
            (S::START, ']') => S::RBRACKET,
            (S::START, ';') => S::SEMI,
            (S::START, '=') => S::EQ,
            (S::START, '\\') => S::ESC,
            (S::START, '0') => S::ZERO,
            (S::START, '1') | (S::START, '2') | (S::START, '3') | (S::START, '4') | (S::START, '5') |
            (S::START, '6') | (S::START, '7') | (S::START, '8') | (S::START, '9') => S::NUM,
            (S::START, 'a') | (S::START, 'g') | (S::START, 'l') | (S::START, 'q') | (S::START, 'v') |
            (S::START, 'b') | (S::START, 'h') | (S::START, 'm') | (S::START, 'r') | (S::START, 'w') |
            (S::START, 'c') | (S::START, 'i') | (S::START, 'n') | (S::START, 's') | (S::START, 'x') |
            (S::START, 'd') | (S::START, 'j') | (S::START, 'o') | (S::START, 't') | (S::START, 'y') |
            (S::START, 'e') | (S::START, 'k') | (S::START, 'p') | (S::START, 'u') | (S::START, 'z') |
            (S::START, 'f') | (S::START, 'A') | (S::START, 'B') | (S::START, 'C') | (S::START, 'D') |
            (S::START, 'E') | (S::START, 'F') | (S::START, 'G') | (S::START, 'H') | (S::START, 'I') |
            (S::START, 'J') | (S::START, 'K') | (S::START, 'L') | (S::START, 'M') | (S::START, 'N') |
            (S::START, 'O') | (S::START, 'P') | (S::START, 'Q') | (S::START, 'R') | (S::START, 'S') |
            (S::START, 'T') | (S::START, 'U') | (S::START, 'V') | (S::START, 'W') | (S::START, 'X') |
            (S::START, 'Y') | (S::START, 'Z') => S::ALPHA,
            (S::START, _) => S::FILLER,

            (S::FILLER, '\\') => S::ESC,
            (S::FILLER, '{') | (S::FILLER, '}') | (S::FILLER, '[') | (S::FILLER, ';') |
            (S::FILLER, '=')  => S::FAIL,
            (S::FILLER, _) => S::FILLER,

            (S::ESC, _) => S::FILLER,

            (S::NUM, '0') | (S::NUM, '1') | (S::NUM, '2') | (S::NUM, '3') | (S::NUM, '4') |
            (S::NUM, '5') | (S::NUM, '6') | (S::NUM, '7') | (S::NUM, '8') | (S::NUM, '9') => S::NUM,

            (S::ALPHA, 'a') | (S::ALPHA, 'g') | (S::ALPHA, 'l') | (S::ALPHA, 'q') | (S::ALPHA, 'v') |
            (S::ALPHA, 'b') | (S::ALPHA, 'h') | (S::ALPHA, 'm') | (S::ALPHA, 'r') | (S::ALPHA, 'w') |
            (S::ALPHA, 'c') | (S::ALPHA, 'i') | (S::ALPHA, 'n') | (S::ALPHA, 's') | (S::ALPHA, 'x') |
            (S::ALPHA, 'd') | (S::ALPHA, 'j') | (S::ALPHA, 'o') | (S::ALPHA, 't') | (S::ALPHA, 'y') |
            (S::ALPHA, 'e') | (S::ALPHA, 'k') | (S::ALPHA, 'p') | (S::ALPHA, 'u') | (S::ALPHA, 'z') |
            (S::ALPHA, 'f') | (S::ALPHA, 'A') | (S::ALPHA, 'B') | (S::ALPHA, 'C') | (S::ALPHA, 'D') |
            (S::ALPHA, 'E') | (S::ALPHA, 'F') | (S::ALPHA, 'G') | (S::ALPHA, 'H') | (S::ALPHA, 'I') |
            (S::ALPHA, 'J') | (S::ALPHA, 'K') | (S::ALPHA, 'L') | (S::ALPHA, 'M') | (S::ALPHA, 'N') |
            (S::ALPHA, 'O') | (S::ALPHA, 'P') | (S::ALPHA, 'Q') | (S::ALPHA, 'R') | (S::ALPHA, 'S') |
            (S::ALPHA, 'T') | (S::ALPHA, 'U') | (S::ALPHA, 'V') | (S::ALPHA, 'W') | (S::ALPHA, 'X') |
            (S::ALPHA, 'Y') | (S::ALPHA, 'Z') => S::ALPHA,

            (_, _) => S::FAIL,
        };
        let tokenizer: fn(S) -> String = |state| match state {
            S::SEMI => "SEMI",
            S::EQ => "EQ",
            S::LBRACE => "LBRACE",
            S::RBRACE => "RBRACE",
            S::LBRACKET => "LBRACKET",
            S::RBRACKET => "RBRACKET",
            S::ZERO => "NUM",
            S::NUM => "NUM",
            S::ALPHA => "ALPHA",
            S::FILLER => "FILLER",
            _ => "",
        }.to_string();

        let dfa = DFA{
            alphabet: PATTERN_ALPHABET.to_string(),
            start: S::START,
            td: Box::new(CompileTransitionDelta::build(delta, tokenizer, S::FAIL)),
        };
        dfa
    };
}

lazy_static! {
    static ref PATTERN_PRODUCTIONS: Vec<Production> = {
        parse::build_prods(&[
            "pattern segs",

            "segs seg segs",
            "segs ",

            "seg filler",
            "seg sub",
            "seg cap",

            "filler FILLER",
            "filler ALPHA",
            "filler NUM",

            "sub LBRACKET ALPHA RBRACKET",

            "cap LBRACE capdesc RBRACE",

            "capdesc capindex",
            "capdesc capindex SEMI decls",

            "capindex NUM",
            "capindex ",

            "decls decls SEMI decl",
            "decls decl",

            "decl ALPHA EQ val",

            "val pattern",
            "val ",

        ])
    };

    static ref PATTERN_GRAMMAR: Grammar = {
        let mut builder = GrammarBuilder::new();
        builder.try_mark_start(&PATTERN_PRODUCTIONS.first().unwrap().lhs);
        builder.add_productions(PATTERN_PRODUCTIONS.clone());
        builder.build()
    };
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

pub fn generate_pattern(input: &str, prod: &Production) -> Result<Pattern, BuildError> {
    let parse = parse_pattern(input)?;
    generate_pattern_internal(&parse, prod)
}

pub fn generate_pattern_internal(root: &Tree, prod: &Production) -> Result<Pattern, BuildError> {
    let mut segments: Vec<Segment> = vec![];
    generate_pattern_recursive(&root, &mut segments, prod, 0)?;
    Ok(Pattern { segments })
}

fn generate_pattern_recursive<'a>(
    node: &'a Tree,
    accumulator: &'a mut Vec<Segment>,
    prod: &Production,
    captures: usize,
) -> Result<usize, BuildError> {
    match &node.lhs.kind[..] {
        "FILLER" | "ALPHA" | "NUM" => {
            let name = string_utils::replace_escapes(&node.lhs.lexeme[..]);
            accumulator.push(Segment::Filler(name));
        }
        "sub" => {
            accumulator.push(Segment::Substitution(node.get_child(1).lhs.lexeme.clone()));
        }
        "capdesc" => {
            let mut declarations: Vec<Declaration> = Vec::new();
            if node.children.len() == 3 {
                parse_decls(&node.get_child(2), &mut declarations, prod)?
            }

            let cap_index = node.get_child(0);
            let child_index = if cap_index.is_empty() {
                captures
            } else {
                cap_index.get_child(0).lhs.lexeme.parse::<usize>().unwrap()
            };

            if child_index >= prod.rhs.len() {
                return Err(BuildError::CaptureErr(format!(
                    "Capture index {} out of bounds for production '{}' with {} children",
                    child_index,
                    prod.to_string(),
                    prod.rhs.len()
                )));
            }

            accumulator.push(Segment::Capture(Capture { child_index, declarations }));
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

fn parse_decls<'a>(
    decls_node: &'a Tree,
    accumulator: &'a mut Vec<Declaration>,
    prod: &Production,
) -> Result<(), BuildError> {
    accumulator.push(parse_decl(decls_node.children.last().unwrap(), prod)?);
    if decls_node.children.len() == 3 {
        parse_decls(decls_node.get_child(0), accumulator, prod)?;
    }
    Ok(())
}

fn parse_decl(decl: &Tree, prod: &Production) -> Result<Declaration, BuildError> {
    let val_node = decl.get_child(2).get_child(0);
    Ok(Declaration {
        key: decl.get_child(0).lhs.lexeme.clone(),
        value: if val_node.is_null() {
            None
        } else {
            Some(generate_pattern_internal(val_node.get_child(0), prod)?)
        },
    })
}

fn parse_pattern(input: &str) -> Result<Tree, BuildError> {
    PATTERN_DFA.with(|f| -> Result<Tree, BuildError> {
        let tokens = compile::def_scanner().scan(input, f)?;
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
            BuildError::CaptureErr(ref err) => write!(f, "Pattern capture error: {}", err)
        }
    }
}

impl error::Error for BuildError {
    fn cause(&self) -> Option<&error::Error> {
        match *self {
            BuildError::ScanErr(ref err) => Some(err),
            BuildError::ParseErr(ref err) => Some(err),
            BuildError::CaptureErr(_) => None
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
        let input = "\t \n\n\n\n{1}  {2}  {45;something=\n\n \t} {46;somethinelse=\n\n \t;some=}";

        //exercise
        let tree = parse_pattern(input);

        //verify
        assert_eq!(tree.unwrap().to_string(),
                   "└── pattern
    └── segs
        ├── seg
        │   └── filler
        │       └── FILLER <- '\\t \\n\\n\\n\\n'
        └── segs
            ├── seg
            │   └── cap
            │       ├── LBRACE <- '{'
            │       ├── capdesc
            │       │   └── capindex
            │       │       └── NUM <- '1'
            │       └── RBRACE <- '}'
            └── segs
                ├── seg
                │   └── filler
                │       └── FILLER <- '  '
                └── segs
                    ├── seg
                    │   └── cap
                    │       ├── LBRACE <- '{'
                    │       ├── capdesc
                    │       │   └── capindex
                    │       │       └── NUM <- '2'
                    │       └── RBRACE <- '}'
                    └── segs
                        ├── seg
                        │   └── filler
                        │       └── FILLER <- '  '
                        └── segs
                            ├── seg
                            │   └── cap
                            │       ├── LBRACE <- '{'
                            │       ├── capdesc
                            │       │   ├── capindex
                            │       │   │   └── NUM <- '45'
                            │       │   ├── SEMI <- ';'
                            │       │   └── decls
                            │       │       └── decl
                            │       │           ├── ALPHA <- 'something'
                            │       │           ├── EQ <- '='
                            │       │           └── val
                            │       │               └── pattern
                            │       │                   └── segs
                            │       │                       ├── seg
                            │       │                       │   └── filler
                            │       │                       │       └── FILLER <- '\\n\\n \\t'
                            │       │                       └── segs
                            │       │                           └──  <- 'NULL'
                            │       └── RBRACE <- '}'
                            └── segs
                                ├── seg
                                │   └── filler
                                │       └── FILLER <- ' '
                                └── segs
                                    ├── seg
                                    │   └── cap
                                    │       ├── LBRACE <- '{'
                                    │       ├── capdesc
                                    │       │   ├── capindex
                                    │       │   │   └── NUM <- '46'
                                    │       │   ├── SEMI <- ';'
                                    │       │   └── decls
                                    │       │       ├── decls
                                    │       │       │   └── decl
                                    │       │       │       ├── ALPHA <- 'somethinelse'
                                    │       │       │       ├── EQ <- '='
                                    │       │       │       └── val
                                    │       │       │           └── pattern
                                    │       │       │               └── segs
                                    │       │       │                   ├── seg
                                    │       │       │                   │   └── filler
                                    │       │       │                   │       └── FILLER <- '\\n\\n \\t'
                                    │       │       │                   └── segs
                                    │       │       │                       └──  <- 'NULL'
                                    │       │       ├── SEMI <- ';'
                                    │       │       └── decl
                                    │       │           ├── ALPHA <- 'some'
                                    │       │           ├── EQ <- '='
                                    │       │           └── val
                                    │       │               └──  <- 'NULL'
                                    │       └── RBRACE <- '}'
                                    └── segs
                                        └──  <- 'NULL'"
        );
    }

    #[test]
    fn generate_pattern_simple() {
        //setup
        let input = "\t \n\n\n\n{1}  {2}  {4;something=\n\n \t} {3;somethinelse=\n\n \t;some=}";
        let prod = Production {
            lhs: String::new(),
            rhs: vec![String::new(), String::new(), String::new(), String::new(), String::new()],
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
            lhs: String::new(),
            rhs: vec![String::new(), String::new(), String::new(), String::new(), String::new()],
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
        let input = "\t \n[a]{1}  {} [prefix] ";
        let prod = Production {
            lhs: String::new(),
            rhs: vec![String::new(), String::new()],
        };

        //exercise
        let pattern = generate_pattern(input, &prod).unwrap();

        //verify
        assert_eq!(pattern.segments.len(), 8);
        assert!(match pattern.segments.get(0).unwrap() {
            &Segment::Filler(ref s) => "\t \n" == *s,
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
            lhs: String::new(),
            rhs: vec![String::new()],
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
            lhs: String::new(),
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
            lhs: String::new(),
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
            lhs: String::from("lhs"),
            rhs: vec![String::from("rhs_item")],
        };

        //exercise
        let res = generate_pattern(input, &prod);

        //verify
        assert!(res.is_err());
        assert_eq!(
            format!("{}", res.err().unwrap()),
            "Pattern capture error: Capture index 1 out of bounds for production 'lhs rhs_item' with 1 children"
        );
    }
}
