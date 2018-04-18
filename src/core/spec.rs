use core::parse::build_prods;
use core::scan::def_scanner;
use core::parse::def_parser;
use core::fmt::PatternPair;
use core::fmt::Formatter;
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
            "comment" => "_",
            "ws" => "_",
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
    static ref SPEC_PRODUCTIONS: Vec<Production> = build_prods(&[
            "spec dfa gram",

            "dfa CILC states",

            "states states state",
            "states state",

            "state sdec transopt SEMI ",

            "sdec ID",
            "sdec ID HAT ID",
            "sdec ID HAT DEF",

            "transopt trans",
            "transopt ",

            "trans trans tran",
            "trans tran",

            "tran CILC ARROW ID",
            "tran DEF ARROW ID",

            "gram prods",

            "prods prod prods",
            "prods prod",

            "prod ID rhss SEMI",

            "rhss rhss rhs",
            "rhss rhs",

            "rhs ARROW ids pattopt",

            "pattopt PATTC",
            "pattopt ",

            "ids ID ids",
            "ids ",
        ]);

    static ref SPEC_GRAMMAR: Grammar = Grammar::from(SPEC_PRODUCTIONS.clone());
}

pub fn generate_spec(parse: &Tree) -> (DFA, Grammar, Formatter) {
    let dfa = generate_dfa(parse.get_child(0));
    let (grammar, pattern_pairs) = generate_grammar(parse.get_child(1));
    let formatter = Formatter::create(pattern_pairs);
    (dfa, grammar, formatter)
}

fn generate_dfa(tree: &Tree) -> DFA {
    let mut delta: HashMap<State, HashMap<char, State>> = HashMap::new();
    let mut tokenizer: HashMap<State, Kind> = HashMap::new();

    let alphabet_string = tree.get_child(0).lhs.lexeme.trim_matches('\'');
    let alphabet = alphabet_string.replace("\\n", "\n")
        .replace("\\t", "\t")
        .replace("\\\'", "\'")
        .replace("\\\\", "\\"); //TODO separate, more performant function

    let start = generate_dfa_states(tree.get_child(1), &mut delta, &mut tokenizer);

    DFA {
        alphabet,
        start,
        td: Box::new(RuntimeTransitionDelta{
            delta,
            tokenizer,
        }),
    }
}

fn generate_dfa_states<'a>(states_node: &Tree, delta: &mut HashMap<State, HashMap<char, State>>, tokenizer: &mut HashMap<State, Kind>) -> String {
    let state_node = states_node.get_child(states_node.children.len() - 1);

    let sdec_node = state_node.get_child(0);
    let state: &State = &sdec_node.get_child(0).lhs.lexeme;
    if sdec_node.children.len() == 3 {
        tokenizer.insert(state.clone(), sdec_node.get_child(2).lhs.lexeme.clone());
    }

    let transopt_node = state_node.get_child(1);
    if !transopt_node.is_empty() {
        let mut state_delta: HashMap<char, State> = HashMap::new();
        generate_dfa_trans(transopt_node.get_child(0), &mut state_delta);
        delta.insert(state.clone(), state_delta);
    }

    if states_node.children.len() == 2 {
        return generate_dfa_states(states_node.get_child(0), delta, tokenizer);
    }
    state.clone()
}

fn generate_dfa_trans<'a>(trans_node: &'a Tree, state_delta: &mut HashMap<char, State>) {
    let tran_node = trans_node.get_child(trans_node.children.len() - 1);

    let dest: &State = &tran_node.get_child(2).lhs.lexeme;
    let matcher = tran_node.get_child(0);
    match &matcher.lhs.kind[..] {
        "CILC" => {
            let mut matcher_string = matcher.lhs.lexeme.trim_matches('\'');
            let matcher_cleaned = matcher_string.replace("\\n", "\n")
                .replace("\\t", "\t")
                .replace("\\\'", "\'")
                .replace("\\\\", "\\"); //TODO separate, more performant function
            for c in matcher_cleaned.chars() {
                state_delta.insert(c, dest.clone());
            }
        },
        "DEF" => { //TODO need to incorporate def matcher into dfa in the RuntimeTransitionDelta
            if state_delta.contains_key(&DEF_MATCHER) {
                panic!("DFA generation failed, more than one default matcher");
            }
            state_delta.insert(DEF_MATCHER, dest.clone());
        },
        &_ => panic!("Transition map input is neither CILC or DEF"),
    }
    if trans_node.children.len() == 2 {
        generate_dfa_trans(trans_node.get_child(0), state_delta)
    }
}

fn generate_grammar(tree: &Tree) -> (Grammar, Vec<PatternPair>) {
    let mut productions: Vec<Production> = vec![];
    let mut pattern_pairs: Vec<PatternPair> = vec![];
    generate_grammar_prods(tree.get_child(0), &mut productions, &mut pattern_pairs);

    (Grammar::from(productions), pattern_pairs)
}

fn generate_grammar_prods<'a, 'b>(prods_node: &'a Tree, accumulator: &'b mut Vec<Production>, pp_accumulator: &'b mut Vec<PatternPair>){
    let prod_node = prods_node.get_child(0);

    let id = &prod_node.get_child(0).lhs.lexeme;

    generate_grammar_rhss(prod_node.get_child(1), id, accumulator, pp_accumulator);

    if prods_node.children.len() == 2 {
        generate_grammar_prods(prods_node.get_child(1), accumulator, pp_accumulator);
    }
}

fn generate_grammar_rhss<'a, 'b>(rhss_node: &'a Tree, lhs: &'a String, accumulator: &'b mut Vec<Production>, pp_accumulator: &'b mut Vec<PatternPair>){
    let rhs_node = rhss_node.get_child(rhss_node.children.len() - 1);

    let mut ids: Vec<String> = vec![];
    generate_grammar_ids(rhs_node.get_child(1), &mut ids);

    let production = Production{
        lhs: lhs.clone(),
        rhs: ids,
    };

    accumulator.push(production);

    let pattopt_node = rhs_node.get_child(2);
    if !pattopt_node.is_empty() {
        let pattc = &pattopt_node.get_child(0).lhs.lexeme;
        let pattern_string = &pattc[..].trim_matches('`');
        let pattern = pattern_string.replace("\\n", "\n")
            .replace("\\t", "\t")
            .replace("\\\'", "\'")
            .replace("\\\\", "\\"); //TODO separate, more performant function

        pp_accumulator.push(PatternPair{
            production: accumulator.last().unwrap().to_string(),
            pattern,
        });
    }

    if rhss_node.children.len() == 2 {
        generate_grammar_rhss(rhss_node.get_child(0), lhs, accumulator, pp_accumulator);
    }
}

fn generate_grammar_ids<'a, 'b>(ids_node: &'a Tree, accumulator: &'b mut Vec<String>){
    if !ids_node.is_empty() {
        let id = ids_node.get_child(0).lhs.lexeme.clone();

        accumulator.push(id);

        generate_grammar_ids(ids_node.get_child(1), accumulator)
    }
}

pub fn parse_spec(input: &str) -> Option<Tree> {
    let scanner = def_scanner();
    let parser = def_parser();

    let mut parse: Option<Tree> = None;
    SPEC_DFA.with(|f| {
        let tokens = scanner.scan(input, f);
        parse = parser.parse(tokens.unwrap(), &SPEC_GRAMMAR)
    });
    parse

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_spec_spaces() {
        //setup
        let input = "' 'start;s->s b;";

        //execute
        let tree = parse_spec(input);

        //verify
        assert_eq!(tree.unwrap().to_string(),
"└── spec
    ├── dfa
    │   ├── CILC <- '' ''
    │   └── states
    │       └── state
    │           ├── sdec
    │           │   └── ID <- 'start'
    │           ├── transopt
    │           │   └──  <- 'NULL'
    │           └── SEMI <- ';'
    └── gram
        └── prods
            └── prod
                ├── ID <- 's'
                ├── rhss
                │   └── rhs
                │       ├── ARROW <- '->'
                │       ├── ids
                │       │   ├── ID <- 's'
                │       │   └── ids
                │       │       ├── ID <- 'b'
                │       │       └── ids
                │       │           └──  <- 'NULL'
                │       └── pattopt
                │           └──  <- 'NULL'
                └── SEMI <- ';'"
        );
    }

    #[test]
    fn parse_spec_simple() {
        //setup
        let input = "
' \t\n{}'
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
    │   ├── CILC <- '' \\t\\n{}''
    │   └── states
    │       ├── states
    │       │   ├── states
    │       │   │   ├── states
    │       │   │   │   └── state
    │       │   │   │       ├── sdec
    │       │   │   │       │   └── ID <- 'start'
    │       │   │   │       ├── transopt
    │       │   │   │       │   └── trans
    │       │   │   │       │       ├── trans
    │       │   │   │       │       │   ├── trans
    │       │   │   │       │       │   │   ├── trans
    │       │   │   │       │       │   │   │   ├── trans
    │       │   │   │       │       │   │   │   │   └── tran
    │       │   │   │       │       │   │   │   │       ├── CILC <- '' ''
    │       │   │   │       │       │   │   │   │       ├── ARROW <- '->'
    │       │   │   │       │       │   │   │   │       └── ID <- 'ws'
    │       │   │   │       │       │   │   │   └── tran
    │       │   │   │       │       │   │   │       ├── CILC <- ''\\t''
    │       │   │   │       │       │   │   │       ├── ARROW <- '->'
    │       │   │   │       │       │   │   │       └── ID <- 'ws'
    │       │   │   │       │       │   │   └── tran
    │       │   │   │       │       │   │       ├── CILC <- ''\\n''
    │       │   │   │       │       │   │       ├── ARROW <- '->'
    │       │   │   │       │       │   │       └── ID <- 'ws'
    │       │   │   │       │       │   └── tran
    │       │   │   │       │       │       ├── CILC <- ''{''
    │       │   │   │       │       │       ├── ARROW <- '->'
    │       │   │   │       │       │       └── ID <- 'lbr'
    │       │   │   │       │       └── tran
    │       │   │   │       │           ├── CILC <- ''}''
    │       │   │   │       │           ├── ARROW <- '->'
    │       │   │   │       │           └── ID <- 'rbr'
    │       │   │   │       └── SEMI <- ';'
    │       │   │   └── state
    │       │   │       ├── sdec
    │       │   │       │   ├── ID <- 'ws'
    │       │   │       │   ├── HAT <- '^'
    │       │   │       │   └── ID <- 'WHITESPACE'
    │       │   │       ├── transopt
    │       │   │       │   └── trans
    │       │   │       │       ├── trans
    │       │   │       │       │   ├── trans
    │       │   │       │       │   │   └── tran
    │       │   │       │       │   │       ├── CILC <- '' ''
    │       │   │       │       │   │       ├── ARROW <- '->'
    │       │   │       │       │   │       └── ID <- 'ws'
    │       │   │       │       │   └── tran
    │       │   │       │       │       ├── CILC <- ''\\t''
    │       │   │       │       │       ├── ARROW <- '->'
    │       │   │       │       │       └── ID <- 'ws'
    │       │   │       │       └── tran
    │       │   │       │           ├── CILC <- ''\\n''
    │       │   │       │           ├── ARROW <- '->'
    │       │   │       │           └── ID <- 'ws'
    │       │   │       └── SEMI <- ';'
    │       │   └── state
    │       │       ├── sdec
    │       │       │   ├── ID <- 'lbr'
    │       │       │   ├── HAT <- '^'
    │       │       │   └── ID <- 'LBRACKET'
    │       │       ├── transopt
    │       │       │   └──  <- 'NULL'
    │       │       └── SEMI <- ';'
    │       └── state
    │           ├── sdec
    │           │   ├── ID <- 'rbr'
    │           │   ├── HAT <- '^'
    │           │   └── ID <- 'RBRACKET'
    │           ├── transopt
    │           │   └──  <- 'NULL'
    │           └── SEMI <- ';'
    └── gram
        └── prods
            ├── prod
            │   ├── ID <- 's'
            │   ├── rhss
            │   │   ├── rhss
            │   │   │   └── rhs
            │   │   │       ├── ARROW <- '->'
            │   │   │       ├── ids
            │   │   │       │   ├── ID <- 's'
            │   │   │       │   └── ids
            │   │   │       │       ├── ID <- 'b'
            │   │   │       │       └── ids
            │   │   │       │           └──  <- 'NULL'
            │   │   │       └── pattopt
            │   │   │           └──  <- 'NULL'
            │   │   └── rhs
            │   │       ├── ARROW <- '->'
            │   │       ├── ids
            │   │       │   └──  <- 'NULL'
            │   │       └── pattopt
            │   │           └──  <- 'NULL'
            │   └── SEMI <- ';'
            └── prods
                ├── prod
                │   ├── ID <- 'b'
                │   ├── rhss
                │   │   ├── rhss
                │   │   │   └── rhs
                │   │   │       ├── ARROW <- '->'
                │   │   │       ├── ids
                │   │   │       │   ├── ID <- 'LBRACKET'
                │   │   │       │   └── ids
                │   │   │       │       ├── ID <- 's'
                │   │   │       │       └── ids
                │   │   │       │           ├── ID <- 'RBRACKET'
                │   │   │       │           └── ids
                │   │   │       │               └──  <- 'NULL'
                │   │   │       └── pattopt
                │   │   │           └── PATTC <- '``'
                │   │   └── rhs
                │   │       ├── ARROW <- '->'
                │   │       ├── ids
                │   │       │   ├── ID <- 'w'
                │   │       │   └── ids
                │   │       │       └──  <- 'NULL'
                │   │       └── pattopt
                │   │           └──  <- 'NULL'
                │   └── SEMI <- ';'
                └── prods
                    └── prod
                        ├── ID <- 'w'
                        ├── rhss
                        │   └── rhs
                        │       ├── ARROW <- '->'
                        │       ├── ids
                        │       │   ├── ID <- 'WHITESPACE'
                        │       │   └── ids
                        │       │       └──  <- 'NULL'
                        │       └── pattopt
                        │           └── PATTC <- '`[prefix]{0}\\n\\n{1;prefix=[prefix]\\t}[prefix]{2}\\n\\n`'
                        └── SEMI <- ';'"
        );
    }

    #[test]
    fn generate_spec_simple() {
        //setup
        let spec = "
' \\t\\n{}'

# dfa
start ' \\t\\n' -> ws
 '{' -> lbr
 '}' -> rbr;
ws^WHITESPACE
 ' \\t\\n' -> ws;
lbr^LBRACKET;
rbr^RBRACKET;

# grammar
s -> s b
  -> ;
b -> LBRACKET s RBRACKET `[prefix]{0}\\n\\n{1;prefix=[prefix]\\t}[prefix]{2}\\n\\n`
  -> w ;
w -> WHITESPACE ``;
        ";

        let input = "  {  {  {{{\t}}}\n {} }  }   { {}\n } ";

        let scanner = def_scanner();
        let parser = def_parser();

        //specification
        let tree = parse_spec(spec);
        let parse = tree.unwrap();
        let (dfa, grammar, formatter) = generate_spec(&parse);

        //input
        let tokens = scanner.scan(input, &dfa);
        let tree = parser.parse(tokens.unwrap(), &grammar);
        let parse = tree.unwrap();

        //exercise
        let res = formatter.format(&parse);

        //verify
        assert_eq!(res,
"{

	{

		{

			{

				{

				}

			}

		}

		{

		}

	}

}

{

	{

	}

}\n\n"
        );
    }
}