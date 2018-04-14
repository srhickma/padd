use core::parse::build_prods;
use core::scan::def_scanner;
use core::parse::def_parser;
use core::parse::Grammar;
use core::parse::Production;
use core::parse::Tree;
use core::scan::State;
use core::scan::Kind;
use core::scan::DFA;
use core::scan::CompileTransitionDelta;
use core::scan::RuntimeTransitionDelta;
use std::collections::HashMap;

static SPEC_ALPHABET: &'static str = "`-=~!@#$%^&*()_+{}|[]\\;':\"<>?,./QWERTYUIOPASDFGHJKLZXCVBNM1234567890abcdefghijklmnopqrstuvwxyz \n\t";
static SPEC_STATES: [&'static str; 14] = ["hat", "minus", "patt", "cil", "comment", "semi", "def", "ws", "id", "arrow", "pattc", "cilc", "cilbs", ""];
static DEF_MATCHER: char = '_';

thread_local! {
    static SPEC_DFA: DFA = {
        let start: State = "start".to_string();
        let delta: fn(&str, char) -> &str = |state, c| match (state, c) {
            ("start", '^') => "hat",
            ("start", '-') => "minus",
            ("start", '\\') => "bslash",
            ("start", '`') => "patt",
            ("start", '\'') => "cil",
            ("start", '#') => "comment",
            ("start", ';') => "semi",
            ("start", '_') => "def",
            ("start", ' ') | ("start", '\t') | ("start", '\n') => "ws",
            ("start", '0') | ("start", '1') | ("start", '2') | ("start", '3') | ("start", '4') |
            ("start", '5') | ("start", '6') | ("start", '7') | ("start", '8') | ("start", '9') |
            ("start", 'a') | ("start", 'g') | ("start", 'l') | ("start", 'q') | ("start", 'v') |
            ("start", 'b') | ("start", 'h') | ("start", 'm') | ("start", 'r') | ("start", 'w') |
            ("start", 'c') | ("start", 'i') | ("start", 'n') | ("start", 's') | ("start", 'x') |
            ("start", 'd') | ("start", 'j') | ("start", 'o') | ("start", 't') | ("start", 'y') |
            ("start", 'e') | ("start", 'k') | ("start", 'p') | ("start", 'u') | ("start", 'z') |
            ("start", 'f') | ("start", 'A') | ("start", 'G') | ("start", 'L') | ("start", 'Q') |
            ("start", 'V') | ("start", 'B') | ("start", 'H') | ("start", 'M') | ("start", 'R') |
            ("start", 'W') | ("start", 'C') | ("start", 'I') | ("start", 'N') | ("start", 'S') |
            ("start", 'X') | ("start", 'D') | ("start", 'J') | ("start", 'O') | ("start", 'T') |
            ("start", 'Y') | ("start", 'E') | ("start", 'K') | ("start", 'P') | ("start", 'U') |
            ("start", 'Z') | ("start", 'F') => "id",

            ("minus", '>') => "arrow",

            ("id", '0') | ("id", '1') | ("id", '2') | ("id", '3') | ("id", '4') |
            ("id", '5') | ("id", '6') | ("id", '7') | ("id", '8') | ("id", '9') |
            ("id", 'a') | ("id", 'g') | ("id", 'l') | ("id", 'q') | ("id", 'v') |
            ("id", 'b') | ("id", 'h') | ("id", 'm') | ("id", 'r') | ("id", 'w') |
            ("id", 'c') | ("id", 'i') | ("id", 'n') | ("id", 's') | ("id", 'x') |
            ("id", 'd') | ("id", 'j') | ("id", 'o') | ("id", 't') | ("id", 'y') |
            ("id", 'e') | ("id", 'k') | ("id", 'p') | ("id", 'u') | ("id", 'z') |
            ("id", 'f') | ("id", 'A') | ("id", 'G') | ("id", 'L') | ("id", 'Q') |
            ("id", 'V') | ("id", 'B') | ("id", 'H') | ("id", 'M') | ("id", 'R') |
            ("id", 'W') | ("id", 'C') | ("id", 'I') | ("id", 'N') | ("id", 'S') |
            ("id", 'X') | ("id", 'D') | ("id", 'J') | ("id", 'O') | ("id", 'T') |
            ("id", 'Y') | ("id", 'E') | ("id", 'K') | ("id", 'P') | ("id", 'U') |
            ("id", 'Z') | ("id", 'F') => "id",

            ("ws", ' ') | ("ws", '\t') | ("ws", '\n') => "ws",

            ("patt", '`') => "pattc",
            ("patt", _) => "patt",

            ("cil", '\'') => "cilc",
            ("cil", '\\') => "cilbs",
            ("cil", _) => "cil",

            ("cilbs", _) => "cil",

            ("comment", '\n') => "",
            ("comment", _) => "comment",

            (&_, _) => "",
        };
        let tokenizer: fn(&str) -> &'static str = |state| match state {
            "hat" => "HAT",
            "arrow" => "ARROW",
            "pattc" => "PATTC",
            "cilc" => "CILC",
            "comment" => "COMMENT",
            "ws" => "WHITESPACE",
            "id" => "ID",
            "def" => "DEF",
            "semi" => "SEMI",
            _ => "",
        };

        DFA{
            alphabet: SPEC_ALPHABET.to_string(),
            start,
            td: Box::new(CompileTransitionDelta::build(&SPEC_STATES, delta, tokenizer)),
        }
    };
}

lazy_static! {
    static ref SPEC_PRODUCTIONS: Vec<Production<'static>> = build_prods(&[
            "spec dfa gram w",

            "dfa states",

            "states states state",
            "states ",

            "state w sdec trans w SEMI ",

            "sdec ID",
            "sdec ID w HAT w ID",
            "sdec DEF",
            "sdec DEF w HAT w ID",

            "trans trans tran",
            "trans ",

            "tran w CILC w ARROW w ID",
            "tran w DEF w ARROW w ID",

            "gram prods",

            "prods prods prod",
            "prods ",

            "prod w ID rhss w SEMI",

            "rhss rhssopt rhs",

            "rhssopt rhssopt rhs",
            "rhssopt ",

            "rhs w ARROW ids pattopt",

            "pattopt w PATTC",
            "pattopt ",

            "ids ids w ID",
            "ids ",

            "w WHITESPACE",
            "w COMMENT WHITESPACE",
            "w WHITESPACE COMMENT WHITESPACE",
            "w ",
        ]);

    static ref SPEC_GRAMMAR: Grammar<'static> = Grammar::from(&SPEC_PRODUCTIONS[..]);
}

fn generate_spec(input: &str){
    match parse_spec(input) {
        None => panic!("Failed to parse specification"),
        Some(parse) => {
            let dfa_tree = parse.get_child(0);
            let grammar_tree = parse.get_child(1);
        },
    }
}

fn generate_dfa(tree: &Tree) -> DFA {
    let mut delta: HashMap<State, HashMap<char, State>> = HashMap::new();
    let mut tokenizer: HashMap<State, Kind> = HashMap::new();

    generate_dfa_internal(tree, &mut delta, &mut tokenizer);

    DFA {
        alphabet: "".to_string(),
        start: "".to_string(),
        td: Box::new(RuntimeTransitionDelta{
            delta,
            tokenizer,
        }),
    }
}

fn generate_dfa_internal<'a>(state_node: &Tree, delta: &mut HashMap<State, HashMap<char, State>>, tokenizer: &mut HashMap<State, Kind>) {

}

fn generate_dfa_state<'a>(state_node: &Tree, delta: &mut HashMap<State, HashMap<char, State>>, tokenizer: &mut HashMap<State, Kind>) {
    let sdec_node = state_node.get_child(1);
    let trans_node = state_node.get_child(2);

    let state: &State = &sdec_node.get_child(0).lhs.lexeme;
    if sdec_node.children.len() == 5 {
        tokenizer.insert(state.clone(), sdec_node.get_child(4).lhs.lexeme.clone());
    }
}

fn generate_dfa_trans<'a>(trans_node: &'a Tree, state_delta: &mut HashMap<char, State>) {
    if !trans_node.is_leaf() {
        let tran_node = trans_node.get_child(1);

        let dest: &State = &tran_node.get_child(5).lhs.lexeme;
        let matcher = tran_node.get_child(1);
        match &matcher.lhs.kind[..] {
            "CILC" => {

            },
            "DEF" => {
                if state_delta.contains_key(&DEF_MATCHER) {
                    panic!("DFA generation failed, more than one default matcher");
                }
                state_delta.insert(DEF_MATCHER, dest.clone());
            },
            &_ => panic!("Transition map input is neither CILC or DEF"),
        }
        generate_dfa_trans(trans_node.get_child(0), state_delta)
    }
}

//fn generate_grammar(tree: &Tree) -> Grammar {
//
//}

fn parse_spec(input: &str) -> Option<Tree> {
    let scanner = def_scanner();
    let parser = def_parser();

    let mut parse: Option<Tree> = None;
    SPEC_DFA.with(|f| {
        let tokens = scanner.scan(input, f);
        parse = parser.parse(tokens, &SPEC_GRAMMAR)
    });
    parse

}

#[cfg(test)]
mod tests {
    use super::*;
    use stopwatch::{Stopwatch};

    #[test]
    fn parse_spec_simple() {
        //setup
        let input = "
# dfa
start ' ' -> ws
 '\t' -> ws
 '\n' -> ws
 '{' -> lbr
 '}' -> rbr
;
ws^WHITESPACE
 ' ' -> ws
 '\t' -> ws
 '\n' -> ws
;
lbr^LBRACKET
;
rbr^RBRACKET
;
# grammar
s -> s b
  ->
;
b -> LBRACKET s RBRACKET ``
  -> w
;
w -> WHITESPACE `[prefix]{0}\n\n{1;prefix=[prefix]\t}[prefix]{2}\n\n`
;
        ";

        //execute
        let tree = parse_spec(input);

        //verify
        assert_eq!(tree.unwrap().to_string(),
"└── spec
    ├── dfa
    │   └── states
    │       ├── states
    │       │   ├── states
    │       │   │   ├── states
    │       │   │   │   ├── states
    │       │   │   │   │   └──  <- 'NULL'
    │       │   │   │   └── state
    │       │   │   │       ├── w
    │       │   │   │       │   ├── WHITESPACE <- '\\n'
    │       │   │   │       │   ├── COMMENT <- '# dfa'
    │       │   │   │       │   └── WHITESPACE <- '\\n'
    │       │   │   │       ├── sdec
    │       │   │   │       │   └── ID <- 'start'
    │       │   │   │       ├── trans
    │       │   │   │       │   ├── trans
    │       │   │   │       │   │   ├── trans
    │       │   │   │       │   │   │   ├── trans
    │       │   │   │       │   │   │   │   ├── trans
    │       │   │   │       │   │   │   │   │   ├── trans
    │       │   │   │       │   │   │   │   │   │   └──  <- 'NULL'
    │       │   │   │       │   │   │   │   │   └── tran
    │       │   │   │       │   │   │   │   │       ├── w
    │       │   │   │       │   │   │   │   │       │   └── WHITESPACE <- ' '
    │       │   │   │       │   │   │   │   │       ├── CILC <- '' ''
    │       │   │   │       │   │   │   │   │       ├── w
    │       │   │   │       │   │   │   │   │       │   └── WHITESPACE <- ' '
    │       │   │   │       │   │   │   │   │       ├── ARROW <- '->'
    │       │   │   │       │   │   │   │   │       ├── w
    │       │   │   │       │   │   │   │   │       │   └── WHITESPACE <- ' '
    │       │   │   │       │   │   │   │   │       └── ID <- 'ws'
    │       │   │   │       │   │   │   │   └── tran
    │       │   │   │       │   │   │   │       ├── w
    │       │   │   │       │   │   │   │       │   └── WHITESPACE <- '\\n '
    │       │   │   │       │   │   │   │       ├── CILC <- ''\\t''
    │       │   │   │       │   │   │   │       ├── w
    │       │   │   │       │   │   │   │       │   └── WHITESPACE <- ' '
    │       │   │   │       │   │   │   │       ├── ARROW <- '->'
    │       │   │   │       │   │   │   │       ├── w
    │       │   │   │       │   │   │   │       │   └── WHITESPACE <- ' '
    │       │   │   │       │   │   │   │       └── ID <- 'ws'
    │       │   │   │       │   │   │   └── tran
    │       │   │   │       │   │   │       ├── w
    │       │   │   │       │   │   │       │   └── WHITESPACE <- '\\n '
    │       │   │   │       │   │   │       ├── CILC <- ''\\n''
    │       │   │   │       │   │   │       ├── w
    │       │   │   │       │   │   │       │   └── WHITESPACE <- ' '
    │       │   │   │       │   │   │       ├── ARROW <- '->'
    │       │   │   │       │   │   │       ├── w
    │       │   │   │       │   │   │       │   └── WHITESPACE <- ' '
    │       │   │   │       │   │   │       └── ID <- 'ws'
    │       │   │   │       │   │   └── tran
    │       │   │   │       │   │       ├── w
    │       │   │   │       │   │       │   └── WHITESPACE <- '\\n '
    │       │   │   │       │   │       ├── CILC <- ''{''
    │       │   │   │       │   │       ├── w
    │       │   │   │       │   │       │   └── WHITESPACE <- ' '
    │       │   │   │       │   │       ├── ARROW <- '->'
    │       │   │   │       │   │       ├── w
    │       │   │   │       │   │       │   └── WHITESPACE <- ' '
    │       │   │   │       │   │       └── ID <- 'lbr'
    │       │   │   │       │   └── tran
    │       │   │   │       │       ├── w
    │       │   │   │       │       │   └── WHITESPACE <- '\\n '
    │       │   │   │       │       ├── CILC <- ''}''
    │       │   │   │       │       ├── w
    │       │   │   │       │       │   └── WHITESPACE <- ' '
    │       │   │   │       │       ├── ARROW <- '->'
    │       │   │   │       │       ├── w
    │       │   │   │       │       │   └── WHITESPACE <- ' '
    │       │   │   │       │       └── ID <- 'rbr'
    │       │   │   │       ├── w
    │       │   │   │       │   └── WHITESPACE <- '\\n'
    │       │   │   │       └── SEMI <- ';'
    │       │   │   └── state
    │       │   │       ├── w
    │       │   │       │   └── WHITESPACE <- '\\n'
    │       │   │       ├── sdec
    │       │   │       │   ├── ID <- 'ws'
    │       │   │       │   ├── w
    │       │   │       │   │   └──  <- 'NULL'
    │       │   │       │   ├── HAT <- '^'
    │       │   │       │   ├── w
    │       │   │       │   │   └──  <- 'NULL'
    │       │   │       │   └── ID <- 'WHITESPACE'
    │       │   │       ├── trans
    │       │   │       │   ├── trans
    │       │   │       │   │   ├── trans
    │       │   │       │   │   │   ├── trans
    │       │   │       │   │   │   │   └──  <- 'NULL'
    │       │   │       │   │   │   └── tran
    │       │   │       │   │   │       ├── w
    │       │   │       │   │   │       │   └── WHITESPACE <- '\\n '
    │       │   │       │   │   │       ├── CILC <- '' ''
    │       │   │       │   │   │       ├── w
    │       │   │       │   │   │       │   └── WHITESPACE <- ' '
    │       │   │       │   │   │       ├── ARROW <- '->'
    │       │   │       │   │   │       ├── w
    │       │   │       │   │   │       │   └── WHITESPACE <- ' '
    │       │   │       │   │   │       └── ID <- 'ws'
    │       │   │       │   │   └── tran
    │       │   │       │   │       ├── w
    │       │   │       │   │       │   └── WHITESPACE <- '\\n '
    │       │   │       │   │       ├── CILC <- ''\\t''
    │       │   │       │   │       ├── w
    │       │   │       │   │       │   └── WHITESPACE <- ' '
    │       │   │       │   │       ├── ARROW <- '->'
    │       │   │       │   │       ├── w
    │       │   │       │   │       │   └── WHITESPACE <- ' '
    │       │   │       │   │       └── ID <- 'ws'
    │       │   │       │   └── tran
    │       │   │       │       ├── w
    │       │   │       │       │   └── WHITESPACE <- '\\n '
    │       │   │       │       ├── CILC <- ''\\n''
    │       │   │       │       ├── w
    │       │   │       │       │   └── WHITESPACE <- ' '
    │       │   │       │       ├── ARROW <- '->'
    │       │   │       │       ├── w
    │       │   │       │       │   └── WHITESPACE <- ' '
    │       │   │       │       └── ID <- 'ws'
    │       │   │       ├── w
    │       │   │       │   └── WHITESPACE <- '\\n'
    │       │   │       └── SEMI <- ';'
    │       │   └── state
    │       │       ├── w
    │       │       │   └── WHITESPACE <- '\\n'
    │       │       ├── sdec
    │       │       │   ├── ID <- 'lbr'
    │       │       │   ├── w
    │       │       │   │   └──  <- 'NULL'
    │       │       │   ├── HAT <- '^'
    │       │       │   ├── w
    │       │       │   │   └──  <- 'NULL'
    │       │       │   └── ID <- 'LBRACKET'
    │       │       ├── trans
    │       │       │   └──  <- 'NULL'
    │       │       ├── w
    │       │       │   └── WHITESPACE <- '\\n'
    │       │       └── SEMI <- ';'
    │       └── state
    │           ├── w
    │           │   └── WHITESPACE <- '\\n'
    │           ├── sdec
    │           │   ├── ID <- 'rbr'
    │           │   ├── w
    │           │   │   └──  <- 'NULL'
    │           │   ├── HAT <- '^'
    │           │   ├── w
    │           │   │   └──  <- 'NULL'
    │           │   └── ID <- 'RBRACKET'
    │           ├── trans
    │           │   └──  <- 'NULL'
    │           ├── w
    │           │   └── WHITESPACE <- '\\n'
    │           └── SEMI <- ';'
    ├── gram
    │   └── prods
    │       ├── prods
    │       │   ├── prods
    │       │   │   ├── prods
    │       │   │   │   └──  <- 'NULL'
    │       │   │   └── prod
    │       │   │       ├── w
    │       │   │       │   ├── WHITESPACE <- '\\n'
    │       │   │       │   ├── COMMENT <- '# grammar'
    │       │   │       │   └── WHITESPACE <- '\\n'
    │       │   │       ├── ID <- 's'
    │       │   │       ├── rhss
    │       │   │       │   ├── rhssopt
    │       │   │       │   │   ├── rhssopt
    │       │   │       │   │   │   └──  <- 'NULL'
    │       │   │       │   │   └── rhs
    │       │   │       │   │       ├── w
    │       │   │       │   │       │   └── WHITESPACE <- ' '
    │       │   │       │   │       ├── ARROW <- '->'
    │       │   │       │   │       ├── ids
    │       │   │       │   │       │   ├── ids
    │       │   │       │   │       │   │   ├── ids
    │       │   │       │   │       │   │   │   └──  <- 'NULL'
    │       │   │       │   │       │   │   ├── w
    │       │   │       │   │       │   │   │   └── WHITESPACE <- ' '
    │       │   │       │   │       │   │   └── ID <- 's'
    │       │   │       │   │       │   ├── w
    │       │   │       │   │       │   │   └── WHITESPACE <- ' '
    │       │   │       │   │       │   └── ID <- 'b'
    │       │   │       │   │       └── pattopt
    │       │   │       │   │           └──  <- 'NULL'
    │       │   │       │   └── rhs
    │       │   │       │       ├── w
    │       │   │       │       │   └── WHITESPACE <- '\\n  '
    │       │   │       │       ├── ARROW <- '->'
    │       │   │       │       ├── ids
    │       │   │       │       │   └──  <- 'NULL'
    │       │   │       │       └── pattopt
    │       │   │       │           └──  <- 'NULL'
    │       │   │       ├── w
    │       │   │       │   └── WHITESPACE <- '\\n'
    │       │   │       └── SEMI <- ';'
    │       │   └── prod
    │       │       ├── w
    │       │       │   └── WHITESPACE <- '\\n'
    │       │       ├── ID <- 'b'
    │       │       ├── rhss
    │       │       │   ├── rhssopt
    │       │       │   │   ├── rhssopt
    │       │       │   │   │   └──  <- 'NULL'
    │       │       │   │   └── rhs
    │       │       │   │       ├── w
    │       │       │   │       │   └── WHITESPACE <- ' '
    │       │       │   │       ├── ARROW <- '->'
    │       │       │   │       ├── ids
    │       │       │   │       │   ├── ids
    │       │       │   │       │   │   ├── ids
    │       │       │   │       │   │   │   ├── ids
    │       │       │   │       │   │   │   │   └──  <- 'NULL'
    │       │       │   │       │   │   │   ├── w
    │       │       │   │       │   │   │   │   └── WHITESPACE <- ' '
    │       │       │   │       │   │   │   └── ID <- 'LBRACKET'
    │       │       │   │       │   │   ├── w
    │       │       │   │       │   │   │   └── WHITESPACE <- ' '
    │       │       │   │       │   │   └── ID <- 's'
    │       │       │   │       │   ├── w
    │       │       │   │       │   │   └── WHITESPACE <- ' '
    │       │       │   │       │   └── ID <- 'RBRACKET'
    │       │       │   │       └── pattopt
    │       │       │   │           ├── w
    │       │       │   │           │   └── WHITESPACE <- ' '
    │       │       │   │           └── PATTC <- '``'
    │       │       │   └── rhs
    │       │       │       ├── w
    │       │       │       │   └── WHITESPACE <- '\\n  '
    │       │       │       ├── ARROW <- '->'
    │       │       │       ├── ids
    │       │       │       │   ├── ids
    │       │       │       │   │   └──  <- 'NULL'
    │       │       │       │   ├── w
    │       │       │       │   │   └── WHITESPACE <- ' '
    │       │       │       │   └── ID <- 'w'
    │       │       │       └── pattopt
    │       │       │           └──  <- 'NULL'
    │       │       ├── w
    │       │       │   └── WHITESPACE <- '\\n'
    │       │       └── SEMI <- ';'
    │       └── prod
    │           ├── w
    │           │   └── WHITESPACE <- '\\n'
    │           ├── ID <- 'w'
    │           ├── rhss
    │           │   ├── rhssopt
    │           │   │   └──  <- 'NULL'
    │           │   └── rhs
    │           │       ├── w
    │           │       │   └── WHITESPACE <- ' '
    │           │       ├── ARROW <- '->'
    │           │       ├── ids
    │           │       │   ├── ids
    │           │       │   │   └──  <- 'NULL'
    │           │       │   ├── w
    │           │       │   │   └── WHITESPACE <- ' '
    │           │       │   └── ID <- 'WHITESPACE'
    │           │       └── pattopt
    │           │           ├── w
    │           │           │   └── WHITESPACE <- ' '
    │           │           └── PATTC <- '`[prefix]{0}\\n\\n{1;prefix=[prefix]\\t}[prefix]{2}\\n\\n`'
    │           ├── w
    │           │   └── WHITESPACE <- '\\n'
    │           └── SEMI <- ';'
    └── w
        └── WHITESPACE <- '\\n        '"
        );
    }
}