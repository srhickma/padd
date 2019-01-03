use {
    core::{
        data::stream::StreamSource,
        parse::{
            self,
            grammar::{Grammar, GrammarBuilder},
            Tree,
        },
        scan::{
            self,
            CDFABuilder,
            ecdfa::{EncodedCDFA, EncodedCDFABuilder},
        },
        spec,
    },
};

static SPEC_ALPHABET: &'static str = "`-=~!@#$%^&*()+{}|[]\\;':\"<>?,./_0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ \n\t";

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
enum S {
    Start,
    Alphabet,
    AlphabetTag,
    AlphabetString,
    AlphabetStringPartial,
    AlphabetStringEscaped,
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
        .mark_chain(&S::CDFATag, "cdfa".chars())?
        .mark_chain(&S::GrammarTag, "grammar".chars())?
        .mark_trans(&S::Comment, '#')?
        .mark_trans(&S::Whitespace, ' ')?
        .mark_trans(&S::Whitespace, '\t')?
        .mark_trans(&S::Whitespace, '\n')?;

    // Alphabet

    builder.state(&S::AlphabetTag)
        .accept_to_from_all(&S::Alphabet)?
        .tokenize(&"ALPHABET".to_string());

    builder.state(&S::Alphabet)
        .mark_trans(&S::AlphabetStringPartial, '\'')?
        .mark_trans(&S::Comment, '#')?
        .mark_trans(&S::Whitespace, ' ')?
        .mark_trans(&S::Whitespace, '\t')?
        .mark_trans(&S::Whitespace, '\n')?;

    builder.state(&S::AlphabetStringPartial)
        .mark_trans(&S::AlphabetString, '\'')?
        .mark_trans(&S::AlphabetStringEscaped, '\\')?
        .default_to(&S::AlphabetStringPartial)?;

    builder.state(&S::AlphabetString)
        .accept_to_from_all(&S::Start)?
        .tokenize(&"CILC".to_string());

    builder.state(&S::AlphabetStringEscaped)
        .default_to(&S::AlphabetStringPartial)?;

    // CDFA

    builder.state(&S::CDFATag)
        .accept_to_from_all(&S::CDFA)?
        .tokenize(&"CDFA".to_string());

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
        .tokenize(&"GRAMMAR".to_string());

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
        "spec regions",
        "regions regions region",
        "regions region",
        "region alphabet",
        "region cdfa",
        "region grammar",
        "alphabet ALPHABET CILC",
        "cdfa CDFA LBRACE states RBRACE",
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
        "grammar GRAMMAR LBRACE prods RBRACE",
        "prods prods prod",
        "prods prod",
        "prod ID patt_opt rhss SEMI",
        "rhss rhss rhs",
        "rhss rhs",
        "rhs OR ids patt_opt",
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

pub fn parse_spec(input: &str) -> Result<Tree, spec::ParseError> {
    SPEC_ECDFA.with(|cdfa| -> Result<Tree, spec::ParseError> {
        let mut iter = input.chars();
        let mut getter = || iter.next();
        let mut source = StreamSource::observe(&mut getter);

        let tokens = scan::def_scanner().scan(&mut source, cdfa)?;
        let parse = parse::def_parser().parse(tokens, &SPEC_GRAMMAR)?;
        Ok(parse)
    })
}
