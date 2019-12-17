use {
    core::{
        data::Data,
        lex::{
            self,
            ecdfa::{EncodedCDFA, EncodedCDFABuilder},
            CDFABuilder, Transit,
        },
        parse::{
            self,
            grammar::{self, GrammarBuilder, GrammarSymbol, SimpleGrammar, SimpleGrammarBuilder},
            Production, Tree,
        },
        util::string_utils,
    },
    std::{error, fmt},
};

/// S: An enum whose elements are the states of the CDFA for lexing a pattern.
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

/// Returns the ECDFA to lex formatting patterns, or an error if there is an issue with the ECDFA
/// definition (e.g. ambiguity).
fn build_pattern_ecdfa() -> Result<EncodedCDFA<PatternSymbol>, lex::CDFAError> {
    let mut builder: EncodedCDFABuilder<S, PatternSymbol> = EncodedCDFABuilder::new();

    builder.mark_start(&S::Start);

    builder
        .state(&S::Start)
        .mark_trans(Transit::to(S::LeftBrace), '{')?
        .mark_trans(Transit::to(S::RightBrace), '}')?
        .mark_trans(Transit::to(S::LeftBracket), '[')?
        .mark_trans(Transit::to(S::RightBracket), ']')?
        .mark_trans(Transit::to(S::Semi), ';')?
        .mark_trans(Transit::to(S::Equals), '=')?
        .mark_trans(Transit::to(S::Escape), '\\')?
        .mark_trans(Transit::to(S::Zero), '0')?
        .mark_range(Transit::to(S::Number), '1', '9')?
        .mark_range(Transit::to(S::Alpha), 'a', 'z')?
        .mark_range(Transit::to(S::Alpha), 'A', 'Z')?
        .default_to(Transit::to(S::Filler))?;

    builder
        .state(&S::Filler)
        .mark_trans(Transit::to(S::Escape), '\\')?
        .mark_trans(Transit::to(S::Fail), '{')?
        .mark_trans(Transit::to(S::Fail), '}')?
        .mark_trans(Transit::to(S::Fail), '[')?
        .mark_trans(Transit::to(S::Fail), ';')?
        .mark_trans(Transit::to(S::Fail), '=')?
        .default_to(Transit::to(S::Filler))?
        .accept()
        .tokenize(&PatternSymbol::TFiller);

    builder.default_to(&S::Escape, Transit::to(S::Filler))?;

    builder
        .state(&S::Number)
        .mark_range(Transit::to(S::Number), '0', '9')?
        .accept()
        .tokenize(&PatternSymbol::TNumber);

    builder
        .state(&S::Alpha)
        .mark_range(Transit::to(S::Alpha), 'a', 'z')?
        .mark_range(Transit::to(S::Alpha), 'A', 'Z')?
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

/// Pattern Symbol: An enum whose elements are the symbols in the grammar of a pattern.
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
    fn default() -> Self {
        Self::Pattern
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

/// Returns the grammar to parse formatting patterns.
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

/// Pattern: A struct which represents a formatter specification. This is the internal
/// representation of patterns (after parsing + cleanup) used during the formatting traversal.
///
/// # Fields
///
/// * `segments` - a vector storing the different segments/sections of the pattern.
#[derive(Clone)]
pub struct Pattern {
    pub segments: Vec<Segment>,
}

/// Segment: Represents a single section of a pattern.
///
/// # Types
///
/// * `Filler` - stores a literal string of "filler" text, which will be inserted as-is during
/// formatting.
/// * `Substitution` - stores the variable name for a run-time value substitution into the pattern
/// during formatting.
/// * `Capture` - stores a `Capture`, indicating that a child (in the parse tree) should be
/// formatted and inserted at this position during formatting.
#[derive(Clone)]
pub enum Segment {
    Filler(String),
    Substitution(String),
    Capture(Capture),
}

/// Capture: Stores information about the capture of a child's formatting, to be inserted into the
/// pattern string of the parent.
///
/// # Fields
///
/// * `child_index` - stores the index (in the production right-hand-side) of the child to be
/// inserted at this position.
/// * `declarations` - vector of variable declarations to be applied during the scope of this
/// capture (i.e. the scope of this child and its descendants).
#[derive(Clone)]
pub struct Capture {
    pub child_index: usize,
    pub declarations: Vec<Declaration>,
}

/// Declaration: Represents a variable declaration in a `Capture`.
///
/// # Fields
///
/// * `key` - the name of the variable.
/// * `value` - the pattern which should be evaluated to determine the value of the variable. A
/// value of `None` indicates that the variable will be cleared.
#[derive(Clone)]
pub struct Declaration {
    pub key: String,
    pub value: Option<Pattern>,
}

/// Returns a `Pattern` object given an input string storing the pattern (from the specification),
/// or an error if the input string does not represent a valid pattern.
///
/// # Type Parameters
///
/// * `Symbol` - the symbol type of the `Grammar` associated with this pattern.
///
/// # Parameters
///
/// * `input` - the pattern string from the specification.
/// * `prod` - the grammar production this pattern is associated with.
/// * `string_prod` - the string representation of `prod`, useful for building errors.
pub fn generate_pattern<Symbol: GrammarSymbol>(
    input: &str,
    prod: &Production<Symbol>,
    string_prod: &Production<String>,
) -> Result<Pattern, BuildError> {
    let parse = parse_pattern(input)?;
    generate_pattern_internal(&parse, prod, string_prod)
}

/// Returns a `Pattern` object given a pattern parse tree, or an error if the parsed pattern does
/// not represent a valid pattern (i.e. fails context-sensitive analysis).
///
/// # Type Parameters
///
/// * `Symbol` - the symbol type of the `Grammar` associated with this pattern.
///
/// # Parameters
///
/// * `root` - the parse tree generated for a pattern string.
/// * `prod` - the grammar production this pattern is associated with.
/// * `string_prod` - the string representation of `prod`, useful for building errors.
pub fn generate_pattern_internal<Symbol: GrammarSymbol>(
    root: &Tree<PatternSymbol>,
    prod: &Production<Symbol>,
    string_prod: &Production<String>,
) -> Result<Pattern, BuildError> {
    let mut segments: Vec<Segment> = vec![];
    generate_pattern_recursive(&root, &mut segments, prod, string_prod, 0)?;
    Ok(Pattern { segments })
}

/// Traverses the parse tree of a pattern to build its segments and place them in an accumulator.
///
/// Returns the number of capture segments that have been visited so far, or an error if the
/// pattern cannot be built.
///
/// # Type Parameters
///
/// * `Symbol` - the symbol type of the `Grammar` associated with this pattern.
///
/// # Parameters
///
/// * `node` - the parse tree node of the current segment of the pattern.
/// * `accumulator` - a vector to accumulate the segments of the pattern as it is built.
/// * `prod` - the production associated with the pattern being built.
/// * `string_prod` - the string representation of `prod`, useful for building errors.
/// * `captures` - the number of capture segments that have already been visited in the pattern.
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

/// Recursively builds the declarations of a capture segment, placing them in an accumulator.
///
/// Returns an error if the pattern cannot be built.
///
/// # Type Parameters
///
/// * `Symbol` - the symbol type of the `Grammar` associated with this pattern.
///
/// # Parameters
///
/// * `decls_node` - the declarations node in the parse tree.
/// * `accumulator` - a vector to accumulate the declarations of the capture as they are built.
/// * `prod` - the production associated with the pattern being built.
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

/// Builds a `Declaration` from the parse tree of a capture declaration.
///
/// Returns a `Declaration`, or an error if the declaration or any sub-patterns cannot be built.
///
/// # Type Parameters
///
/// * `Symbol` - the symbol type of the `Grammar` associated with this pattern.
///
/// # Parameters
///
/// * `decl` - the declaration node in the parse tree.
/// * `prod` - the production associated with the pattern being built.
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

/// Parses a pattern from an input string.
///
/// Returns the root node of the parse tree, or an error if a pattern could not be parsed from the
/// input.
///
/// # Parameters
///
/// * `input` - the input string from which to parse the pattern.
fn parse_pattern(input: &str) -> Result<Tree<PatternSymbol>, BuildError> {
    PATTERN_ECDFA.with(|cdfa| -> Result<Tree<PatternSymbol>, BuildError> {
        let chars: Vec<char> = input.chars().collect();

        let tokens = lex::def_lexer().lex(&chars[..], cdfa)?;
        let parse = parse::def_parser().parse(tokens, &*PATTERN_GRAMMAR)?;
        Ok(parse)
    })
}

/// Build Error: Represents an error encountered while building a pattern.
///
/// # Types
///
/// * `LexErr` - indicates that an error occurred while lexing a pattern.
/// * `ParseErr` - indicates that an error occurred while parsing a pattern.
/// * `CaptureErr` - indicates that an invalid pattern capture is present (e.g. out-of-bounds).
#[derive(Debug)]
pub enum BuildError {
    LexErr(lex::Error),
    ParseErr(parse::Error),
    CaptureErr(String),
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::LexErr(ref err) => write!(f, "Pattern lex error: {}", err),
            Self::ParseErr(ref err) => write!(f, "Pattern parse error: {}", err),
            Self::CaptureErr(ref err) => write!(f, "Pattern capture error: {}", err),
        }
    }
}

impl error::Error for BuildError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Self::LexErr(ref err) => Some(err),
            Self::ParseErr(ref err) => Some(err),
            Self::CaptureErr(_) => None,
        }
    }
}

impl From<lex::Error> for BuildError {
    fn from(err: lex::Error) -> Self {
        Self::LexErr(err)
    }
}

impl From<parse::Error> for BuildError {
    fn from(err: parse::Error) -> Self {
        Self::ParseErr(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use core::parse::ProductionSymbol;

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
                ProductionSymbol::symbol(PatternSymbol::Pattern),
                ProductionSymbol::symbol(PatternSymbol::Pattern),
                ProductionSymbol::symbol(PatternSymbol::Pattern),
                ProductionSymbol::symbol(PatternSymbol::Pattern),
                ProductionSymbol::symbol(PatternSymbol::Pattern),
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
                ProductionSymbol::symbol(PatternSymbol::Pattern),
                ProductionSymbol::symbol(PatternSymbol::Pattern),
                ProductionSymbol::symbol(PatternSymbol::Pattern),
                ProductionSymbol::symbol(PatternSymbol::Pattern),
                ProductionSymbol::symbol(PatternSymbol::Pattern),
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
            rhs: vec![
                ProductionSymbol::symbol(PatternSymbol::Pattern),
                ProductionSymbol::symbol(PatternSymbol::Pattern),
            ],
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
            rhs: vec![ProductionSymbol::symbol(PatternSymbol::Pattern)],
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
    fn pattern_lex_error() {
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
            "Pattern lex error: No accepting tokens after (1,1): \\..."
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
            rhs: vec![ProductionSymbol::symbol(PatternSymbol::TSemi)],
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
