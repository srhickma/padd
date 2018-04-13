use core::parse::build_prods;
use core::scan::def_scanner;
use core::parse::def_parser;
use core::parse::Grammar;
use core::parse::Production;
use core::parse::Tree;
use core::scan::State;
use core::scan::DFA;

static PATTERN_ALPHABET: &'static str = "{}[];=1234567890abcdefghijklmnopqrstuvwxyz \n\t";
static PATTERN_STATES: [State; 11] = ["start", "semi", "eq", "lbrace", "rbrace", "lbracket", "rbracket", "zero", "num", "alpha", "ws"];
static PATTERN_ACCEPTING: [State; 10] = ["semi", "eq", "lbrace", "rbrace", "lbracket", "rbracket", "zero", "num", "alpha", "ws"];

lazy_static! {
    static ref PATTERN_DFA: DFA<'static> = {
        let start: State = "start";
        let delta: fn(State, char) -> State = |state, c| match (state, c) {
            ("start", '{') => "lbrace",
            ("start", '}') => "rbrace",
            ("start", '[') => "lbracket",
            ("start", ']') => "rbracket",
            ("start", ';') => "semi",
            ("start", '=') => "eq",
            ("start", '0') => "zero",
            ("start", '1') | ("start", '2') | ("start", '3') | ("start", '4') | ("start", '5') |
            ("start", '6') | ("start", '7') | ("start", '8') | ("start", '9') => "num",
            ("start", 'a') | ("start", 'g') | ("start", 'l') | ("start", 'q') | ("start", 'v') |
            ("start", 'b') | ("start", 'h') | ("start", 'm') | ("start", 'r') | ("start", 'w') |
            ("start", 'c') | ("start", 'i') | ("start", 'n') | ("start", 's') | ("start", 'x') |
            ("start", 'd') | ("start", 'j') | ("start", 'o') | ("start", 't') | ("start", 'y') |
            ("start", 'e') | ("start", 'k') | ("start", 'p') | ("start", 'u') | ("start", 'z') |
            ("start", 'f') => "alpha",
            ("start", ' ') => "ws",
            ("start", '\t') => "ws",
            ("start", '\n') => "ws",

            ("num", '0') | ("num", '1') | ("num", '2') | ("num", '3') | ("num", '4') |
            ("num", '5') | ("num", '6') | ("num", '7') | ("num", '8') | ("num", '9') => "num",

            ("alpha", 'a') | ("alpha", 'g') | ("alpha", 'l') | ("alpha", 'q') | ("alpha", 'v') |
            ("alpha", 'b') | ("alpha", 'h') | ("alpha", 'm') | ("alpha", 'r') | ("alpha", 'w') |
            ("alpha", 'c') | ("alpha", 'i') | ("alpha", 'n') | ("alpha", 's') | ("alpha", 'x') |
            ("alpha", 'd') | ("alpha", 'j') | ("alpha", 'o') | ("alpha", 't') | ("alpha", 'y') |
            ("alpha", 'e') | ("alpha", 'k') | ("alpha", 'p') | ("alpha", 'u') | ("alpha", 'z') |
            ("alpha", 'f') => "alpha",

            ("ws", ' ') => "ws",
            ("ws", '\t') => "ws",
            ("ws", '\n') => "ws",

            (&_, _) => "",
        };
        let tokenizer: fn(State) -> &str = |state| match state {
            "semi" => "SEMI",
            "eq" => "EQ",
            "lbrace" => "LBRACE",
            "rbrace" => "RBRACE",
            "lbracket" => "LBRACKET",
            "rbracket" => "RBRACKET",
            "zero" => "NUM",
            "num" => "NUM",
            "alpha" => "ALPHA",
            "ws" => "WHITESPACE",
            _ => "",
        };

        let dfa = DFA{
            alphabet: PATTERN_ALPHABET,
            start,
            accepting: &PATTERN_ACCEPTING,
            delta,
            tokenizer,
        };
        return dfa;
    };

    static ref PATTERN_PRODUCTIONS: Vec<Production<'static>> = {
        return build_prods(&[
            "pattern segs",

            "segs segs seg",
            "segs ",

            "seg filler",
            "seg sub",
            "seg cap",

            "filler WHITESPACE", //For now, only allow whitespace in filler

            "sub LBRACKET ALPHA RBRACKET",

            "cap LBRACE capdesc RBRACE",

            "capdesc NUM",
            "capdesc NUM SEMI decls",

            "decls decl declsopt",

            "declsopt SEMI decl declsopt",
            "declsopt ",

            "decl ALPHA EQ val",

            "val pattern",
            "val ",

        ]);
    };

    static ref PATTERN_GRAMMAR: Grammar<'static> = {
        return Grammar::from(&PATTERN_PRODUCTIONS[..]);
    };
}

pub struct Pattern {
    pub segments: Vec<Segment>,
}

pub enum Segment {
    Filler(String),
    Substitution(String),
    Capture(Capture),
}

pub struct Capture {
    pub child_index: usize,
    pub declarations: Vec<Declaration>,
}

pub struct Declaration {
    pub key: String,
    pub value: Option<Pattern>,
}

pub fn generate_pattern(input: &str) -> Pattern {
    return match parse_pattern(input) {
        Some(root) => generate_pattern_internal(&root),
        None => panic!("Failed to parse pattern"),
    };
}

pub fn generate_pattern_internal<'a>(root: &'a Tree) -> Pattern {
    let mut segments: Vec<Segment> = vec![];
    generate_pattern_recursive(&root, &mut segments);
    return Pattern {
        segments,
    }
}

fn generate_pattern_recursive<'a>(node: &'a Tree, accumulator: &'a mut Vec<Segment>) {
    match &node.lhs.kind[..] {
        "WHITESPACE" => {
            accumulator.push(Segment::Filler(node.lhs.lexeme.clone()));
        },
        "sub" => {
            accumulator.push(Segment::Substitution(node.get_child(1).lhs.lexeme.clone()));
        },
        "capdesc" => {
            let declarations: Vec<Declaration> = if node.children.len() == 3 {
                parse_declarations(&node.get_child(2))
            } else { //No declarations
                vec![]
            };
            accumulator.push(Segment::Capture(Capture{
                child_index: node.get_child(0).lhs.lexeme.parse::<usize>().unwrap(),
                declarations,
            }));
        },
        _ => {
            for child in &node.children {
                generate_pattern_recursive(child, accumulator);
            }
        },
    }
}

fn parse_declarations<'a>(decls_node: &'a Tree) -> Vec<Declaration> {
    let mut declarations: Vec<Declaration> = vec![
        parse_declaration(decls_node.get_child(0)),
    ];
    parse_optional_declarations(decls_node.get_child(1), &mut declarations);
    return declarations;
}

fn parse_optional_declarations<'a>(declsopt_node: &'a Tree, accumulator: &'a mut Vec<Declaration>) {
    if declsopt_node.children.len() == 3 {
        accumulator.push(parse_declaration(declsopt_node.get_child(1)));
        parse_optional_declarations(declsopt_node.get_child(2), accumulator);
    }
}

fn parse_declaration(decl: &Tree) -> Declaration {
    let val_node = decl.get_child(2).get_child(0);
    return Declaration{
        key: decl.get_child(0).lhs.lexeme.clone(),
        value: if val_node.is_null() {
            None
        } else {
            Some(generate_pattern_internal(val_node.get_child(0)))
        },
    }
}

fn parse_pattern(input: &str) -> Option<Tree> {
    let scanner = def_scanner();
    let parser = def_parser();

    let tokens = scanner.scan(input, &PATTERN_DFA);
    return parser.parse(tokens, &PATTERN_GRAMMAR);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pattern_simple() {
        //setup
        let input = "\t \n\n\n\n{1}  {2}  {45;something=\n\n \t} {46;somethinelse=\n\n \t;some=}";

        //execute
        let tree = parse_pattern(input);

        //verify
        assert_eq!(tree.unwrap().to_string(),
"└── pattern
    └── segs
        ├── segs
        │   ├── segs
        │   │   ├── segs
        │   │   │   ├── segs
        │   │   │   │   ├── segs
        │   │   │   │   │   ├── segs
        │   │   │   │   │   │   ├── segs
        │   │   │   │   │   │   │   ├── segs
        │   │   │   │   │   │   │   │   └──  <- 'NULL'
        │   │   │   │   │   │   │   └── seg
        │   │   │   │   │   │   │       └── filler
        │   │   │   │   │   │   │           └── WHITESPACE <- '\\t \\n\\n\\n\\n'
        │   │   │   │   │   │   └── seg
        │   │   │   │   │   │       └── cap
        │   │   │   │   │   │           ├── LBRACE <- '{'
        │   │   │   │   │   │           ├── capdesc
        │   │   │   │   │   │           │   └── NUM <- '1'
        │   │   │   │   │   │           └── RBRACE <- '}'
        │   │   │   │   │   └── seg
        │   │   │   │   │       └── filler
        │   │   │   │   │           └── WHITESPACE <- '  '
        │   │   │   │   └── seg
        │   │   │   │       └── cap
        │   │   │   │           ├── LBRACE <- '{'
        │   │   │   │           ├── capdesc
        │   │   │   │           │   └── NUM <- '2'
        │   │   │   │           └── RBRACE <- '}'
        │   │   │   └── seg
        │   │   │       └── filler
        │   │   │           └── WHITESPACE <- '  '
        │   │   └── seg
        │   │       └── cap
        │   │           ├── LBRACE <- '{'
        │   │           ├── capdesc
        │   │           │   ├── NUM <- '45'
        │   │           │   ├── SEMI <- ';'
        │   │           │   └── decls
        │   │           │       ├── decl
        │   │           │       │   ├── ALPHA <- 'something'
        │   │           │       │   ├── EQ <- '='
        │   │           │       │   └── val
        │   │           │       │       └── pattern
        │   │           │       │           └── segs
        │   │           │       │               ├── segs
        │   │           │       │               │   └──  <- 'NULL'
        │   │           │       │               └── seg
        │   │           │       │                   └── filler
        │   │           │       │                       └── WHITESPACE <- '\\n\\n \\t'
        │   │           │       └── declsopt
        │   │           │           └──  <- 'NULL'
        │   │           └── RBRACE <- '}'
        │   └── seg
        │       └── filler
        │           └── WHITESPACE <- ' '
        └── seg
            └── cap
                ├── LBRACE <- '{'
                ├── capdesc
                │   ├── NUM <- '46'
                │   ├── SEMI <- ';'
                │   └── decls
                │       ├── decl
                │       │   ├── ALPHA <- 'somethinelse'
                │       │   ├── EQ <- '='
                │       │   └── val
                │       │       └── pattern
                │       │           └── segs
                │       │               ├── segs
                │       │               │   └──  <- 'NULL'
                │       │               └── seg
                │       │                   └── filler
                │       │                       └── WHITESPACE <- '\\n\\n \\t'
                │       └── declsopt
                │           ├── SEMI <- ';'
                │           ├── decl
                │           │   ├── ALPHA <- 'some'
                │           │   ├── EQ <- '='
                │           │   └── val
                │           │       └──  <- 'NULL'
                │           └── declsopt
                │               └──  <- 'NULL'
                └── RBRACE <- '}'"
        );
    }

    #[test]
    fn generate_pattern_simple() {
        //setup
        let input = "\t \n\n\n\n{1}  {2}  {45;something=\n\n \t} {46;somethinelse=\n\n \t;some=}";

        //execute
        let pattern = generate_pattern(input);

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
            &Segment::Capture(ref c) => c.child_index == 45 && c.declarations.len() == 1,
        });
        assert!(match pattern.segments.get(4).unwrap() {
            &Segment::Filler(ref s) => "  " == *s,
            &Segment::Substitution(_) => false,
            &Segment::Capture(_) => false,
        });
        assert!(match pattern.segments.get(7).unwrap() {
            &Segment::Filler(_) => false,
            &Segment::Substitution(_) => false,
            &Segment::Capture(ref c) => c.child_index == 46 && c.declarations.len() == 2,
        });
    }
}