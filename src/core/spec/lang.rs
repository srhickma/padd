use core::{
    data::Data,
    lex::{
        self,
        ecdfa::{EncodedCDFA, EncodedCDFABuilder},
        CDFABuilder, Transit,
    },
    parse::{
        self,
        grammar::{self, GrammarBuilder, GrammarSymbol, SimpleGrammar, SimpleGrammarBuilder},
        Tree,
    },
    spec,
};

static SPEC_ALPHABET: &'static str = "`-=~!@#$%^&*()+{}|[]\\;':\"<>?,./_0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ \n\t\r";

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
enum S {
    Start,
    InjectableTag,
    InjectablePreAffinity,
    InjectionAffinity,
    InjectablePreId,
    InjectableId,
    InjectablePreComplete,
    Ignorable,
    IgnorableTag,
    IgnorableId,
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
    DoubleArrow,
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

impl Data for S {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

thread_local! {
    static SPEC_ECDFA: EncodedCDFA<SpecSymbol> = build_spec_ecdfa().unwrap();
}

fn build_spec_ecdfa() -> Result<EncodedCDFA<SpecSymbol>, lex::CDFAError> {
    let mut builder: EncodedCDFABuilder<S, SpecSymbol> = EncodedCDFABuilder::new();

    builder
        .set_alphabet(SPEC_ALPHABET.chars())
        .mark_start(&S::Start);

    builder
        .state(&S::Start)
        .mark_chain(Transit::to(S::InjectableTag), "inject".chars())?
        .mark_chain(Transit::to(S::IgnorableTag), "ignore".chars())?
        .mark_chain(Transit::to(S::AlphabetTag), "alphabet".chars())?
        .mark_chain(Transit::to(S::CDFATag), "cdfa".chars())?
        .mark_chain(Transit::to(S::GrammarTag), "grammar".chars())?
        .mark_trans(Transit::to(S::Comment), '#')?
        .mark_trans(Transit::to(S::Whitespace), ' ')?
        .mark_trans(Transit::to(S::Whitespace), '\t')?
        .mark_trans(Transit::to(S::Whitespace), '\n')?
        .mark_trans(Transit::to(S::Whitespace), '\r')?;

    build_injectable_region(&mut builder)?;
    build_ignorable_region(&mut builder)?;
    build_alphabet_region(&mut builder)?;
    build_cdfa_region(&mut builder)?;
    build_grammar_region(&mut builder)?;

    builder.state(&S::Whitespace).accept();

    builder
        .state(&S::RegionExitBrace)
        .accept_to(&S::Start)
        .tokenize(&SpecSymbol::TRightBrace);

    builder.state(&S::Or).accept().tokenize(&SpecSymbol::TOr);

    builder
        .state(&S::Semi)
        .accept()
        .tokenize(&SpecSymbol::TSemi);

    builder
        .state(&S::CilPartial)
        .mark_trans(Transit::to(S::Cil), '\'')?
        .mark_trans(Transit::to(S::CilEscaped), '\\')?
        .default_to(Transit::to(S::CilPartial))?;

    builder.state(&S::Cil).accept().tokenize(&SpecSymbol::TCil);

    builder
        .state(&S::CilEscaped)
        .default_to(Transit::to(S::CilPartial))?;

    builder //_0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ
        .state(&S::Id)
        .mark_range(Transit::to(S::Id), 'a', 'z')?
        .mark_range(Transit::to(S::Id), 'A', 'Z')?
        .mark_range(Transit::to(S::Id), '0', '9')?
        .mark_trans(Transit::to(S::Id), '_')?
        .accept()
        .tokenize(&SpecSymbol::TId);

    builder
        .state(&S::PatternPartial)
        .mark_trans(Transit::to(S::Pattern), '`')?
        .default_to(Transit::to(S::PatternPartial))?;

    builder
        .state(&S::Pattern)
        .accept()
        .tokenize(&SpecSymbol::TPattern);

    builder
        .state(&S::Comment)
        .mark_trans(Transit::to(S::Fail), '\n')?
        .default_to(Transit::to(S::Comment))?
        .accept();

    builder.build()
}

fn build_injectable_region(
    builder: &mut EncodedCDFABuilder<S, SpecSymbol>,
) -> Result<(), lex::CDFAError> {
    builder
        .state(&S::InjectableTag)
        .accept_to(&S::InjectablePreAffinity)
        .tokenize(&SpecSymbol::TInjectable);

    builder
        .state(&S::InjectablePreAffinity)
        .mark_chain(Transit::to(S::InjectionAffinity), "left".chars())?
        .mark_chain(Transit::to(S::InjectionAffinity), "right".chars())?
        .mark_trans(Transit::to(S::Comment), '#')?
        .mark_trans(Transit::to(S::Whitespace), ' ')?
        .mark_trans(Transit::to(S::Whitespace), '\t')?
        .mark_trans(Transit::to(S::Whitespace), '\r')?
        .mark_trans(Transit::to(S::Whitespace), '\n')?;

    builder
        .state(&S::InjectionAffinity)
        .accept_to(&S::InjectablePreId)
        .tokenize(&SpecSymbol::TInjectionAffinity);

    builder
        .state(&S::InjectablePreId)
        .mark_range(Transit::to(S::InjectableId), 'a', 'z')?
        .mark_range(Transit::to(S::InjectableId), 'A', 'Z')?
        .mark_range(Transit::to(S::InjectableId), '0', '9')?
        .mark_trans(Transit::to(S::Comment), '#')?
        .mark_trans(Transit::to(S::Whitespace), ' ')?
        .mark_trans(Transit::to(S::Whitespace), '\t')?
        .mark_trans(Transit::to(S::Whitespace), '\r')?
        .mark_trans(Transit::to(S::Whitespace), '\n')?;

    builder
        .state(&S::InjectableId)
        .mark_range(Transit::to(S::InjectableId), 'a', 'z')?
        .mark_range(Transit::to(S::InjectableId), 'A', 'Z')?
        .mark_range(Transit::to(S::InjectableId), '0', '9')?
        .mark_trans(Transit::to(S::InjectableId), '_')?
        .accept_to(&S::InjectablePreComplete)
        .tokenize(&SpecSymbol::TId);

    builder
        .state(&S::InjectablePreComplete)
        .mark_trans(Transit::to(S::PatternPartial), '`')?
        .mark_trans(Transit::to(S::Comment), '#')?
        .mark_trans(Transit::to(S::Whitespace), ' ')?
        .mark_trans(Transit::to(S::Whitespace), '\t')?
        .mark_trans(Transit::to(S::Whitespace), '\r')?
        .mark_trans(Transit::to(S::Whitespace), '\n')?
        .accept_to(&S::Start);

    Ok(())
}

fn build_ignorable_region(
    builder: &mut EncodedCDFABuilder<S, SpecSymbol>,
) -> Result<(), lex::CDFAError> {
    builder
        .state(&S::IgnorableTag)
        .accept_to(&S::Ignorable)
        .tokenize(&SpecSymbol::TIgnorable);

    builder
        .state(&S::Ignorable)
        .mark_range(Transit::to(S::IgnorableId), 'a', 'z')?
        .mark_range(Transit::to(S::IgnorableId), 'A', 'Z')?
        .mark_range(Transit::to(S::IgnorableId), '0', '9')?
        .mark_trans(Transit::to(S::Comment), '#')?
        .mark_trans(Transit::to(S::Whitespace), ' ')?
        .mark_trans(Transit::to(S::Whitespace), '\t')?
        .mark_trans(Transit::to(S::Whitespace), '\r')?
        .mark_trans(Transit::to(S::Whitespace), '\n')?;

    builder
        .state(&S::IgnorableId)
        .mark_range(Transit::to(S::IgnorableId), 'a', 'z')?
        .mark_range(Transit::to(S::IgnorableId), 'A', 'Z')?
        .mark_range(Transit::to(S::IgnorableId), '0', '9')?
        .mark_trans(Transit::to(S::IgnorableId), '_')?
        .accept_to(&S::Start)
        .tokenize(&SpecSymbol::TId);

    Ok(())
}

fn build_alphabet_region(
    builder: &mut EncodedCDFABuilder<S, SpecSymbol>,
) -> Result<(), lex::CDFAError> {
    builder
        .state(&S::AlphabetTag)
        .accept_to(&S::Alphabet)
        .tokenize(&SpecSymbol::TAlphabet);

    builder
        .state(&S::Alphabet)
        .mark_trans(Transit::to(S::AlphabetStringPartial), '\'')?
        .mark_trans(Transit::to(S::Comment), '#')?
        .mark_trans(Transit::to(S::Whitespace), ' ')?
        .mark_trans(Transit::to(S::Whitespace), '\t')?
        .mark_trans(Transit::to(S::Whitespace), '\r')?
        .mark_trans(Transit::to(S::Whitespace), '\n')?;

    builder
        .state(&S::AlphabetStringPartial)
        .mark_trans(Transit::to(S::AlphabetString), '\'')?
        .mark_trans(Transit::to(S::AlphabetStringEscaped), '\\')?
        .default_to(Transit::to(S::AlphabetStringPartial))?;

    builder
        .state(&S::AlphabetString)
        .accept_to(&S::Start)
        .tokenize(&SpecSymbol::TCil);

    builder
        .state(&S::AlphabetStringEscaped)
        .default_to(Transit::to(S::AlphabetStringPartial))?;

    Ok(())
}

fn build_cdfa_region(
    builder: &mut EncodedCDFABuilder<S, SpecSymbol>,
) -> Result<(), lex::CDFAError> {
    builder
        .state(&S::CDFATag)
        .accept_to(&S::CDFA)
        .tokenize(&SpecSymbol::TCDFA);

    builder
        .state(&S::CDFA)
        .mark_trans(Transit::to(S::CDFAEntryBrace), '{')?
        .mark_trans(Transit::to(S::Comment), '#')?
        .mark_trans(Transit::to(S::Whitespace), ' ')?
        .mark_trans(Transit::to(S::Whitespace), '\t')?
        .mark_trans(Transit::to(S::Whitespace), '\r')?
        .mark_trans(Transit::to(S::Whitespace), '\n')?;

    builder
        .state(&S::CDFAEntryBrace)
        .accept_to(&S::CDFABody)
        .tokenize(&SpecSymbol::TLeftBrace);

    builder
        .state(&S::CDFABody)
        .mark_trans(Transit::to(S::Or), '|')?
        .mark_trans(Transit::to(S::Semi), ';')?
        .mark_trans(Transit::to(S::CilPartial), '\'')?
        .mark_range(Transit::to(S::Id), 'a', 'z')?
        .mark_range(Transit::to(S::Id), 'A', 'Z')?
        .mark_range(Transit::to(S::Id), '0', '9')?
        .mark_trans(Transit::to(S::Hat), '^')?
        .mark_chain(Transit::to(S::Arrow), "->".chars())?
        .mark_chain(Transit::to(S::Range), "..".chars())?
        .mark_trans(Transit::to(S::Def), '_')?
        .mark_trans(Transit::to(S::RegionExitBrace), '}')?
        .mark_trans(Transit::to(S::Comment), '#')?
        .mark_trans(Transit::to(S::Whitespace), ' ')?
        .mark_trans(Transit::to(S::Whitespace), '\t')?
        .mark_trans(Transit::to(S::Whitespace), '\r')?
        .mark_trans(Transit::to(S::Whitespace), '\n')?;

    builder.state(&S::Hat).accept().tokenize(&SpecSymbol::THat);

    builder
        .state(&S::Arrow)
        .mark_trans(Transit::to(S::DoubleArrow), '>')?
        .accept()
        .tokenize(&SpecSymbol::TArrow);

    builder
        .state(&S::DoubleArrow)
        .accept()
        .tokenize(&SpecSymbol::TDoubleArrow);

    builder
        .state(&S::Range)
        .accept()
        .tokenize(&SpecSymbol::TRange);

    builder.state(&S::Def).accept().tokenize(&SpecSymbol::TDef);

    Ok(())
}

fn build_grammar_region(
    builder: &mut EncodedCDFABuilder<S, SpecSymbol>,
) -> Result<(), lex::CDFAError> {
    builder
        .state(&S::GrammarTag)
        .accept_to(&S::Grammar)
        .tokenize(&SpecSymbol::TGrammar);

    builder
        .state(&S::Grammar)
        .mark_trans(Transit::to(S::GrammarEntryBrace), '{')?
        .mark_trans(Transit::to(S::Comment), '#')?
        .mark_trans(Transit::to(S::Whitespace), ' ')?
        .mark_trans(Transit::to(S::Whitespace), '\t')?
        .mark_trans(Transit::to(S::Whitespace), '\r')?
        .mark_trans(Transit::to(S::Whitespace), '\n')?;

    builder
        .state(&S::GrammarEntryBrace)
        .accept_to(&S::GrammarBody)
        .tokenize(&SpecSymbol::TLeftBrace);

    builder
        .state(&S::GrammarBody)
        .mark_trans(Transit::to(S::Or), '|')?
        .mark_trans(Transit::to(S::Semi), ';')?
        .mark_range(Transit::to(S::Id), 'a', 'z')?
        .mark_range(Transit::to(S::Id), 'A', 'Z')?
        .mark_range(Transit::to(S::Id), '0', '9')?
        .mark_trans(Transit::to(S::OptIdPartial), '[')?
        .mark_trans(Transit::to(S::PatternPartial), '`')?
        .mark_trans(Transit::to(S::RegionExitBrace), '}')?
        .mark_trans(Transit::to(S::Comment), '#')?
        .mark_trans(Transit::to(S::Whitespace), ' ')?
        .mark_trans(Transit::to(S::Whitespace), '\t')?
        .mark_trans(Transit::to(S::Whitespace), '\r')?
        .mark_trans(Transit::to(S::Whitespace), '\n')?;

    builder
        .state(&S::OptIdPartial)
        .mark_trans(Transit::to(S::OptId), ']')?
        .mark_range(Transit::to(S::OptIdPartial), 'a', 'z')?
        .mark_range(Transit::to(S::OptIdPartial), 'A', 'Z')?
        .mark_range(Transit::to(S::OptIdPartial), '0', '9')?
        .mark_trans(Transit::to(S::OptIdPartial), '_')?;

    builder
        .state(&S::OptId)
        .accept()
        .tokenize(&SpecSymbol::TOptId);

    Ok(())
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum SpecSymbol {
    Spec,
    Regions,
    Region,
    Injectable,
    Ignorable,
    Alphabet,
    CDFA,
    States,
    State,
    StateDeclarator,
    TransitionsOpt,
    Transitions,
    Transition,
    TransitionPattern,
    TransitionMethod,
    TransitionDestination,
    Matchers,
    Matcher,
    Targets,
    Acceptor,
    IdOrDef,
    AcceptorDestinationOpt,
    Grammar,
    Productions,
    Production,
    PatternOpt,
    RightHandSides,
    RightHandSide,
    Ids,
    TAlphabet,
    TCil,
    TCDFA,
    TLeftBrace,
    TRightBrace,
    TSemi,
    THat,
    TArrow,
    TDoubleArrow,
    TId,
    TOr,
    TRange,
    TGrammar,
    TPattern,
    TOptId,
    TDef,
    TIgnorable,
    TInjectable,
    TInjectionAffinity,
}

impl Default for SpecSymbol {
    fn default() -> SpecSymbol {
        SpecSymbol::Spec
    }
}

impl Data for SpecSymbol {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

impl GrammarSymbol for SpecSymbol {}

lazy_static! {
    static ref SPEC_GRAMMAR: SimpleGrammar<SpecSymbol> = build_spec_grammar().unwrap();
}

fn build_spec_grammar() -> Result<SimpleGrammar<SpecSymbol>, grammar::BuildError> {
    let mut builder = SimpleGrammarBuilder::new();
    builder.try_mark_start(&SpecSymbol::Spec);

    builder.from(SpecSymbol::Spec).to(vec![SpecSymbol::Regions]);

    builder
        .from(SpecSymbol::Regions)
        .to(vec![SpecSymbol::Regions, SpecSymbol::Region])
        .to(vec![SpecSymbol::Region]);

    builder
        .from(SpecSymbol::Region)
        .to(vec![SpecSymbol::Injectable])
        .to(vec![SpecSymbol::Ignorable])
        .to(vec![SpecSymbol::Alphabet])
        .to(vec![SpecSymbol::CDFA])
        .to(vec![SpecSymbol::Grammar]);

    builder.from(SpecSymbol::Injectable).to(vec![
        SpecSymbol::TInjectable,
        SpecSymbol::TInjectionAffinity,
        SpecSymbol::TId,
        SpecSymbol::PatternOpt,
    ]);

    builder
        .from(SpecSymbol::Ignorable)
        .to(vec![SpecSymbol::TIgnorable, SpecSymbol::TId]);

    builder
        .from(SpecSymbol::Alphabet)
        .to(vec![SpecSymbol::TAlphabet, SpecSymbol::TCil]);

    builder.from(SpecSymbol::CDFA).to(vec![
        SpecSymbol::TCDFA,
        SpecSymbol::TLeftBrace,
        SpecSymbol::States,
        SpecSymbol::TRightBrace,
    ]);

    builder
        .from(SpecSymbol::States)
        .to(vec![SpecSymbol::States, SpecSymbol::State])
        .to(vec![SpecSymbol::State]);

    builder.from(SpecSymbol::State).to(vec![
        SpecSymbol::StateDeclarator,
        SpecSymbol::TransitionsOpt,
        SpecSymbol::TSemi,
    ]);

    builder
        .from(SpecSymbol::StateDeclarator)
        .to(vec![SpecSymbol::Targets])
        .to(vec![SpecSymbol::Targets, SpecSymbol::Acceptor]);

    builder.from(SpecSymbol::Acceptor).to(vec![
        SpecSymbol::THat,
        SpecSymbol::IdOrDef,
        SpecSymbol::AcceptorDestinationOpt,
    ]);

    builder
        .from(SpecSymbol::AcceptorDestinationOpt)
        .to(vec![SpecSymbol::TArrow, SpecSymbol::TId])
        .epsilon();

    builder
        .from(SpecSymbol::Targets)
        .to(vec![SpecSymbol::TId])
        .to(vec![SpecSymbol::Targets, SpecSymbol::TOr, SpecSymbol::TId]);

    builder
        .from(SpecSymbol::TransitionsOpt)
        .to(vec![SpecSymbol::Transitions])
        .epsilon();

    builder
        .from(SpecSymbol::Transitions)
        .to(vec![SpecSymbol::Transitions, SpecSymbol::Transition])
        .to(vec![SpecSymbol::Transition]);

    builder.from(SpecSymbol::Transition).to(vec![
        SpecSymbol::TransitionPattern,
        SpecSymbol::TransitionMethod,
        SpecSymbol::TransitionDestination,
    ]);

    builder
        .from(SpecSymbol::TransitionPattern)
        .to(vec![SpecSymbol::Matchers])
        .to(vec![SpecSymbol::TDef]);

    builder
        .from(SpecSymbol::TransitionMethod)
        .to(vec![SpecSymbol::TArrow])
        .to(vec![SpecSymbol::TDoubleArrow]);

    builder
        .from(SpecSymbol::TransitionDestination)
        .to(vec![SpecSymbol::TId])
        .to(vec![SpecSymbol::Acceptor]);

    builder
        .from(SpecSymbol::Matchers)
        .to(vec![
            SpecSymbol::Matchers,
            SpecSymbol::TOr,
            SpecSymbol::Matcher,
        ])
        .to(vec![SpecSymbol::Matcher]);

    builder
        .from(SpecSymbol::Matcher)
        .to(vec![SpecSymbol::TCil])
        .to(vec![SpecSymbol::TCil, SpecSymbol::TRange, SpecSymbol::TCil]);

    builder.from(SpecSymbol::Grammar).to(vec![
        SpecSymbol::TGrammar,
        SpecSymbol::TLeftBrace,
        SpecSymbol::Productions,
        SpecSymbol::TRightBrace,
    ]);

    builder
        .from(SpecSymbol::Productions)
        .to(vec![SpecSymbol::Productions, SpecSymbol::Production])
        .to(vec![SpecSymbol::Production]);

    builder.from(SpecSymbol::Production).to(vec![
        SpecSymbol::TId,
        SpecSymbol::PatternOpt,
        SpecSymbol::RightHandSides,
        SpecSymbol::TSemi,
    ]);

    builder
        .from(SpecSymbol::RightHandSides)
        .to(vec![SpecSymbol::RightHandSides, SpecSymbol::RightHandSide])
        .to(vec![SpecSymbol::RightHandSide]);

    builder.from(SpecSymbol::RightHandSide).to(vec![
        SpecSymbol::TOr,
        SpecSymbol::Ids,
        SpecSymbol::PatternOpt,
    ]);

    builder
        .from(SpecSymbol::PatternOpt)
        .to(vec![SpecSymbol::TPattern])
        .epsilon();

    builder
        .from(SpecSymbol::Ids)
        .to(vec![SpecSymbol::Ids, SpecSymbol::TId])
        .to(vec![SpecSymbol::Ids, SpecSymbol::TOptId])
        .epsilon();

    builder
        .from(SpecSymbol::IdOrDef)
        .to(vec![SpecSymbol::TId])
        .to(vec![SpecSymbol::TDef]);

    builder.build()
}

pub fn parse_spec(input: &str) -> Result<Tree<SpecSymbol>, spec::ParseError> {
    SPEC_ECDFA.with(|cdfa| -> Result<Tree<SpecSymbol>, spec::ParseError> {
        let chars: Vec<char> = input.chars().collect();

        let tokens = lex::def_lexer().lex(&chars[..], cdfa)?;
        let parse = parse::def_parser().parse(tokens, &*SPEC_GRAMMAR)?;
        Ok(parse)
    })
}
