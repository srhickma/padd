use core::scan::def_scanner;
use core::scan::DFA;
use core::scan::State;
use core::scan::Kind;
use core::parse::def_parser;
use core::parse::build_prods;
use core::parse::Grammar;
use core::parse::Production;
use core::parse::Tree;

static PATTERN_ALPHABET: &'static str = "{};=1234567890abcdefghijklmnopqrstuvwxyz \n\t";
static PATTERN_STATES: [State; 9] = ["start", "semi", "eq", "lbrace", "rbrace", "zero", "num", "alpha", "ws"];
static PATTERN_ACCEPTING: [State; 8] = ["semi", "eq", "lbrace", "rbrace", "zero", "num", "alpha", "ws"];

lazy_static! {
    static ref PATTERN_DFA: DFA<'static> = {
        let start: State = "start";
        let delta: fn(State, char) -> State = |state, c| match (state, c) {
            ("start", '{') => "lbrace",
            ("start", '}') => "rbrace",
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
            "zero" => "NUM",
            "num" => "NUM",
            "alpha" => "ALPHA",
            "ws" => "WHITESPACE",
            _ => "",
        };

        let dfa = DFA{
            alphabet: PATTERN_ALPHABET,
            states: &PATTERN_STATES,
            start,
            accepting: &PATTERN_ACCEPTING,
            delta,
            tokenizer,
        };
        return dfa;
    };

    static ref PATTERN_PRODUCTIONS: Vec<Production<'static>> = {
        return build_prods(&[
            "pattern secs",

            "secs secs sec",
            "secs ",

            "sec filler",
            "sec cap",

            "filler WHITESPACE", //For now, only allow whitespace in filler

            "cap LBRACE capdesc RBRACE",

            "capdesc NUM",
            "capdesc NUM SEMI decls",

            "decls decl declsopt",

            "declsopt SEMI decl declsopt",
            "declsopt ",

            "decl ALPHA EQ val",

            "val filler",
            "val ",

        ]);
    };

    static ref PATTERN_GRAMMAR: Grammar<'static> = {
        return Grammar::from(&PATTERN_PRODUCTIONS[..]);
    };
}

pub fn parse_pattern(input: &str) -> Option<Tree> {
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
    └── secs
        ├── secs
        │   ├── secs
        │   │   ├── secs
        │   │   │   ├── secs
        │   │   │   │   ├── secs
        │   │   │   │   │   ├── secs
        │   │   │   │   │   │   ├── secs
        │   │   │   │   │   │   │   ├── secs
        │   │   │   │   │   │   │   │   └──  <- NULL
        │   │   │   │   │   │   │   └── sec
        │   │   │   │   │   │   │       └── filler
        │   │   │   │   │   │   │           └── WHITESPACE <- \t \n\n\n\n
        │   │   │   │   │   │   └── sec
        │   │   │   │   │   │       └── cap
        │   │   │   │   │   │           ├── LBRACE <- {
        │   │   │   │   │   │           ├── capdesc
        │   │   │   │   │   │           │   └── NUM <- 1
        │   │   │   │   │   │           └── RBRACE <- }
        │   │   │   │   │   └── sec
        │   │   │   │   │       └── filler
        │   │   │   │   │           └── WHITESPACE <-   \n        │   │   │   │   └── sec
        │   │   │   │       └── cap
        │   │   │   │           ├── LBRACE <- {
        │   │   │   │           ├── capdesc
        │   │   │   │           │   └── NUM <- 2
        │   │   │   │           └── RBRACE <- }
        │   │   │   └── sec
        │   │   │       └── filler
        │   │   │           └── WHITESPACE <-   \n        │   │   └── sec
        │   │       └── cap
        │   │           ├── LBRACE <- {
        │   │           ├── capdesc
        │   │           │   ├── NUM <- 45
        │   │           │   ├── SEMI <- ;
        │   │           │   └── decls
        │   │           │       ├── decl
        │   │           │       │   ├── ALPHA <- something
        │   │           │       │   ├── EQ <- =
        │   │           │       │   └── val
        │   │           │       │       └── filler
        │   │           │       │           └── WHITESPACE <- \n\n \t
        │   │           │       └── declsopt
        │   │           │           └──  <- NULL
        │   │           └── RBRACE <- }
        │   └── sec
        │       └── filler
        │           └── WHITESPACE <-  \n        └── sec
            └── cap
                ├── LBRACE <- {
                ├── capdesc
                │   ├── NUM <- 46
                │   ├── SEMI <- ;
                │   └── decls
                │       ├── decl
                │       │   ├── ALPHA <- somethinelse
                │       │   ├── EQ <- =
                │       │   └── val
                │       │       └── filler
                │       │           └── WHITESPACE <- \n\n \t
                │       └── declsopt
                │           ├── SEMI <- ;
                │           ├── decl
                │           │   ├── ALPHA <- some
                │           │   ├── EQ <- =
                │           │   └── val
                │           │       └──  <- NULL
                │           └── declsopt
                │               └──  <- NULL
                └── RBRACE <- }"
        );
    }
}