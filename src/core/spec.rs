use std;
use std::error;

use core::fmt;
use core::fmt::PatternPair;
use core::fmt::Formatter;
use core::parse;
use core::parse::Grammar;
use core::parse::Production;
use core::parse::Tree;
use core::scan;
use core::scan::State;
use core::scan::compile;
use core::scan::compile::DFA;
use core::scan::compile::CompileTransitionDelta;
use core::scan::runtime;
use core::scan::runtime::CDFABuilder;
use core::scan::runtime::ecdfa::EncodedCDFA;
use core::scan::runtime::ecdfa::EncodedCDFABuilder;

static SPEC_ALPHABET: &'static str = "`-=~!@#$%^&*()_+{}|[]\\;':\"<>?,./QWERTYUIOPASDFGHJKLZXCVBNM1234567890abcdefghijklmnopqrstuvwxyz \n\t";
pub static DEF_MATCHER: char = '_';

#[derive(PartialEq, Clone)]
enum S {
    START,
    HAT,
    MINUS,
    BSLASH,
    PATT,
    PATTC,
    CIL,
    CILC,
    CILBS,
    COMMENT,
    SEMI,
    DEF,
    OR,
    WS,
    ID,
    ARROW,
    FAIL,
}

thread_local! {
    static SPEC_DFA: DFA<S> = {
        let delta: fn(S, char) -> S = |state, c| match (state, c) {
            (S::START, '^') => S::HAT,
            (S::START, '-') => S::MINUS,
            (S::START, '\\') => S::BSLASH,
            (S::START, '`') => S::PATT,
            (S::START, '\'') => S::CIL,
            (S::START, '#') => S::COMMENT,
            (S::START, ';') => S::SEMI,
            (S::START, '_') => S::DEF,
            (S::START, '|') => S::OR,
            (S::START, ' ') | (S::START, '\t') | (S::START, '\n') => S::WS,
            (S::START, '0') | (S::START, '1') | (S::START, '2') | (S::START, '3') | (S::START, '4') |
            (S::START, '5') | (S::START, '6') | (S::START, '7') | (S::START, '8') | (S::START, '9') |
            (S::START, 'a') | (S::START, 'g') | (S::START, 'l') | (S::START, 'q') | (S::START, 'v') |
            (S::START, 'b') | (S::START, 'h') | (S::START, 'm') | (S::START, 'r') | (S::START, 'w') |
            (S::START, 'c') | (S::START, 'i') | (S::START, 'n') | (S::START, 's') | (S::START, 'x') |
            (S::START, 'd') | (S::START, 'j') | (S::START, 'o') | (S::START, 't') | (S::START, 'y') |
            (S::START, 'e') | (S::START, 'k') | (S::START, 'p') | (S::START, 'u') | (S::START, 'z') |
            (S::START, 'f') | (S::START, 'A') | (S::START, 'G') | (S::START, 'L') | (S::START, 'Q') |
            (S::START, 'V') | (S::START, 'B') | (S::START, 'H') | (S::START, 'M') | (S::START, 'R') |
            (S::START, 'W') | (S::START, 'C') | (S::START, 'I') | (S::START, 'N') | (S::START, 'S') |
            (S::START, 'X') | (S::START, 'D') | (S::START, 'J') | (S::START, 'O') | (S::START, 'T') |
            (S::START, 'Y') | (S::START, 'E') | (S::START, 'K') | (S::START, 'P') | (S::START, 'U') |
            (S::START, 'Z') | (S::START, 'F') => S::ID,

            (S::MINUS, '>') => S::ARROW,

            (S::ID, '0') | (S::ID, '1') | (S::ID, '2') | (S::ID, '3') | (S::ID, '4') |
            (S::ID, '5') | (S::ID, '6') | (S::ID, '7') | (S::ID, '8') | (S::ID, '9') |
            (S::ID, 'a') | (S::ID, 'g') | (S::ID, 'l') | (S::ID, 'q') | (S::ID, 'v') |
            (S::ID, 'b') | (S::ID, 'h') | (S::ID, 'm') | (S::ID, 'r') | (S::ID, 'w') |
            (S::ID, 'c') | (S::ID, 'i') | (S::ID, 'n') | (S::ID, 's') | (S::ID, 'x') |
            (S::ID, 'd') | (S::ID, 'j') | (S::ID, 'o') | (S::ID, 't') | (S::ID, 'y') |
            (S::ID, 'e') | (S::ID, 'k') | (S::ID, 'p') | (S::ID, 'u') | (S::ID, 'z') |
            (S::ID, 'f') | (S::ID, 'A') | (S::ID, 'G') | (S::ID, 'L') | (S::ID, 'Q') |
            (S::ID, 'V') | (S::ID, 'B') | (S::ID, 'H') | (S::ID, 'M') | (S::ID, 'R') |
            (S::ID, 'W') | (S::ID, 'C') | (S::ID, 'I') | (S::ID, 'N') | (S::ID, 'S') |
            (S::ID, 'X') | (S::ID, 'D') | (S::ID, 'J') | (S::ID, 'O') | (S::ID, 'T') |
            (S::ID, 'Y') | (S::ID, 'E') | (S::ID, 'K') | (S::ID, 'P') | (S::ID, 'U') |
            (S::ID, 'Z') | (S::ID, 'F') | (S::ID, '_') => S::ID,

            (S::WS, ' ') | (S::WS, '\t') | (S::WS, '\n') => S::WS,

            (S::PATT, '`') => S::PATTC,
            (S::PATT, _) => S::PATT,

            (S::CIL, '\'') => S::CILC,
            (S::CIL, '\\') => S::CILBS,
            (S::CIL, _) => S::CIL,

            (S::CILBS, _) => S::CIL,

            (S::COMMENT, '\n') => S::FAIL,
            (S::COMMENT, _) => S::COMMENT,

            (_, _) => S::FAIL,
        };
        let tokenizer: fn(S) -> String = |state| match state {
            S::HAT => "HAT",
            S::ARROW => "ARROW",
            S::PATTC => "PATTC",
            S::CILC => "CILC",
            S::COMMENT => "_",
            S::WS => "_",
            S::ID => "ID",
            S::DEF => "DEF",
            S::SEMI => "SEMI",
            S::OR => "OR",
            _ => "",
        }.to_string();

        DFA{
            alphabet: SPEC_ALPHABET.to_string(),
            start: S::START,
            td: Box::new(CompileTransitionDelta::build(delta, tokenizer, S::FAIL)),
        }
    };
}

lazy_static! {
    static ref SPEC_PRODUCTIONS: Vec<Production> = parse::build_prods(&[
            "spec dfa gram",

            "dfa CILC states",

            "states states state",
            "states state",

            "state sdec transopt SEMI",

            "sdec targets",
            "sdec targets HAT ID",
            "sdec targets HAT DEF",

            "targets ID",
            "targets targets OR ID",

            "transopt trans",
            "transopt ",

            "trans trans tran",
            "trans tran",

            "tran mtcs ARROW trand",
            "tran DEF ARROW trand",

            "trand ID",
            "trand HAT ID",
            "trand HAT DEF",

            "mtcs mtcs OR CILC",
            "mtcs CILC",

            "gram prods",

            "prods prods prod",
            "prods prod",

            "prod ID rhss SEMI",

            "rhss rhss rhs",
            "rhss rhs",

            "rhs ARROW ids pattopt",

            "pattopt PATTC",
            "pattopt ",

            "ids ids ID",
            "ids ",
        ]);

    static ref SPEC_GRAMMAR: Grammar = Grammar::from(SPEC_PRODUCTIONS.clone());
}

pub fn generate_spec(parse: &Tree) -> Result<(EncodedCDFA, Grammar, Formatter), GenError> {
    let ecdfa = generate_ecdfa(parse.get_child(0))?;
    let (grammar, pattern_pairs) = generate_grammar(parse.get_child(1));
    let formatter = Formatter::create(pattern_pairs)?;
    Ok((ecdfa, grammar, formatter))
}

#[derive(Debug)]
pub enum GenError {
    CDFAError(runtime::CDFAError),
    PatternErr(fmt::BuildError),
}

impl std::fmt::Display for GenError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            GenError::CDFAError(ref err) => write!(f, "ECDFA generation error: {}", err),
            GenError::PatternErr(ref err) => write!(f, "Pattern build error: {}", err),
        }
    }
}

impl error::Error for GenError {
    fn cause(&self) -> Option<&error::Error> {
        match *self {
            GenError::CDFAError(ref err) => Some(err),
            GenError::PatternErr(ref err) => Some(err),
        }
    }
}

impl From<runtime::CDFAError> for GenError {
    fn from(err: runtime::CDFAError) -> GenError {
        GenError::CDFAError(err)
    }
}

impl From<fmt::BuildError> for GenError {
    fn from(err: fmt::BuildError) -> GenError {
        GenError::PatternErr(err)
    }
}

fn generate_ecdfa(tree: &Tree) -> Result<EncodedCDFA, runtime::CDFAError> {
    let mut builder = EncodedCDFABuilder::new();

    generate_ecdfa_alphabet(tree, &mut builder);

    generate_ecdfa_states(tree.get_child(1), &mut builder)?;

    EncodedCDFA::build_from(builder)
}

fn generate_ecdfa_alphabet(tree: &Tree, builder: &mut EncodedCDFABuilder) {
    let alphabet_string = tree.get_child(0).lhs.lexeme.trim_matches('\'');
    let alphabet = replace_escapes(&alphabet_string);

    builder.set_alphabet(alphabet.chars());
}

fn generate_ecdfa_states<'a>(states_node: &Tree, builder: &mut EncodedCDFABuilder) -> Result<(), runtime::CDFAError> {
    let state_node = states_node.get_child(states_node.children.len() - 1);

    let sdec_node = state_node.get_child(0);

    let targets_node = sdec_node.get_child(0);
    let head_state = &targets_node.get_child(targets_node.children.len() - 1).lhs.lexeme;

    let mut states: Vec<&State> = vec![head_state];
    if targets_node.children.len() == 3 {
        generate_ecdfa_targets(targets_node.get_child(0), &mut states);
    }

    if sdec_node.children.len() == 3 {
        let end = &sdec_node.get_child(2).lhs.lexeme;

        for state in &states {
            add_ecdfa_tokenizer(*state, end, builder);
        }
    }

    let transopt_node = state_node.get_child(1);
    if !transopt_node.is_empty() {
        generate_ecdfa_trans(transopt_node.get_child(0), &states, builder)?;
    }

    if states_node.children.len() == 2 {
        generate_ecdfa_states(states_node.get_child(0), builder)
    } else {
        builder.mark_start(head_state);
        Ok(())
    }
}

fn generate_ecdfa_targets<'a>(targets_node: &'a Tree, accumulator: &mut Vec<&'a State>) {
    accumulator.push(&targets_node.get_child(targets_node.children.len() - 1).lhs.lexeme);
    if targets_node.children.len() == 3 {
        generate_ecdfa_targets(targets_node.get_child(0), accumulator);
    }
}

fn generate_ecdfa_trans<'a>(trans_node: &'a Tree, sources: &Vec<&State>, builder: &mut EncodedCDFABuilder) -> Result<(), runtime::CDFAError> {
    let tran_node = trans_node.get_child(trans_node.children.len() - 1);

    let trand_node = tran_node.get_child(2);
    let dest: &State = &trand_node.get_child(trand_node.children.len() - 1).lhs.lexeme;

    let matcher = tran_node.get_child(0);
    match &matcher.lhs.kind[..] {
        "mtcs" => {
            generate_ecdfa_mtcs(matcher, sources, dest, builder)?;
        }
        "DEF" => for source in sources {
            builder.mark_def(source, dest)?;
        },
        &_ => panic!("Transition map input is neither CILC or DEF"),
    }

    if trand_node.children.len() == 2 { //Immediate state pass-through
        add_ecdfa_tokenizer(dest, dest, builder);
    }

    if trans_node.children.len() == 2 {
        generate_ecdfa_trans(trans_node.get_child(0), sources, builder)
    } else {
        Ok(())
    }
}

fn generate_ecdfa_mtcs<'a>(mtcs_node: &'a Tree, sources: &Vec<&State>, dest: &State, builder: &mut EncodedCDFABuilder) -> Result<(), runtime::CDFAError> {
    let matcher = mtcs_node.children.last().unwrap();
    let matcher_string: String = matcher.lhs.lexeme.chars()
        .skip(1)
        .take(matcher.lhs.lexeme.len() - 2)
        .collect();
    let matcher_cleaned = replace_escapes(&matcher_string);
    if matcher_cleaned.len() == 1 {
        for source in sources {
            builder.mark_trans(source, dest, matcher_cleaned.chars().next().unwrap())?;
        }
    } else {
        for source in sources {
            builder.mark_chain(source, dest, matcher_cleaned.chars())?;
        }
    }

    if mtcs_node.children.len() == 3 {
        generate_ecdfa_mtcs(mtcs_node.get_child(0), sources, dest, builder)
    } else {
        Ok(())
    }
}

fn add_ecdfa_tokenizer(state: &State, kind: &String, builder: &mut EncodedCDFABuilder) {
    builder.mark_accepting(state);
    if kind != "_" { //TODO
        builder.mark_token(state, kind);
    }
}

fn generate_grammar(tree: &Tree) -> (Grammar, Vec<PatternPair>) {
    let mut productions: Vec<Production> = vec![];
    let mut pattern_pairs: Vec<PatternPair> = vec![];
    generate_grammar_prods(tree.get_child(0), &mut productions, &mut pattern_pairs);

    (Grammar::from(productions), pattern_pairs)
}

fn generate_grammar_prods<'a, 'b>(prods_node: &'a Tree, accumulator: &'b mut Vec<Production>, pp_accumulator: &'b mut Vec<PatternPair>){
    if prods_node.children.len() == 2 {
        generate_grammar_prods(prods_node.get_child(0), accumulator, pp_accumulator);
    }

    let prod_node = prods_node.get_child(prods_node.children.len() - 1);

    let id = &prod_node.get_child(0).lhs.lexeme;

    generate_grammar_rhss(prod_node.get_child(1), id, accumulator, pp_accumulator);
}

fn generate_grammar_rhss<'a, 'b>(rhss_node: &'a Tree, lhs: &'a String, accumulator: &'b mut Vec<Production>, pp_accumulator: &'b mut Vec<PatternPair>) {
    let rhs_node = rhss_node.get_child(rhss_node.children.len() - 1);

    let mut ids: Vec<String> = vec![];
    generate_grammar_ids(rhs_node.get_child(1), &mut ids);

    let production = Production {
        lhs: lhs.clone(),
        rhs: ids,
    };

    accumulator.push(production);

    let pattopt_node = rhs_node.get_child(2);
    if !pattopt_node.is_empty() {
        let pattc = &pattopt_node.get_child(0).lhs.lexeme;
        let pattern_string = &pattc[..].trim_matches('`');
        let pattern = replace_escapes(pattern_string);

        pp_accumulator.push(PatternPair {
            production: accumulator.last().unwrap().clone(),
            pattern,
        });
    }

    if rhss_node.children.len() == 2 {
        generate_grammar_rhss(rhss_node.get_child(0), lhs, accumulator, pp_accumulator);
    }
}

fn generate_grammar_ids<'a, 'b>(ids_node: &'a Tree, accumulator: &'b mut Vec<String>) {
    if !ids_node.is_empty() {
        let id = ids_node.get_child(1).lhs.lexeme.clone();

        generate_grammar_ids(ids_node.get_child(0), accumulator);

        accumulator.push(id);
    }
}

pub fn parse_spec(input: &str) -> Result<Tree, ParseError> {
    SPEC_DFA.with(|f| -> Result<Tree, ParseError> {
        let tokens = compile::def_scanner().scan(input, f)?;
        let parse = parse::def_parser().parse(tokens, &SPEC_GRAMMAR)?;
        Ok(parse)
    })
}

#[derive(Debug)]
pub enum ParseError {
    ScanErr(scan::Error),
    ParseErr(parse::Error),
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
                }
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
    use core::data::Data;
    use core::data::stream::StreamSource;

    #[test]
    fn parse_spec_spaces() {
        //setup
        let input = "' 'start;s->s b;";

        //exercise
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
                │       │   ├── ids
                │       │   │   ├── ids
                │       │   │   │   └──  <- 'NULL'
                │       │   │   └── ID <- 's'
                │       │   └── ID <- 'b'
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

        //exercise
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
            ├── prods
            │   ├── prods
            │   │   └── prod
            │   │       ├── ID <- 's'
            │   │       ├── rhss
            │   │       │   ├── rhss
            │   │       │   │   └── rhs
            │   │       │   │       ├── ARROW <- '->'
            │   │       │   │       ├── ids
            │   │       │   │       │   ├── ids
            │   │       │   │       │   │   ├── ids
            │   │       │   │       │   │   │   └──  <- 'NULL'
            │   │       │   │       │   │   └── ID <- 's'
            │   │       │   │       │   └── ID <- 'b'
            │   │       │   │       └── pattopt
            │   │       │   │           └──  <- 'NULL'
            │   │       │   └── rhs
            │   │       │       ├── ARROW <- '->'
            │   │       │       ├── ids
            │   │       │       │   └──  <- 'NULL'
            │   │       │       └── pattopt
            │   │       │           └──  <- 'NULL'
            │   │       └── SEMI <- ';'
            │   └── prod
            │       ├── ID <- 'b'
            │       ├── rhss
            │       │   ├── rhss
            │       │   │   └── rhs
            │       │   │       ├── ARROW <- '->'
            │       │   │       ├── ids
            │       │   │       │   ├── ids
            │       │   │       │   │   ├── ids
            │       │   │       │   │   │   ├── ids
            │       │   │       │   │   │   │   └──  <- 'NULL'
            │       │   │       │   │   │   └── ID <- 'LBRACKET'
            │       │   │       │   │   └── ID <- 's'
            │       │   │       │   └── ID <- 'RBRACKET'
            │       │   │       └── pattopt
            │       │   │           └── PATTC <- '``'
            │       │   └── rhs
            │       │       ├── ARROW <- '->'
            │       │       ├── ids
            │       │       │   ├── ids
            │       │       │   │   └──  <- 'NULL'
            │       │       │   └── ID <- 'w'
            │       │       └── pattopt
            │       │           └──  <- 'NULL'
            │       └── SEMI <- ';'
            └── prod
                ├── ID <- 'w'
                ├── rhss
                │   └── rhs
                │       ├── ARROW <- '->'
                │       ├── ids
                │       │   ├── ids
                │       │   │   └──  <- 'NULL'
                │       │   └── ID <- 'WHITESPACE'
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

        let input = "  {  {  {{{\t}}}\n {} }  }   { {}\n } ".to_string();
        let mut iter = input.chars();
        let mut getter = || {
            iter.next()
        };
        let mut stream: StreamSource<char> = StreamSource::observe(&mut getter);

        let scanner = runtime::def_scanner();
        let parser = parse::def_parser();

        //specification
        let tree = parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, grammar, formatter) = generate_spec(&parse).unwrap();

        //input
        let tokens = scanner.scan(&mut stream, &cdfa);
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
    fn generate_spec_advanced_operators() {
        //setup
        let spec = "
        'inj '

        start
            'in' -> ^IN
            ' ' -> ^_
            _ -> ^ID;

        ID | IN
            ' ' -> fail
            _ -> ID;

        s ->;";

        let input = "i ij ijjjijijiji inj in iii".to_string();
        let mut iter = input.chars();
        let mut getter = || {
            iter.next()
        };
        let mut stream: StreamSource<char> = StreamSource::observe(&mut getter);

        let scanner = runtime::def_scanner();

        let tree = parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, _, _) = generate_spec(&parse).unwrap();

        //exercise
        let tokens = scanner.scan(&mut stream, &cdfa).unwrap();

        let mut result = String::new();
        for token in tokens {
            result.push_str(&token.to_string());
            result.push('\n');
        }

        //verify
        assert_eq!(result, "\
ID <- 'i'
ID <- 'ij'
ID <- 'ijjjijijiji'
ID <- 'inj'
IN <- 'in'
ID <- 'iii'
");
    }

    #[test]
    fn default_matcher_conflict() {
        //setup
        let spec = "
' c'

start
    ' ' -> ^WS
    'c' -> id;

id      ^ID
    'c' | '_' -> id;

# grammar
s ->;
        ";

        let input = "c c".to_string();
        let mut iter = input.chars();
        let mut getter = || {
            iter.next()
        };
        let mut stream: StreamSource<char> = StreamSource::observe(&mut getter);

        let scanner = runtime::def_scanner();

        let tree = parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, _, _) = generate_spec(&parse).unwrap();

        //execute
        let tokens = scanner.scan(&mut stream, &cdfa).unwrap();

        //verify
        let mut res_string = String::new();
        for token in tokens {
            res_string = format!("{}\nkind={} lexeme={}", res_string, token.kind, token.lexeme);
        }

        assert_eq!(res_string, "\nkind=ID lexeme=c\nkind=WS lexeme= \nkind=ID lexeme=c")
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

s -> ;";

        let input = "fdkgdfjgdjglkdjglkdjgljbnhbduhoifjeoigjeoghknhkjdfjgoirjt for if endif \
        elseif somethign eldsfnj hi bob joe here final for fob else if id idhere fobre f ".to_string();
        let mut iter = input.chars();
        let mut getter = || {
            iter.next()
        };
        let mut stream: StreamSource<char> = StreamSource::observe(&mut getter);

        let scanner = runtime::def_scanner();
        let tree = parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, _, _) = generate_spec(&parse).unwrap();

        //exercise
        let tokens = scanner.scan(&mut stream, &cdfa).unwrap();

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

        //exercise
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
    │           │       ├── targets
    │           │       │   └── ID <- 'ID'
    │           │       ├── OR <- '|'
    │           │       └── ID <- 'ki'
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
                │   │       │   ├── ids
                │   │       │   │   ├── ids
                │   │       │   │   │   └──  <- 'NULL'
                │   │       │   │   └── ID <- 'ID'
                │   │       │   └── ID <- 's'
                │   │       └── pattopt
                │   │           └──  <- 'NULL'
                │   └── rhs
                │       ├── ARROW <- '->'
                │       ├── ids
                │       │   ├── ids
                │       │   │   └──  <- 'NULL'
                │       │   └── ID <- 'ID'
                │       └── pattopt
                │           └──  <- 'NULL'
                └── SEMI <- ';'"
        );
    }

    #[test]
    fn replace_escapes() {
        //setup
        let input = "ffffnt\'ff\\n\\t\\\\\\\\ffff\\ff\'\\f\\\'fff";

        //exercise
        let res = super::replace_escapes(input);

        //verify
        assert_eq!(res, "ffffnt\'ff\n\t\\\\ffffff\'f\'fff");
    }
}
