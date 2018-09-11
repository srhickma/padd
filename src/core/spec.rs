use std;
use std::error;
use core::parse::build_prods;
use core::scan::def_scanner;
use core::parse::def_parser;
use core::fmt;
use core::fmt::PatternPair;
use core::fmt::Formatter;
use core::parse;
use core::parse::Grammar;
use core::parse::Production;
use core::parse::Tree;
use core::scan;
use core::scan::State;
use core::scan::Kind;
use core::scan::DFA;
use core::scan::CompileTransitionDelta;
use core::scan::RuntimeTransitionDelta;
use std::collections::HashMap;

static SPEC_ALPHABET: &'static str = "`-=~!@#$%^&*()_+{}|[]\\;':\"<>?,./QWERTYUIOPASDFGHJKLZXCVBNM1234567890abcdefghijklmnopqrstuvwxyz \n\t";
static SPEC_STATES: [&'static str; 15] = ["hat", "minus", "patt", "cil", "comment", "semi", "def", "ws", "id", "arrow", "pattc", "cilc", "cilbs", "or", ""];
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
            ("start", '|') => "or",
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
            ("id", 'Z') | ("id", 'F') | ("id", '_') => "id",

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
            "or" => "OR",
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

            "sdec targets",
            "sdec targets HAT ID",
            "sdec targets HAT DEF",

            "targets ID",
            "targets ID OR targets",

            "transopt trans",
            "transopt ",

            "trans trans tran",
            "trans tran",

            "tran mtcs ARROW trand",
            "tran DEF ARROW trand",

            "trand ID",
            "trand HAT ID",
            "trand HAT DEF",

            "mtcs CILC OR mtcs",
            "mtcs CILC",

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

pub fn generate_spec(parse: &Tree) -> Result<(DFA, Grammar, Formatter), GenError> {
    let dfa = generate_dfa(parse.get_child(0));
    let (grammar, pattern_pairs) = generate_grammar(parse.get_child(1));
    let formatter = Formatter::create(pattern_pairs)?;
    Ok((dfa, grammar, formatter))
}

pub type GenError = fmt::BuildError;

fn generate_dfa(tree: &Tree) -> DFA {
    let mut delta: HashMap<State, HashMap<char, State>> = HashMap::new();
    let mut tokenizer: HashMap<State, Kind> = HashMap::new();

    let alphabet_string = tree.get_child(0).lhs.lexeme.trim_matches('\'');
    let alphabet = replace_escapes(&alphabet_string);

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

    let targets_node = sdec_node.get_child(0);
    let head_state = &targets_node.get_child(0).lhs.lexeme;
    let mut tail_states: Vec<&State> = vec![];
    if targets_node.children.len() == 3 {
        generate_dfa_targets(targets_node.get_child(2), &mut tail_states);
    }

    if sdec_node.children.len() == 3 {
        let end = &sdec_node.get_child(2).lhs.lexeme;
        tokenizer.insert(head_state.clone(), end.clone());
        for state in &tail_states {
            tokenizer.insert((*state).clone(), end.clone());
        }
    }

    let transopt_node = state_node.get_child(1);
    if !transopt_node.is_empty() {
        let mut state_delta: HashMap<char, State> = HashMap::new();

        //TODO remove this after CDFA
        generate_dfa_trans_def_prefill(transopt_node.get_child(0), &mut state_delta);

        generate_dfa_trans(transopt_node.get_child(0), &mut state_delta, delta, tokenizer, head_state);
        for state in &tail_states {
            extend_delta(*state, state_delta.clone(), delta);
        }
        extend_delta(head_state, state_delta, delta);
    }

    if states_node.children.len() == 2 {
        return generate_dfa_states(states_node.get_child(0), delta, tokenizer);
    }
    head_state.clone()
}

fn extend_delta(state: &State, state_delta: HashMap<char, State>, delta: &mut HashMap<State, HashMap<char, State>>){
    if delta.contains_key(state) {
        delta.get_mut(state).unwrap().extend(state_delta);
    } else {
        delta.insert(state.clone(), state_delta);
    }
}

fn generate_dfa_targets<'a>(targets_node: &'a Tree, accumulator: &mut Vec<&'a State>){
    accumulator.push(&targets_node.get_child(0).lhs.lexeme);
    if targets_node.children.len() == 3 {
        generate_dfa_targets(targets_node.get_child(2), accumulator);
    }
}

fn generate_dfa_trans_def_prefill<'a>(trans_node: &'a Tree, state_delta: &mut HashMap<char, State>){
    let tran_node = trans_node.get_child(trans_node.children.len() - 1);

    let trand_node = tran_node.get_child(2);
    let dest: &State = &trand_node.get_child(trand_node.children.len() - 1).lhs.lexeme;

    let matcher = tran_node.get_child(0);
    match &matcher.lhs.kind[..] {
        "DEF" => {
            state_delta.insert(DEF_MATCHER, dest.clone());
            return;
        },
        &_ => {},
    }

    if trans_node.children.len() == 2 {
        generate_dfa_trans_def_prefill(trans_node.get_child(0), state_delta);
    }
}

fn generate_dfa_trans<'a>(trans_node: &'a Tree, state_delta: &mut HashMap<char, State>, delta: &mut HashMap<State, HashMap<char, State>>, tokenizer: &mut HashMap<State, Kind>, src: &State){
    let tran_node = trans_node.get_child(trans_node.children.len() - 1);

    let trand_node = tran_node.get_child(2);
    let dest: &State = &trand_node.get_child(trand_node.children.len() - 1).lhs.lexeme;

    let matcher = tran_node.get_child(0);
    match &matcher.lhs.kind[..] {
        "mtcs" => {
            generate_dfa_mtcs(matcher, state_delta, delta, tokenizer, src, dest);
        },
        "DEF" => {
            //TODO fill with CDFA
        },
        &_ => panic!("Transition map input is neither CILC or DEF"),
    }

    if trand_node.children.len() == 2 { //Immediate state pass-through
        tokenizer.insert(dest.clone(), dest.clone());
    }

    if trans_node.children.len() == 2 {
        generate_dfa_trans(trans_node.get_child(0), state_delta, delta, tokenizer, src);
    }
}

fn generate_dfa_mtcs<'a>(mtcs_node: &'a Tree, state_delta: &mut HashMap<char, State>, delta: &mut HashMap<State, HashMap<char, State>>, tokenizer: &mut HashMap<State, Kind>, src: &State, dest: &State){
    let matcher = mtcs_node.children.first().unwrap();
    let matcher_string = matcher.lhs.lexeme.trim_matches('\'');
    let matcher_cleaned = replace_escapes(&matcher_string);
    let mut chars = matcher_cleaned.chars();
    if matcher_cleaned.len() == 1 {
        state_delta.insert(chars.next().unwrap(), dest.clone());
    } else {
        let mut from : State = src.clone();
        let first_char = chars.next().unwrap();
        let mut to: String = format!("#{}#{}", from, first_char);
        state_delta.insert(first_char, to.clone());

        //TODO remove with CDFA
        let def_trans = state_delta.get(&DEF_MATCHER);

        let mut i = 1;
        for c in chars {
            from = to.clone(); //TODO try to reduce cloning
            to = if i == matcher_cleaned.len() - 1 {
                dest.clone()
            } else {
                format!("{}{}", &to, c)
            };

            delta.entry(from.clone())
                .or_insert(HashMap::new())
                .insert(c, to.clone());

            match def_trans {
                Some(state) => {
                    delta.get_mut(&from).unwrap().insert(DEF_MATCHER, state.clone());
                },
                None => {}
            };

            i += 1;
        }
    }

    if mtcs_node.children.len() == 3 {
        generate_dfa_mtcs(mtcs_node.get_child(2), state_delta, delta, tokenizer, src, dest);
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
        let pattern = replace_escapes(pattern_string);

        pp_accumulator.push(PatternPair{
            production: accumulator.last().unwrap().clone(),
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

pub fn parse_spec(input: &str) -> Result<Tree, ParseError> {
    SPEC_DFA.with(|f| -> Result<Tree, ParseError> {
        let tokens = def_scanner().scan(input, f)?;
        let parse = def_parser().parse(tokens, &SPEC_GRAMMAR)?;
        Ok(parse)
    })
}

#[derive(Debug)]
pub enum ParseError {
    ScanErr(scan::Error),
    ParseErr(parse::Error)
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            ParseError::ScanErr(ref err) => write!(f, "Scan error: {}", err),
            ParseError::ParseErr(ref err) => write!(f, "Parse error: {}", err),
        }
    }
}

impl error::Error for ParseError {
    fn cause(&self) -> Option<&error::Error> {
        match *self {
            ParseError::ScanErr(ref err) => Some(err),
            ParseError::ParseErr(ref err) => Some(err),
        }
    }
}

impl From<scan::Error> for ParseError {
    fn from(err: scan::Error) -> ParseError {
        ParseError::ScanErr(err)
    }
}

impl From<parse::Error> for ParseError {
    fn from(err: parse::Error) -> ParseError {
        ParseError::ParseErr(err)
    }
}

fn replace_escapes(input: &str) -> String {
    let mut res = String::with_capacity(input.as_bytes().len());
    let mut i = 0;
    let mut last_char: char = ' ';
    for c in input.chars() {
        let mut hit_double_slash = false;
        if i != 0 && last_char == '\\' {
            res.push(match c {
                'n' => '\n',
                't' => '\t',
                '\'' => '\'',
                '\\' => {
                    last_char = ' '; //Stop \\\\ -> \\\ rather than \\
                    hit_double_slash = true;
                    '\\'
                },
                _ => c,
            });
        } else if c != '\\' {
            res.push(c);
        }
        if !hit_double_slash {
            last_char = c;
        }
        i += 1;
    }
    res
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
    │           │   └── targets
    │           │       └── ID <- 'start'
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
    │       │   │   │       │   └── targets
    │       │   │   │       │       └── ID <- 'start'
    │       │   │   │       ├── transopt
    │       │   │   │       │   └── trans
    │       │   │   │       │       ├── trans
    │       │   │   │       │       │   ├── trans
    │       │   │   │       │       │   │   ├── trans
    │       │   │   │       │       │   │   │   ├── trans
    │       │   │   │       │       │   │   │   │   └── tran
    │       │   │   │       │       │   │   │   │       ├── mtcs
    │       │   │   │       │       │   │   │   │       │   └── CILC <- '' ''
    │       │   │   │       │       │   │   │   │       ├── ARROW <- '->'
    │       │   │   │       │       │   │   │   │       └── trand
    │       │   │   │       │       │   │   │   │           └── ID <- 'ws'
    │       │   │   │       │       │   │   │   └── tran
    │       │   │   │       │       │   │   │       ├── mtcs
    │       │   │   │       │       │   │   │       │   └── CILC <- ''\\t''
    │       │   │   │       │       │   │   │       ├── ARROW <- '->'
    │       │   │   │       │       │   │   │       └── trand
    │       │   │   │       │       │   │   │           └── ID <- 'ws'
    │       │   │   │       │       │   │   └── tran
    │       │   │   │       │       │   │       ├── mtcs
    │       │   │   │       │       │   │       │   └── CILC <- ''\\n''
    │       │   │   │       │       │   │       ├── ARROW <- '->'
    │       │   │   │       │       │   │       └── trand
    │       │   │   │       │       │   │           └── ID <- 'ws'
    │       │   │   │       │       │   └── tran
    │       │   │   │       │       │       ├── mtcs
    │       │   │   │       │       │       │   └── CILC <- ''{''
    │       │   │   │       │       │       ├── ARROW <- '->'
    │       │   │   │       │       │       └── trand
    │       │   │   │       │       │           └── ID <- 'lbr'
    │       │   │   │       │       └── tran
    │       │   │   │       │           ├── mtcs
    │       │   │   │       │           │   └── CILC <- ''}''
    │       │   │   │       │           ├── ARROW <- '->'
    │       │   │   │       │           └── trand
    │       │   │   │       │               └── ID <- 'rbr'
    │       │   │   │       └── SEMI <- ';'
    │       │   │   └── state
    │       │   │       ├── sdec
    │       │   │       │   ├── targets
    │       │   │       │   │   └── ID <- 'ws'
    │       │   │       │   ├── HAT <- '^'
    │       │   │       │   └── ID <- 'WHITESPACE'
    │       │   │       ├── transopt
    │       │   │       │   └── trans
    │       │   │       │       ├── trans
    │       │   │       │       │   ├── trans
    │       │   │       │       │   │   └── tran
    │       │   │       │       │   │       ├── mtcs
    │       │   │       │       │   │       │   └── CILC <- '' ''
    │       │   │       │       │   │       ├── ARROW <- '->'
    │       │   │       │       │   │       └── trand
    │       │   │       │       │   │           └── ID <- 'ws'
    │       │   │       │       │   └── tran
    │       │   │       │       │       ├── mtcs
    │       │   │       │       │       │   └── CILC <- ''\\t''
    │       │   │       │       │       ├── ARROW <- '->'
    │       │   │       │       │       └── trand
    │       │   │       │       │           └── ID <- 'ws'
    │       │   │       │       └── tran
    │       │   │       │           ├── mtcs
    │       │   │       │           │   └── CILC <- ''\\n''
    │       │   │       │           ├── ARROW <- '->'
    │       │   │       │           └── trand
    │       │   │       │               └── ID <- 'ws'
    │       │   │       └── SEMI <- ';'
    │       │   └── state
    │       │       ├── sdec
    │       │       │   ├── targets
    │       │       │   │   └── ID <- 'lbr'
    │       │       │   ├── HAT <- '^'
    │       │       │   └── ID <- 'LBRACKET'
    │       │       ├── transopt
    │       │       │   └──  <- 'NULL'
    │       │       └── SEMI <- ';'
    │       └── state
    │           ├── sdec
    │           │   ├── targets
    │           │   │   └── ID <- 'rbr'
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
start ' ' | '\\t' | '\\n' -> ws
 '{' -> lbr
 '}' -> rbr;
ws^WHITESPACE
 ' ' | '\\t' | '\\n' -> ws;
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
        let (dfa, grammar, formatter) = generate_spec(&parse).unwrap();

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

    #[test]
    fn multi_character_lexing() {
        //setup
        let spec = "
'abcdefghijklmnopqrstuvwxyz '

start
  'if' -> ^IF
  'else' -> ^ELSE
  'for' -> ^FOR
  'fob' -> ^FOB
  'final' -> ^FINAL
  ' ' -> ^_
  _ -> id;

id ^ID
 ' ' -> fail
 _ -> id;

s -> ;
    ";

        let input = "fdkgdfjgdjglkdjglkdjgljbnhbduhoifjeoigjeoghknhkjdfjgoirjt for if endif \
        elseif somethign eldsfnj hi bob joe here final for fob else if id idhere fobre f ";

        let scanner = def_scanner();
        let tree = parse_spec(spec);
        let parse = tree.unwrap();
        let (dfa, _, _) = generate_spec(&parse).unwrap();

        //execute
        let tokens = scanner.scan(input, &dfa).unwrap();

        //verify
        let mut res_string = String::new();
        for token in tokens {
            res_string = format!("{}\nkind={} lexeme={}", res_string, token.kind, token.lexeme);
        }

        assert_eq!(res_string, "
kind=ID lexeme=fdkgdfjgdjglkdjglkdjgljbnhbduhoifjeoigjeoghknhkjdfjgoirjt
kind=FOR lexeme=for
kind=IF lexeme=if
kind=ID lexeme=endif
kind=ELSE lexeme=else
kind=IF lexeme=if
kind=ID lexeme=somethign
kind=ID lexeme=eldsfnj
kind=ID lexeme=hi
kind=ID lexeme=bob
kind=ID lexeme=joe
kind=ID lexeme=here
kind=FINAL lexeme=final
kind=FOR lexeme=for
kind=FOB lexeme=fob
kind=ELSE lexeme=else
kind=IF lexeme=if
kind=ID lexeme=id
kind=ID lexeme=idhere
kind=FOB lexeme=fob
kind=ID lexeme=re
kind=ID lexeme=f")
    }

    #[test]
    fn parse_spec_olap_trans() {
        //setup
        let input = "
        'inj '
        start
            'i' -> ki
            _ -> ^ID;
        ki
            'n' -> ^IN;
        ID | ki
            ' ' -> fail
            _ -> ID;
        s
            -> ID s
            -> ID;";

        //execute
        let tree = parse_spec(input);

        //verify
        assert_eq!(tree.unwrap().to_string(),
"└── spec
    ├── dfa
    │   ├── CILC <- ''inj ''
    │   └── states
    │       ├── states
    │       │   ├── states
    │       │   │   └── state
    │       │   │       ├── sdec
    │       │   │       │   └── targets
    │       │   │       │       └── ID <- 'start'
    │       │   │       ├── transopt
    │       │   │       │   └── trans
    │       │   │       │       ├── trans
    │       │   │       │       │   └── tran
    │       │   │       │       │       ├── mtcs
    │       │   │       │       │       │   └── CILC <- ''i''
    │       │   │       │       │       ├── ARROW <- '->'
    │       │   │       │       │       └── trand
    │       │   │       │       │           └── ID <- 'ki'
    │       │   │       │       └── tran
    │       │   │       │           ├── DEF <- '_'
    │       │   │       │           ├── ARROW <- '->'
    │       │   │       │           └── trand
    │       │   │       │               ├── HAT <- '^'
    │       │   │       │               └── ID <- 'ID'
    │       │   │       └── SEMI <- ';'
    │       │   └── state
    │       │       ├── sdec
    │       │       │   └── targets
    │       │       │       └── ID <- 'ki'
    │       │       ├── transopt
    │       │       │   └── trans
    │       │       │       └── tran
    │       │       │           ├── mtcs
    │       │       │           │   └── CILC <- ''n''
    │       │       │           ├── ARROW <- '->'
    │       │       │           └── trand
    │       │       │               ├── HAT <- '^'
    │       │       │               └── ID <- 'IN'
    │       │       └── SEMI <- ';'
    │       └── state
    │           ├── sdec
    │           │   └── targets
    │           │       ├── ID <- 'ID'
    │           │       ├── OR <- '|'
    │           │       └── targets
    │           │           └── ID <- 'ki'
    │           ├── transopt
    │           │   └── trans
    │           │       ├── trans
    │           │       │   └── tran
    │           │       │       ├── mtcs
    │           │       │       │   └── CILC <- '' ''
    │           │       │       ├── ARROW <- '->'
    │           │       │       └── trand
    │           │       │           └── ID <- 'fail'
    │           │       └── tran
    │           │           ├── DEF <- '_'
    │           │           ├── ARROW <- '->'
    │           │           └── trand
    │           │               └── ID <- 'ID'
    │           └── SEMI <- ';'
    └── gram
        └── prods
            └── prod
                ├── ID <- 's'
                ├── rhss
                │   ├── rhss
                │   │   └── rhs
                │   │       ├── ARROW <- '->'
                │   │       ├── ids
                │   │       │   ├── ID <- 'ID'
                │   │       │   └── ids
                │   │       │       ├── ID <- 's'
                │   │       │       └── ids
                │   │       │           └──  <- 'NULL'
                │   │       └── pattopt
                │   │           └──  <- 'NULL'
                │   └── rhs
                │       ├── ARROW <- '->'
                │       ├── ids
                │       │   ├── ID <- 'ID'
                │       │   └── ids
                │       │       └──  <- 'NULL'
                │       └── pattopt
                │           └──  <- 'NULL'
                └── SEMI <- ';'"
        );
    }

    #[test]
    fn test_replace_escapes() {
        //setup
        let input = "ffffnt\'ff\\n\\t\\\\\\\\ffff\\ff\'\\f\\\'fff";

        //execute
        let res = replace_escapes(input);

        //verify
        assert_eq!(res, "ffffnt\'ff\n\t\\\\ffffff\'f\'fff");
    }
}