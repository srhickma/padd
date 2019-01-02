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

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
enum S {
    Start,
    Alphabet,
    AlphabetTag,
    CDFA,
    CDFATag,
    CDFAEntryBrace,
    CDFABody,
    Grammar,
    GrammarTag,
    GrammarEntryBrace,
    GrammarBody,
    RegionExitBrace,
    Or,
    Hat,
    Arrow,
    Range,
    Pattern,
    PatternPartial,
    Cil,
    CilPartial,
    CilEscaped,
    Comment,
    Semi,
    Def,
    Whitespace,
    Id,
    OptIdPartial,
    OptId,
    Fail,
}

thread_local! {
    static SPEC_ECDFA: EncodedCDFA<String> = build_spec_ecdfa().unwrap();
}

lazy_static! {
    static ref SPEC_GRAMMAR: Grammar = build_spec_grammar();
}

fn build_spec_ecdfa() -> Result<EncodedCDFA<String>, scan::CDFAError> {
    let mut builder: EncodedCDFABuilder<S, String> = EncodedCDFABuilder::new();

    builder.set_alphabet(SPEC_ALPHABET.chars())
        .mark_start(&S::Start);

    builder.state(&S::Start)
        .mark_chain(&S::AlphabetTag, "alphabet".chars())?
        .mark_chain(&S::CDFATag, "cdfa")?
        .mark_chain(&S::GrammarTag, "grammar")?
        .mark_trans(&S::Comment, '#')?
        .mark_trans(&S::Whitespace, ' ')?
        .mark_trans(&S::Whitespace, '\t')?
        .mark_trans(&S::Whitespace, '\n')?;

    // Alphabet

    builder.state(&S::AlphabetTag)
        .accept_to_from_all(&S::Alphabet)?
        .tokenize(&"ALP_T".to_string());

    builder.state(&S::Alphabet)
        .mark_trans(&S::CilPartial, '\'')?
        .mark_trans(&S::Comment, '#')?
        .mark_trans(&S::Whitespace, ' ')?
        .mark_trans(&S::Whitespace, '\t')?
        .mark_trans(&S::Whitespace, '\n')?;

    // CDFA

    builder.state(&S::CDFATag)
        .accept_to_from_all(&S::CDFA)?
        .tokenize(&"CDFA_T".to_string());

    builder.state(&S::CDFA)
        .mark_trans(&S::CDFAEntryBrace, '{')?
        .mark_trans(&S::Comment, '#')?
        .mark_trans(&S::Whitespace, ' ')?
        .mark_trans(&S::Whitespace, '\t')?
        .mark_trans(&S::Whitespace, '\n')?;

    builder.state(&S::CDFAEntryBrace)
        .accept_to_from_all(&S::CDFABody)?
        .tokenize(&"LBRACE".to_string());

    builder.state(&S::CDFABody)
        .mark_trans(&S::Or, '|')?
        .mark_trans(&S::Semi, ';')?
        .mark_trans(&S::CilPartial, '\'')?
        .mark_range(&S::Id, '0', 'Z')?
        .mark_trans(&S::Hat, '^')?
        .mark_chain(&S::Arrow, "->".chars())?
        .mark_chain(&S::Range, "..".chars())?
        .mark_trans(&S::Def, '_')?
        .mark_trans(&S::RegionExitBrace, '}')?
        .mark_trans(&S::Comment, '#')?
        .mark_trans(&S::Whitespace, ' ')?
        .mark_trans(&S::Whitespace, '\t')?
        .mark_trans(&S::Whitespace, '\n')?;

    builder.state(&S::Hat)
        .accept()
        .tokenize(&"HAT".to_string());

    builder.state(&S::Arrow)
        .accept()
        .tokenize(&"ARROW".to_string());

    builder.state(&S::Range)
        .accept()
        .tokenize(&"RANGE".to_string());

    builder.state(&S::Def)
        .accept()
        .tokenize(&"DEF".to_string());

    // Grammar

    builder.state(&S::GrammarTag)
        .accept_to_from_all(&S::Grammar)?
        .tokenize(&"GRAM_T".to_string());

    builder.state(&S::Grammar)
        .mark_trans(&S::GrammarEntryBrace, '{')?
        .mark_trans(&S::Comment, '#')?
        .mark_trans(&S::Whitespace, ' ')?
        .mark_trans(&S::Whitespace, '\t')?
        .mark_trans(&S::Whitespace, '\n')?;

    builder.state(&S::GrammarEntryBrace)
        .accept_to_from_all(&S::GrammarBody)?
        .tokenize(&"LBRACE".to_string());

    builder.state(&S::GrammarBody)
        .mark_trans(&S::Or, '|')?
        .mark_trans(&S::Semi, ';')?
        .mark_range(&S::Id, '0', 'Z')?
        .mark_trans(&S::OptIdPartial, '[')?
        .mark_trans(&S::PatternPartial, '`')?
        .mark_trans(&S::RegionExitBrace, '}')?
        .mark_trans(&S::Comment, '#')?
        .mark_trans(&S::Whitespace, ' ')?
        .mark_trans(&S::Whitespace, '\t')?
        .mark_trans(&S::Whitespace, '\n')?;

    builder.state(&S::OptIdPartial)
        .mark_trans(&S::OptId, ']')?
        .mark_range(&S::OptIdPartial, '_', 'Z')?;

    builder.state(&S::OptId)
        .accept()
        .tokenize(&"COPTID".to_string());

    builder.state(&S::PatternPartial)
        .mark_trans(&S::Pattern, '`')?
        .default_to(&S::PatternPartial)?;

    builder.state(&S::Pattern)
        .accept()
        .tokenize(&"PATTC".to_string());

    // Shared

    builder.state(&S::Whitespace)
        .accept();

    builder.state(&S::RegionExitBrace)
        .accept_to_from_all(&S::Start)?
        .tokenize(&"RBRACE".to_string());

    builder.state(&S::Or)
        .accept()
        .tokenize(&"OR".to_string());

    builder.state(&S::Semi)
        .accept()
        .tokenize(&"SEMI".to_string());

    builder.state(&S::CilPartial)
        .mark_trans(&S::Cil, '\'')?
        .mark_trans(&S::CilEscaped, '\\')?
        .default_to(&S::CilPartial)?;

    builder.state(&S::Cil)
        .accept()
        .tokenize(&"CILC".to_string());

    builder.state(&S::CilEscaped)
        .default_to(&S::CilPartial)?;

    builder.state(&S::Id)
        .mark_range(&S::Id, '_', 'Z')?
        .accept()
        .tokenize(&"ID".to_string());

    builder.state(&S::Comment)
        .mark_trans(&S::Fail, '\n')?
        .default_to(&S::Comment)?
        .accept();

    builder.build()
}

fn build_spec_grammar() -> Grammar {
    let productions = parse::build_prods(&[
        "spec dfa gram",
        "dfa CILC states",
        "states states state",
        "states state",
        "state sdec trans_opt SEMI",
        "sdec targets",
        "sdec targets acceptor",
        "acceptor HAT id_or_def accd_opt",
        "accd_opt ARROW ID",
        "accd_opt ",
        "targets ID",
        "targets targets OR ID",
        "trans_opt trans",
        "trans_opt ",
        "trans trans tran",
        "trans tran",
        "tran mtcs ARROW trand",
        "tran DEF ARROW trand",
        "trand ID",
        "trand acceptor",
        "mtcs mtcs OR mtc",
        "mtcs mtc",
        "mtc CILC",
        "mtc CILC RANGE CILC",
        "gram prods",
        "prods prods prod",
        "prods prod",
        "prod ID patt_opt rhss SEMI",
        "rhss rhss rhs",
        "rhss rhs",
        "rhs ARROW ids patt_opt",
        "patt_opt PATTC",
        "patt_opt ",
        "ids ids ID",
        "ids ids COPTID",
        "ids ",
        "id_or_def ID",
        "id_or_def DEF"
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

    if sdec_node.children.len() == 2 {
        let acceptor_node = sdec_node.get_child(1);
        let id_or_def_node = acceptor_node.get_child(1);
        let token = &id_or_def_node.get_child(0).lhs.lexeme;

        for state in &states {
            add_cdfa_tokenizer(acceptor_node, *state, None, token, builder)?;
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

    let dest = match &trand_node.get_child(0).lhs.kind[..] {
        "ID" => &trand_node.get_child(0).lhs.lexeme,
        "acceptor" => {
            let acceptor_node = trand_node.get_child(0);
            let id_or_def_node = acceptor_node.get_child(1);
            let token = &id_or_def_node.get_child(0).lhs.lexeme;

            // Immediate state pass-through
            for source in sources {
                add_cdfa_tokenizer(acceptor_node, token, Some(*source), token, builder)?;
            }

            token
        }
        kind => panic!("Unexpected transition destination kind: {}", kind)
    };

    let matcher = tran_node.get_child(0);
    match &matcher.lhs.kind[..] {
        "mtcs" => {
            generate_cdfa_mtcs(matcher, sources, dest, builder)?;
        }
        "DEF" => for source in sources {
            builder.default_to(source, dest)?;
        },
        &_ => panic!("Transition map input is neither CILC or DEF"),
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
    acceptor_node: &Tree,
    state: &State,
    from: Option<&State>,
    kind: &String,
    builder: &mut CDFABuilderType,
) -> Result<(), GenError> where
    CDFAType: CDFA<usize, String>,
    CDFABuilderType: CDFABuilder<String, String, CDFAType>
{
    let accd_opt_node = acceptor_node.get_child(2);
    if accd_opt_node.is_empty() {
        builder.accept(state);
    } else {
        let acceptor_destination = &accd_opt_node.get_child(1).lhs.lexeme;
        match from {
            None => builder.accept_to_from_all(state, acceptor_destination)?,
            Some(from_state) => builder.accept_to(state, from_state, acceptor_destination)?
        };
    }

    if kind != DEF_MATCHER {
        builder.tokenize(state, kind);
    }
    Ok(())
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
    use core::{
        data::{
            Data,
            stream::StreamSource,
        },
        scan::Token,
    };

    use super::*;

    #[test]
    fn parse_spec_spaces() {
        //setup
        let input = "' 'start;s->s b;";

        //exercise
        let tree = parse_spec(input).unwrap();

        //verify
        assert_eq!(tree.to_string(),
                   "└── spec
    ├── dfa
    │   ├── CILC <- '' ''
    │   └── states
    │       └── state
    │           ├── sdec
    │           │   └── targets
    │           │       └── ID <- 'start'
    │           ├── trans_opt
    │           │   └──  <- 'NULL'
    │           └── SEMI <- ';'
    └── gram
        └── prods
            └── prod
                ├── ID <- 's'
                ├── patt_opt
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
                │       └── patt_opt
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
    │       │   │   │       ├── trans_opt
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
    │       │   │       │   └── acceptor
    │       │   │       │       ├── HAT <- '^'
    │       │   │       │       ├── id_or_def
    │       │   │       │       │   └── ID <- 'WHITESPACE'
    │       │   │       │       └── accd_opt
    │       │   │       │           └──  <- 'NULL'
    │       │   │       ├── trans_opt
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
    │       │       │   └── acceptor
    │       │       │       ├── HAT <- '^'
    │       │       │       ├── id_or_def
    │       │       │       │   └── ID <- 'LBRACKET'
    │       │       │       └── accd_opt
    │       │       │           └──  <- 'NULL'
    │       │       ├── trans_opt
    │       │       │   └──  <- 'NULL'
    │       │       └── SEMI <- ';'
    │       └── state
    │           ├── sdec
    │           │   ├── targets
    │           │   │   └── ID <- 'rbr'
    │           │   └── acceptor
    │           │       ├── HAT <- '^'
    │           │       ├── id_or_def
    │           │       │   └── ID <- 'RBRACKET'
    │           │       └── accd_opt
    │           │           └──  <- 'NULL'
    │           ├── trans_opt
    │           │   └──  <- 'NULL'
    │           └── SEMI <- ';'
    └── gram
        └── prods
            ├── prods
            │   ├── prods
            │   │   └── prod
            │   │       ├── ID <- 's'
            │   │       ├── patt_opt
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
            │   │       │   │       └── patt_opt
            │   │       │   │           └──  <- 'NULL'
            │   │       │   └── rhs
            │   │       │       ├── ARROW <- '->'
            │   │       │       ├── ids
            │   │       │       │   └──  <- 'NULL'
            │   │       │       └── patt_opt
            │   │       │           └──  <- 'NULL'
            │   │       └── SEMI <- ';'
            │   └── prod
            │       ├── ID <- 'b'
            │       ├── patt_opt
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
            │       │   │       └── patt_opt
            │       │   │           └── PATTC <- '``'
            │       │   └── rhs
            │       │       ├── ARROW <- '->'
            │       │       ├── ids
            │       │       │   ├── ids
            │       │       │   │   └──  <- 'NULL'
            │       │       │   └── ID <- 'w'
            │       │       └── patt_opt
            │       │           └──  <- 'NULL'
            │       └── SEMI <- ';'
            └── prod
                ├── ID <- 'w'
                ├── patt_opt
                │   └──  <- 'NULL'
                ├── rhss
                │   └── rhs
                │       ├── ARROW <- '->'
                │       ├── ids
                │       │   ├── ids
                │       │   │   └──  <- 'NULL'
                │       │   └── ID <- 'WHITESPACE'
                │       └── patt_opt
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
        assert_eq!(tokens_string(tokens), "\nkind=ID lexeme=c\nkind=WS lexeme= \nkind=ID lexeme=c")
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
        assert_eq!(tokens_string(tokens), "\nkind=ID lexeme=a\nkind=ID lexeme=ababab\nkind=ID lexeme=_abab\nkind=ID lexeme=ab_abba_")
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
        assert_eq!(tokens_string(tokens), "
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
    │       │   │       ├── trans_opt
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
    │       │   │       │               └── acceptor
    │       │   │       │                   ├── HAT <- '^'
    │       │   │       │                   ├── id_or_def
    │       │   │       │                   │   └── ID <- 'ID'
    │       │   │       │                   └── accd_opt
    │       │   │       │                       └──  <- 'NULL'
    │       │   │       └── SEMI <- ';'
    │       │   └── state
    │       │       ├── sdec
    │       │       │   └── targets
    │       │       │       └── ID <- 'ki'
    │       │       ├── trans_opt
    │       │       │   └── trans
    │       │       │       └── tran
    │       │       │           ├── mtcs
    │       │       │           │   └── mtc
    │       │       │           │       └── CILC <- ''n''
    │       │       │           ├── ARROW <- '->'
    │       │       │           └── trand
    │       │       │               └── acceptor
    │       │       │                   ├── HAT <- '^'
    │       │       │                   ├── id_or_def
    │       │       │                   │   └── ID <- 'IN'
    │       │       │                   └── accd_opt
    │       │       │                       └──  <- 'NULL'
    │       │       └── SEMI <- ';'
    │       └── state
    │           ├── sdec
    │           │   └── targets
    │           │       ├── targets
    │           │       │   └── ID <- 'ID'
    │           │       ├── OR <- '|'
    │           │       └── ID <- 'ki'
    │           ├── trans_opt
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
                ├── patt_opt
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
                │   │       └── patt_opt
                │   │           └──  <- 'NULL'
                │   └── rhs
                │       ├── ARROW <- '->'
                │       ├── ids
                │       │   ├── ids
                │       │   │   └──  <- 'NULL'
                │       │   └── ID <- 'ID'
                │       └── patt_opt
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
    │           ├── trans_opt
    │           │   └── trans
    │           │       ├── trans
    │           │       │   └── tran
    │           │       │       ├── mtcs
    │           │       │       │   └── mtc
    │           │       │       │       └── CILC <- ''a''
    │           │       │       ├── ARROW <- '->'
    │           │       │       └── trand
    │           │       │           └── acceptor
    │           │       │               ├── HAT <- '^'
    │           │       │               ├── id_or_def
    │           │       │               │   └── ID <- 'A'
    │           │       │               └── accd_opt
    │           │       │                   └──  <- 'NULL'
    │           │       └── tran
    │           │           ├── mtcs
    │           │           │   └── mtc
    │           │           │       └── CILC <- ''b''
    │           │           ├── ARROW <- '->'
    │           │           └── trand
    │           │               └── acceptor
    │           │                   ├── HAT <- '^'
    │           │                   ├── id_or_def
    │           │                   │   └── ID <- 'B'
    │           │                   └── accd_opt
    │           │                       └──  <- 'NULL'
    │           └── SEMI <- ';'
    └── gram
        └── prods
            └── prod
                ├── ID <- 's'
                ├── patt_opt
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
                │   │       └── patt_opt
                │   │           └──  <- 'NULL'
                │   └── rhs
                │       ├── ARROW <- '->'
                │       ├── ids
                │       │   └──  <- 'NULL'
                │       └── patt_opt
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
    │           ├── trans_opt
    │           │   └── trans
    │           │       ├── trans
    │           │       │   └── tran
    │           │       │       ├── mtcs
    │           │       │       │   └── mtc
    │           │       │       │       └── CILC <- ''a''
    │           │       │       ├── ARROW <- '->'
    │           │       │       └── trand
    │           │       │           └── acceptor
    │           │       │               ├── HAT <- '^'
    │           │       │               ├── id_or_def
    │           │       │               │   └── ID <- 'A'
    │           │       │               └── accd_opt
    │           │       │                   └──  <- 'NULL'
    │           │       └── tran
    │           │           ├── mtcs
    │           │           │   └── mtc
    │           │           │       └── CILC <- ''b''
    │           │           ├── ARROW <- '->'
    │           │           └── trand
    │           │               └── acceptor
    │           │                   ├── HAT <- '^'
    │           │                   ├── id_or_def
    │           │                   │   └── ID <- 'B'
    │           │                   └── accd_opt
    │           │                       └──  <- 'NULL'
    │           └── SEMI <- ';'
    └── gram
        └── prods
            └── prod
                ├── ID <- 's'
                ├── patt_opt
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
                │   │   │       └── patt_opt
                │   │   │           └──  <- 'NULL'
                │   │   └── rhs
                │   │       ├── ARROW <- '->'
                │   │       ├── ids
                │   │       │   ├── ids
                │   │       │   │   ├── ids
                │   │       │   │   │   └──  <- 'NULL'
                │   │       │   │   └── ID <- 's'
                │   │       │   └── ID <- 'B'
                │   │       └── patt_opt
                │   │           └──  <- 'NULL'
                │   └── rhs
                │       ├── ARROW <- '->'
                │       ├── ids
                │       │   └──  <- 'NULL'
                │       └── patt_opt
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

    #[test]
    fn context_sensitive_scanner() {
        //setup
        let spec = "
'a!123456789'

start
    'a' -> a
    '!' -> bang_in;

bang_in ^BANG -> hidden;

a       ^A
    'a' -> a;

hidden
    '1' .. '9' -> num
    '!' -> ^BANG -> start;

num     ^NUM;

s -> ;";

        let input = "!!aaa!!a!49913!a".to_string();
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
        assert_eq!(tokens_string(tokens), "
kind=BANG lexeme=!
kind=BANG lexeme=!
kind=A lexeme=aaa
kind=BANG lexeme=!
kind=BANG lexeme=!
kind=A lexeme=a
kind=BANG lexeme=!
kind=NUM lexeme=4
kind=NUM lexeme=9
kind=NUM lexeme=9
kind=NUM lexeme=1
kind=NUM lexeme=3
kind=BANG lexeme=!
kind=A lexeme=a")
    }

    fn tokens_string(tokens: Vec<Token<String>>) -> String {
        let mut res_string = String::new();
        for token in tokens {
            res_string = format!("{}\nkind={} lexeme={}", res_string, token.kind, token.lexeme);
        }
        res_string
    }
}
