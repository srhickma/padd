use {
    core::{
        data::stream::StreamSource,
        fmt::{self, Formatter, FormatterBuilder, PatternPair},
        parse::{
            self,
            grammar::{Grammar, GrammarBuilder},
            Production,
            Tree,
        },
        scan::{
            self,
            CDFA,
            CDFABuilder,
            ecdfa::{EncodedCDFA, EncodedCDFABuilder},
            Kind,
            State,
        },
        util::string_utils,
    },
    std::{
        self,
        collections::HashSet,
        error,
    },
};

static SPEC_ALPHABET: &'static str = "`-=~!@#$%^&*()+{}|[]\\;':\"<>?,./_0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ \n\t";
pub static DEF_MATCHER: &'static str = "_";

#[derive(PartialEq, Eq, Hash, Clone)]
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
    OPTID,
    COPTID,
    ARROW,
    DOT,
    RANGE,
    FAIL,
}

thread_local! {
    static SPEC_ECDFA: EncodedCDFA<String> = build_spec_ecdfa().unwrap();
}

lazy_static! {
    static ref SPEC_GRAMMAR: Grammar = build_spec_grammar();
}

fn build_spec_ecdfa() -> Result<EncodedCDFA<String>, scan::CDFAError> {
    let mut builder: EncodedCDFABuilder<S, String> = EncodedCDFABuilder::new();

    builder.set_alphabet(SPEC_ALPHABET.chars());
    builder.mark_start(&S::START);

    builder.state(&S::START)
        .mark_trans(&S::HAT, '^')?
        .mark_trans(&S::MINUS, '-')?
        .mark_trans(&S::BSLASH, '\\')?
        .mark_trans(&S::PATT, '`')?
        .mark_trans(&S::CIL, '\'')?
        .mark_trans(&S::COMMENT, '#')?
        .mark_trans(&S::SEMI, ';')?
        .mark_trans(&S::DEF, '_')?
        .mark_trans(&S::OR, '|')?
        .mark_trans(&S::OPTID, '[')?
        .mark_trans(&S::DOT, '.')?
        .mark_trans(&S::WS, ' ')?
        .mark_trans(&S::WS, '\t')?
        .mark_trans(&S::WS, '\n')?
        .mark_range(&S::ID, '0', 'Z')?;

    builder.mark_trans(&S::MINUS, &S::ARROW, '>')?;

    builder.state(&S::OPTID)
        .mark_trans(&S::COPTID, ']')?
        .mark_range(&S::OPTID, '_', 'Z')?;

    builder.state(&S::ID)
        .mark_range(&S::ID, '_', 'Z')?
        .mark_token(&"ID".to_string());

    builder.state(&S::WS)
        .mark_trans(&S::WS, ' ')?
        .mark_trans(&S::WS, '\t')?
        .mark_trans(&S::WS, '\n')?
        .mark_accepting();

    builder.state(&S::PATT)
        .mark_trans(&S::PATTC, '`')?
        .mark_def(&S::PATT)?;

    builder.state(&S::CIL)
        .mark_trans(&S::CILC, '\'')?
        .mark_trans(&S::CILBS, '\\')?
        .mark_def(&S::CIL)?;

    builder.mark_def(&S::CILBS, &S::CIL)?;

    builder.mark_trans(&S::DOT, &S::RANGE, '.')?;

    builder.state(&S::COMMENT)
        .mark_trans(&S::FAIL, '\n')?
        .mark_def(&S::COMMENT)?
        .mark_accepting();

    builder
        .mark_token(&S::HAT, &"HAT".to_string())
        .mark_token(&S::ARROW, &"ARROW".to_string())
        .mark_token(&S::PATTC, &"PATTC".to_string())
        .mark_token(&S::CILC, &"CILC".to_string())
        .mark_token(&S::CILC, &"CILC".to_string())
        .mark_token(&S::COPTID, &"COPTID".to_string())
        .mark_token(&S::DEF, &"DEF".to_string())
        .mark_token(&S::SEMI, &"SEMI".to_string())
        .mark_token(&S::OR, &"OR".to_string())
        .mark_token(&S::RANGE, &"RANGE".to_string());

    builder.build()
}

fn build_spec_grammar() -> Grammar {
    let productions = parse::build_prods(&[
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
        "mtcs mtcs OR mtc",
        "mtcs mtc",
        "mtc CILC",
        "mtc CILC RANGE CILC",
        "gram prods",
        "prods prods prod",
        "prods prod",
        "prod ID pattopt rhss SEMI",
        "rhss rhss rhs",
        "rhss rhs",
        "rhs ARROW ids pattopt",
        "pattopt PATTC",
        "pattopt ",
        "ids ids ID",
        "ids ids COPTID",
        "ids ",
    ]);

    let mut builder = GrammarBuilder::new();
    builder.try_mark_start(&productions.first().unwrap().lhs);
    builder.add_productions(productions.clone());
    builder.build()
}

pub fn generate_spec(parse: &Tree) -> Result<(EncodedCDFA<Kind>, Grammar, Formatter), GenError> {
    let ecdfa = generate_ecdfa(parse.get_child(0))?;
    let (grammar, formatter) = traverse_grammar(parse.get_child(1))?;

    orphan_check(&ecdfa, &grammar)?;

    Ok((ecdfa, grammar, formatter))
}

#[derive(Debug)]
pub enum GenError {
    MatcherErr(String),
    MappingErr(String),
    CDFAErr(scan::CDFAError),
    PatternErr(fmt::BuildError),
}

impl std::fmt::Display for GenError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            GenError::MatcherErr(ref err) => write!(f, "Matcher definition error: {}", err),
            GenError::MappingErr(ref err) => write!(f, "ECDFA to grammar mapping error: {}", err),
            GenError::CDFAErr(ref err) => write!(f, "ECDFA generation error: {}", err),
            GenError::PatternErr(ref err) => write!(f, "Pattern build error: {}", err),
        }
    }
}

impl error::Error for GenError {
    fn cause(&self) -> Option<&error::Error> {
        match *self {
            GenError::MatcherErr(_) => None,
            GenError::MappingErr(_) => None,
            GenError::CDFAErr(ref err) => Some(err),
            GenError::PatternErr(ref err) => Some(err),
        }
    }
}

impl From<scan::CDFAError> for GenError {
    fn from(err: scan::CDFAError) -> GenError {
        GenError::CDFAErr(err)
    }
}

impl From<fmt::BuildError> for GenError {
    fn from(err: fmt::BuildError) -> GenError {
        GenError::PatternErr(err)
    }
}

fn generate_ecdfa(tree: &Tree) -> Result<EncodedCDFA<Kind>, GenError> {
    let mut builder = EncodedCDFABuilder::new();

    generate_cdfa_alphabet(tree, &mut builder);

    generate_cdfa_states(tree.get_child(1), &mut builder)?;

    Ok(builder.build()?)
}

fn generate_cdfa_alphabet<CDFABuilderType, CDFAType>(
    tree: &Tree,
    builder: &mut CDFABuilderType,
) where
    CDFAType: CDFA<usize, String>,
    CDFABuilderType: CDFABuilder<String, String, CDFAType>
{
    let alphabet_string = tree.get_child(0).lhs.lexeme.trim_matches('\'');
    let alphabet = string_utils::replace_escapes(&alphabet_string);

    builder.set_alphabet(alphabet.chars());
}

fn generate_cdfa_states<CDFABuilderType, CDFAType>(
    states_node: &Tree,
    builder: &mut CDFABuilderType,
) -> Result<(), GenError> where
    CDFAType: CDFA<usize, String>,
    CDFABuilderType: CDFABuilder<String, String, CDFAType>
{
    let state_node = states_node.get_child(states_node.children.len() - 1);

    let sdec_node = state_node.get_child(0);

    let targets_node = sdec_node.get_child(0);
    let head_state = &targets_node.get_child(targets_node.children.len() - 1).lhs.lexeme;

    let mut states: Vec<&State> = vec![head_state];
    if targets_node.children.len() == 3 {
        generate_cdfa_targets(targets_node.get_child(0), &mut states);
    }

    if sdec_node.children.len() == 3 {
        let end = &sdec_node.get_child(2).lhs.lexeme;

        for state in &states {
            add_cdfa_tokenizer(*state, end, builder);
        }
    }

    let transopt_node = state_node.get_child(1);
    if !transopt_node.is_empty() {
        generate_cdfa_trans(transopt_node.get_child(0), &states, builder)?;
    }

    if states_node.children.len() == 2 {
        generate_cdfa_states(states_node.get_child(0), builder)
    } else {
        builder.mark_start(head_state);
        Ok(())
    }
}

fn generate_cdfa_targets<'tree>(
    targets_node: &'tree Tree,
    accumulator: &mut Vec<&'tree State>,
) {
    accumulator.push(&targets_node.get_child(targets_node.children.len() - 1).lhs.lexeme);
    if targets_node.children.len() == 3 {
        generate_cdfa_targets(targets_node.get_child(0), accumulator);
    }
}

fn generate_cdfa_trans<CDFABuilderType, CDFAType>(
    trans_node: &Tree,
    sources: &Vec<&State>,
    builder: &mut CDFABuilderType,
) -> Result<(), GenError> where
    CDFAType: CDFA<usize, String>,
    CDFABuilderType: CDFABuilder<String, String, CDFAType>
{
    let tran_node = trans_node.get_child(trans_node.children.len() - 1);

    let trand_node = tran_node.get_child(2);
    let dest: &State = &trand_node.get_child(trand_node.children.len() - 1).lhs.lexeme;

    let matcher = tran_node.get_child(0);
    match &matcher.lhs.kind[..] {
        "mtcs" => {
            generate_cdfa_mtcs(matcher, sources, dest, builder)?;
        }
        "DEF" => for source in sources {
            builder.mark_def(source, dest)?;
        },
        &_ => panic!("Transition map input is neither CILC or DEF"),
    }

    if trand_node.children.len() == 2 { //Immediate state pass-through
        add_cdfa_tokenizer(dest, dest, builder);
    }

    if trans_node.children.len() == 2 {
        generate_cdfa_trans(trans_node.get_child(0), sources, builder)
    } else {
        Ok(())
    }
}

fn generate_cdfa_mtcs<CDFABuilderType, CDFAType>(
    mtcs_node: &Tree,
    sources: &Vec<&State>,
    dest: &State,
    builder: &mut CDFABuilderType,
) -> Result<(), GenError> where
    CDFAType: CDFA<usize, String>,
    CDFABuilderType: CDFABuilder<String, String, CDFAType>
{
    let mtc_node = mtcs_node.children.last().unwrap();

    if mtc_node.children.len() == 1 {
        let matcher = mtc_node.get_child(0);
        let matcher_string: String = matcher.lhs.lexeme.chars()
            .skip(1)
            .take(matcher.lhs.lexeme.len() - 2)
            .collect();
        let matcher_cleaned = string_utils::replace_escapes(&matcher_string);
        if matcher_cleaned.len() == 1 {
            for source in sources {
                builder.mark_trans(source, dest, matcher_cleaned.chars().next().unwrap())?;
            }
        } else {
            for source in sources {
                builder.mark_chain(source, dest, matcher_cleaned.chars())?;
            }
        }
    } else {
        let range_start_node = mtc_node.get_child(0);
        let range_end_node = mtc_node.get_child(2);

        let escaped_range_start_string: String = range_start_node.lhs.lexeme.chars()
            .skip(1)
            .take(range_start_node.lhs.lexeme.len() - 2)
            .collect();

        let range_start_string = string_utils::replace_escapes(&escaped_range_start_string);
        if range_start_string.len() > 1 {
            return Err(GenError::MatcherErr(format!(
                "Range start must be one character, but was '{}'", range_start_string
            )));
        }

        let escaped_range_end_string: String = range_end_node.lhs.lexeme.chars()
            .skip(1)
            .take(range_end_node.lhs.lexeme.len() - 2)
            .collect();

        let range_end_string: String = string_utils::replace_escapes(&escaped_range_end_string);
        if range_end_string.len() > 1 {
            return Err(GenError::MatcherErr(format!(
                "Range end must be one character, but was '{}'", range_end_string
            )));
        }

        let range_start = range_start_string.chars().next().unwrap();
        let range_end = range_end_string.chars().next().unwrap();

        builder.mark_range_for_all(sources.iter(), dest, range_start, range_end)?;
    }

    if mtcs_node.children.len() == 3 {
        generate_cdfa_mtcs(mtcs_node.get_child(0), sources, dest, builder)
    } else {
        Ok(())
    }
}

fn add_cdfa_tokenizer<CDFABuilderType, CDFAType>(
    state: &State,
    kind: &String,
    builder: &mut CDFABuilderType,
) where
    CDFAType: CDFA<usize, String>,
    CDFABuilderType: CDFABuilder<String, String, CDFAType>
{
    builder.mark_accepting(state);
    if kind != DEF_MATCHER {
        builder.mark_token(state, kind);
    }
}

fn traverse_grammar(tree: &Tree) -> Result<(Grammar, Formatter), GenError> {
    let mut grammar_builder = GrammarBuilder::new();
    let mut formatter_builder = FormatterBuilder::new();

    generate_grammar_prods(tree.get_child(0), &mut grammar_builder, &mut formatter_builder)?;

    Ok((grammar_builder.build(), formatter_builder.build()))
}

fn generate_grammar_prods(
    prods_node: &Tree,
    grammar_builder: &mut GrammarBuilder,
    formatter_builder: &mut FormatterBuilder,
) -> Result<(), GenError> {
    if prods_node.children.len() == 2 {
        generate_grammar_prods(prods_node.get_child(0), grammar_builder, formatter_builder)?;
    }

    let prod_node = prods_node.get_child(prods_node.children.len() - 1);

    let id = &prod_node.get_child(0).lhs.lexeme;

    let def_pattern_node = &prod_node.get_child(1);

    generate_grammar_rhss(
        prod_node.get_child(2),
        id,
        def_pattern_node,
        grammar_builder,
        formatter_builder,
    )
}

fn generate_grammar_rhss(
    rhss_node: &Tree,
    lhs: &String,
    def_pattern_node: &Tree,
    grammar_builder: &mut GrammarBuilder,
    formatter_builder: &mut FormatterBuilder,
) -> Result<(), GenError> {
    let rhs_node = rhss_node.get_child(rhss_node.children.len() - 1);

    let mut ids: Vec<String> = Vec::new();
    generate_grammar_ids(rhs_node.get_child(1), &mut ids, grammar_builder);

    let production = Production {
        lhs: lhs.clone(),
        rhs: ids,
    };

    grammar_builder.try_mark_start(lhs);
    grammar_builder.add_production(production.clone());

    let mut pattopt_node = rhs_node.get_child(2);
    if pattopt_node.is_empty() {
        pattopt_node = def_pattern_node
    }

    if !pattopt_node.is_empty() {
        let pattc = &pattopt_node.get_child(0).lhs.lexeme;
        let pattern_string = &pattc[..].trim_matches('`');
        let pattern = string_utils::replace_escapes(pattern_string);

        formatter_builder.add_pattern(PatternPair {
            production,
            pattern,
        })?;
    }

    if rhss_node.children.len() == 2 {
        generate_grammar_rhss(
            rhss_node.get_child(0),
            lhs,
            def_pattern_node,
            grammar_builder,
            formatter_builder,
        )?;
    }

    Ok(())
}

fn generate_grammar_ids(
    ids_node: &Tree,
    ids_accumulator: &mut Vec<String>,
    grammar_builder: &mut GrammarBuilder,
) {
    if !ids_node.is_empty() {
        generate_grammar_ids(ids_node.get_child(0), ids_accumulator, grammar_builder);

        let id_node = ids_node.get_child(1);
        let id = match &id_node.lhs.kind[..] {
            "ID" => id_node.lhs.lexeme.clone(),
            "COPTID" => {
                let lex = &id_node.lhs.lexeme[..];
                let dest = &lex[1..lex.len() - 1];
                grammar_builder.add_optional_state(dest)
            }
            &_ => panic!("Production identifier is neither an ID or a COPTID")
        };

        ids_accumulator.push(id);
    }
}

fn orphan_check(ecdfa: &EncodedCDFA<Kind>, grammar: &Grammar) -> Result<(), GenError> {
    let mut ecdfa_products: HashSet<&String> = HashSet::new();
    for product in ecdfa.produces() {
        ecdfa_products.insert(product);
    }

    for symbol in grammar.terminals() {
        if !ecdfa_products.contains(symbol) {
            return Err(GenError::MappingErr(format!(
                "Orphaned terminal '{}' is not tokenized by the ECDFA", symbol
            )));
        }
    }

    Ok(())
}

pub fn parse_spec(input: &str) -> Result<Tree, ParseError> {
    SPEC_ECDFA.with(|cdfa| -> Result<Tree, ParseError> {
        let mut iter = input.chars();
        let mut getter = || iter.next();
        let mut source = StreamSource::observe(&mut getter);

        let tokens = scan::def_scanner().scan(&mut source, cdfa)?;
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

#[cfg(test)]
mod tests {
    use core::data::{
        Data,
        stream::StreamSource,
    };

    use super::*;

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
                ├── pattopt
                │   └──  <- 'NULL'
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
        let tree = parse_spec(input).unwrap();

        //verify
        assert_eq!(tree.to_string(),
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
    │       │   │   │       │       │   │   │   │       │   └── mtc
    │       │   │   │       │       │   │   │   │       │       └── CILC <- '' ''
    │       │   │   │       │       │   │   │   │       ├── ARROW <- '->'
    │       │   │   │       │       │   │   │   │       └── trand
    │       │   │   │       │       │   │   │   │           └── ID <- 'ws'
    │       │   │   │       │       │   │   │   └── tran
    │       │   │   │       │       │   │   │       ├── mtcs
    │       │   │   │       │       │   │   │       │   └── mtc
    │       │   │   │       │       │   │   │       │       └── CILC <- ''\\t''
    │       │   │   │       │       │   │   │       ├── ARROW <- '->'
    │       │   │   │       │       │   │   │       └── trand
    │       │   │   │       │       │   │   │           └── ID <- 'ws'
    │       │   │   │       │       │   │   └── tran
    │       │   │   │       │       │   │       ├── mtcs
    │       │   │   │       │       │   │       │   └── mtc
    │       │   │   │       │       │   │       │       └── CILC <- ''\\n''
    │       │   │   │       │       │   │       ├── ARROW <- '->'
    │       │   │   │       │       │   │       └── trand
    │       │   │   │       │       │   │           └── ID <- 'ws'
    │       │   │   │       │       │   └── tran
    │       │   │   │       │       │       ├── mtcs
    │       │   │   │       │       │       │   └── mtc
    │       │   │   │       │       │       │       └── CILC <- ''{''
    │       │   │   │       │       │       ├── ARROW <- '->'
    │       │   │   │       │       │       └── trand
    │       │   │   │       │       │           └── ID <- 'lbr'
    │       │   │   │       │       └── tran
    │       │   │   │       │           ├── mtcs
    │       │   │   │       │           │   └── mtc
    │       │   │   │       │           │       └── CILC <- ''}''
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
    │       │   │       │       │   │       │   └── mtc
    │       │   │       │       │   │       │       └── CILC <- '' ''
    │       │   │       │       │   │       ├── ARROW <- '->'
    │       │   │       │       │   │       └── trand
    │       │   │       │       │   │           └── ID <- 'ws'
    │       │   │       │       │   └── tran
    │       │   │       │       │       ├── mtcs
    │       │   │       │       │       │   └── mtc
    │       │   │       │       │       │       └── CILC <- ''\\t''
    │       │   │       │       │       ├── ARROW <- '->'
    │       │   │       │       │       └── trand
    │       │   │       │       │           └── ID <- 'ws'
    │       │   │       │       └── tran
    │       │   │       │           ├── mtcs
    │       │   │       │           │   └── mtc
    │       │   │       │           │       └── CILC <- ''\\n''
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
            │   │       ├── pattopt
            │   │       │   └──  <- 'NULL'
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
            │       ├── pattopt
            │       │   └──  <- 'NULL'
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
                ├── pattopt
                │   └──  <- 'NULL'
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

        let scanner = scan::def_scanner();
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

        let scanner = scan::def_scanner();

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

s ->;
        ";

        let input = "c c".to_string();
        let mut iter = input.chars();
        let mut getter = || {
            iter.next()
        };
        let mut stream: StreamSource<char> = StreamSource::observe(&mut getter);

        let scanner = scan::def_scanner();

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

        assert_eq!(res_string, "\nkind=ID lexeme=c\nkind=WS lexeme= \nkind=ID lexeme=c")
    }

    #[test]
    fn complex_id() {
        //setup
        let spec = "
' ab_'

start
    ' ' -> ws
    _ -> id;

ws      ^_;

id      ^ID
    'a' | 'b' | '_' -> id;

s
    -> ids
    ->;
ids
    -> ids ID
    -> ID;
        ";

        let input = "a ababab _abab ab_abba_".to_string();
        let mut iter = input.chars();
        let mut getter = || {
            iter.next()
        };
        let mut stream: StreamSource<char> = StreamSource::observe(&mut getter);

        let scanner = scan::def_scanner();

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

        assert_eq!(res_string, "\nkind=ID lexeme=a\nkind=ID lexeme=ababab\nkind=ID lexeme=_abab\nkind=ID lexeme=ab_abba_")
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

        let input = "fdkgdfjgdjglkdjglkdjgljbnhbduhoifjeoigjeoghknhkjdfjgoirjt for if endif elseif somethign eldsfnj hi bob joe here final for fob else if id idhere fobre f ".to_string();
        let mut iter = input.chars();
        let mut getter = || {
            iter.next()
        };
        let mut stream: StreamSource<char> = StreamSource::observe(&mut getter);

        let scanner = scan::def_scanner();
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
        let tree = parse_spec(input).unwrap();

        //verify
        assert_eq!(tree.to_string(),
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
    │       │   │       │       │       │   └── mtc
    │       │   │       │       │       │       └── CILC <- ''i''
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
    │       │       │           │   └── mtc
    │       │       │           │       └── CILC <- ''n''
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
    │           │       │       │   └── mtc
    │           │       │       │       └── CILC <- '' ''
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
                ├── pattopt
                │   └──  <- 'NULL'
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
    fn parse_spec_optional_shorthand() {
        //setup
        let spec = "
'ab'

start
  'a' -> ^A
  'b' -> ^B;

s -> A [B] s
  ->;
  ";

        //exercise
        let tree = parse_spec(spec).unwrap();

        //verify
        assert_eq!(tree.to_string(),
                   "└── spec
    ├── dfa
    │   ├── CILC <- ''ab''
    │   └── states
    │       └── state
    │           ├── sdec
    │           │   └── targets
    │           │       └── ID <- 'start'
    │           ├── transopt
    │           │   └── trans
    │           │       ├── trans
    │           │       │   └── tran
    │           │       │       ├── mtcs
    │           │       │       │   └── mtc
    │           │       │       │       └── CILC <- ''a''
    │           │       │       ├── ARROW <- '->'
    │           │       │       └── trand
    │           │       │           ├── HAT <- '^'
    │           │       │           └── ID <- 'A'
    │           │       └── tran
    │           │           ├── mtcs
    │           │           │   └── mtc
    │           │           │       └── CILC <- ''b''
    │           │           ├── ARROW <- '->'
    │           │           └── trand
    │           │               ├── HAT <- '^'
    │           │               └── ID <- 'B'
    │           └── SEMI <- ';'
    └── gram
        └── prods
            └── prod
                ├── ID <- 's'
                ├── pattopt
                │   └──  <- 'NULL'
                ├── rhss
                │   ├── rhss
                │   │   └── rhs
                │   │       ├── ARROW <- '->'
                │   │       ├── ids
                │   │       │   ├── ids
                │   │       │   │   ├── ids
                │   │       │   │   │   ├── ids
                │   │       │   │   │   │   └──  <- 'NULL'
                │   │       │   │   │   └── ID <- 'A'
                │   │       │   │   └── COPTID <- '[B]'
                │   │       │   └── ID <- 's'
                │   │       └── pattopt
                │   │           └──  <- 'NULL'
                │   └── rhs
                │       ├── ARROW <- '->'
                │       ├── ids
                │       │   └──  <- 'NULL'
                │       └── pattopt
                │           └──  <- 'NULL'
                └── SEMI <- ';'"
        );
    }

    #[test]
    fn single_reference_optional_shorthand() {
        //setup
        let spec = "
'ab'

start
  'a' -> ^A
  'b' -> ^B;

s -> A [B] s
  ->;
  ";

        let input = "ababaaaba".to_string();
        let mut iter = input.chars();
        let mut getter = || {
            iter.next()
        };
        let mut stream: StreamSource<char> = StreamSource::observe(&mut getter);

        let scanner = scan::def_scanner();
        let tree = parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, grammar, _) = generate_spec(&parse).unwrap();
        let parser = parse::def_parser();

        //exercise
        let tokens = scanner.scan(&mut stream, &cdfa).unwrap();
        let tree = parser.parse(tokens, &grammar).unwrap();

        //verify
        assert_eq!(tree.to_string(),
                   "└── s
    ├── A <- 'a'
    ├── opt#B
    │   └── B <- 'b'
    └── s
        ├── A <- 'a'
        ├── opt#B
        │   └── B <- 'b'
        └── s
            ├── A <- 'a'
            ├── opt#B
            │   └──  <- 'NULL'
            └── s
                ├── A <- 'a'
                ├── opt#B
                │   └──  <- 'NULL'
                └── s
                    ├── A <- 'a'
                    ├── opt#B
                    │   └── B <- 'b'
                    └── s
                        ├── A <- 'a'
                        ├── opt#B
                        │   └──  <- 'NULL'
                        └── s
                            └──  <- 'NULL'"
        );
    }

    #[test]
    fn def_pattern() {
        //setup
        let spec = "
'ab'

start
    'a' -> ^A
    'b' -> ^B;

s `{} {}`
    -> s A
    -> s B
    -> `SEPARATED:`;
        ";

        //exercise
        let tree = parse_spec(spec).unwrap();

        //verify
        assert_eq!(tree.to_string(),
                   "└── spec
    ├── dfa
    │   ├── CILC <- ''ab''
    │   └── states
    │       └── state
    │           ├── sdec
    │           │   └── targets
    │           │       └── ID <- 'start'
    │           ├── transopt
    │           │   └── trans
    │           │       ├── trans
    │           │       │   └── tran
    │           │       │       ├── mtcs
    │           │       │       │   └── mtc
    │           │       │       │       └── CILC <- ''a''
    │           │       │       ├── ARROW <- '->'
    │           │       │       └── trand
    │           │       │           ├── HAT <- '^'
    │           │       │           └── ID <- 'A'
    │           │       └── tran
    │           │           ├── mtcs
    │           │           │   └── mtc
    │           │           │       └── CILC <- ''b''
    │           │           ├── ARROW <- '->'
    │           │           └── trand
    │           │               ├── HAT <- '^'
    │           │               └── ID <- 'B'
    │           └── SEMI <- ';'
    └── gram
        └── prods
            └── prod
                ├── ID <- 's'
                ├── pattopt
                │   └── PATTC <- '`{} {}`'
                ├── rhss
                │   ├── rhss
                │   │   ├── rhss
                │   │   │   └── rhs
                │   │   │       ├── ARROW <- '->'
                │   │   │       ├── ids
                │   │   │       │   ├── ids
                │   │   │       │   │   ├── ids
                │   │   │       │   │   │   └──  <- 'NULL'
                │   │   │       │   │   └── ID <- 's'
                │   │   │       │   └── ID <- 'A'
                │   │   │       └── pattopt
                │   │   │           └──  <- 'NULL'
                │   │   └── rhs
                │   │       ├── ARROW <- '->'
                │   │       ├── ids
                │   │       │   ├── ids
                │   │       │   │   ├── ids
                │   │       │   │   │   └──  <- 'NULL'
                │   │       │   │   └── ID <- 's'
                │   │       │   └── ID <- 'B'
                │   │       └── pattopt
                │   │           └──  <- 'NULL'
                │   └── rhs
                │       ├── ARROW <- '->'
                │       ├── ids
                │       │   └──  <- 'NULL'
                │       └── pattopt
                │           └── PATTC <- '`SEPARATED:`'
                └── SEMI <- ';'"
        );
    }

    #[test]
    fn range_based_matchers() {
        //setup
        let spec = "
'abcdefghijklmnopqrstuvwxyz'

start
  'a'..'d' -> ^A
  'e'..'k' | 'l' -> ^B
  'm'..'m' -> ^C
  'n'..'o' -> ^D
  _ -> ^E;

E
    'p'..'z' -> E;

s -> ;";

        let input = "abcdefghijklmnopqrstuvwxyz".to_string();
        let mut iter = input.chars();
        let mut getter = || iter.next();
        let mut stream: StreamSource<char> = StreamSource::observe(&mut getter);

        let scanner = scan::def_scanner();
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
kind=A lexeme=a
kind=A lexeme=b
kind=A lexeme=c
kind=A lexeme=d
kind=B lexeme=e
kind=B lexeme=f
kind=B lexeme=g
kind=B lexeme=h
kind=B lexeme=i
kind=B lexeme=j
kind=B lexeme=k
kind=B lexeme=l
kind=C lexeme=m
kind=D lexeme=n
kind=D lexeme=o
kind=E lexeme=pqrstuvwxyz")
    }
}
