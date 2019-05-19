use core::{
    data::Data,
    parse::{
        self,
        grammar::{self, GrammarBuilder, SimpleGrammar, SimpleGrammarBuilder, GrammarSymbol},
        Tree,
    },
    scan::{
        self,
        ecdfa::{EncodedCDFA, EncodedCDFABuilder},
        CDFABuilder,
    },
    spec,
};

static SPEC_ALPHABET: &'static str = "`-=~!@#$%^&*()+{}|[]\\;':\"<>?,./_0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ \n\t\r";

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
enum S {
    Start,
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

fn build_spec_ecdfa() -> Result<EncodedCDFA<SpecSymbol>, scan::CDFAError> {
    let mut builder: EncodedCDFABuilder<S, SpecSymbol> = EncodedCDFABuilder::new();

    builder
        .set_alphabet(SPEC_ALPHABET.chars())
        .mark_start(&S::Start);

    builder
        .state(&S::Start)
        .mark_chain(&S::IgnorableTag, "ignore".chars())?
        .mark_chain(&S::AlphabetTag, "alphabet".chars())?
        .mark_chain(&S::CDFATag, "cdfa".chars())?
        .mark_chain(&S::GrammarTag, "grammar".chars())?
        .mark_trans(&S::Comment, '#')?
        .mark_trans(&S::Whitespace, ' ')?
        .mark_trans(&S::Whitespace, '\t')?
        .mark_trans(&S::Whitespace, '\n')?
        .mark_trans(&S::Whitespace, '\r')?;

    build_ignorable_region(&mut builder)?;
    build_alphabet_region(&mut builder)?;
    build_cdfa_region(&mut builder)?;
    build_grammar_region(&mut builder)?;

    builder.state(&S::Whitespace).accept();

    builder
        .state(&S::RegionExitBrace)
        .accept_to_from_all(&S::Start)?
        .tokenize(&SpecSymbol::TRightBrace);

    builder.state(&S::Or).accept().tokenize(&SpecSymbol::TOr);

    builder.state(&S::Semi).accept().tokenize(&SpecSymbol::TSemi);

    builder
        .state(&S::CilPartial)
        .mark_trans(&S::Cil, '\'')?
        .mark_trans(&S::CilEscaped, '\\')?
        .default_to(&S::CilPartial)?;

    builder.state(&S::Cil).accept().tokenize(&SpecSymbol::TCil);

    builder.state(&S::CilEscaped).default_to(&S::CilPartial)?;

    builder
        .state(&S::Id)
        .mark_range(&S::Id, '_', 'Z')?
        .accept()
        .tokenize(&SpecSymbol::TId);

    builder
        .state(&S::Comment)
        .mark_trans(&S::Fail, '\n')?
        .default_to(&S::Comment)?
        .accept();

    builder.build()
}

fn build_ignorable_region(
    builder: &mut EncodedCDFABuilder<S, SpecSymbol>,
) -> Result<(), scan::CDFAError> {
    builder
        .state(&S::IgnorableTag)
        .accept_to_from_all(&S::Ignorable)?
        .tokenize(&SpecSymbol::TIgnorable);

    builder
        .state(&S::Ignorable)
        .mark_range(&S::IgnorableId, '0', 'Z')?
        .mark_trans(&S::Comment, '#')?
        .mark_trans(&S::Whitespace, ' ')?
        .mark_trans(&S::Whitespace, '\t')?
        .mark_trans(&S::Whitespace, '\n')?;

    builder
        .state(&S::IgnorableId)
        .mark_range(&S::IgnorableId, '_', 'Z')?
        .accept_to_from_all(&S::Start)?
        .tokenize(&SpecSymbol::TId);

    Ok(())
}

fn build_alphabet_region(
    builder: &mut EncodedCDFABuilder<S, SpecSymbol>,
) -> Result<(), scan::CDFAError> {
    builder
        .state(&S::AlphabetTag)
        .accept_to_from_all(&S::Alphabet)?
        .tokenize(&SpecSymbol::TAlphabet);

    builder
        .state(&S::Alphabet)
        .mark_trans(&S::AlphabetStringPartial, '\'')?
        .mark_trans(&S::Comment, '#')?
        .mark_trans(&S::Whitespace, ' ')?
        .mark_trans(&S::Whitespace, '\t')?
        .mark_trans(&S::Whitespace, '\n')?
        .mark_trans(&S::Whitespace, '\r')?;

    builder
        .state(&S::AlphabetStringPartial)
        .mark_trans(&S::AlphabetString, '\'')?
        .mark_trans(&S::AlphabetStringEscaped, '\\')?
        .default_to(&S::AlphabetStringPartial)?;

    builder
        .state(&S::AlphabetString)
        .accept_to_from_all(&S::Start)?
        .tokenize(&SpecSymbol::TCil);

    builder
        .state(&S::AlphabetStringEscaped)
        .default_to(&S::AlphabetStringPartial)?;

    Ok(())
}

fn build_cdfa_region(builder: &mut EncodedCDFABuilder<S, SpecSymbol>) -> Result<(), scan::CDFAError> {
    builder
        .state(&S::CDFATag)
        .accept_to_from_all(&S::CDFA)?
        .tokenize(&SpecSymbol::TCDFA);

    builder
        .state(&S::CDFA)
        .mark_trans(&S::CDFAEntryBrace, '{')?
        .mark_trans(&S::Comment, '#')?
        .mark_trans(&S::Whitespace, ' ')?
        .mark_trans(&S::Whitespace, '\t')?
        .mark_trans(&S::Whitespace, '\n')?
        .mark_trans(&S::Whitespace, '\r')?;

    builder
        .state(&S::CDFAEntryBrace)
        .accept_to_from_all(&S::CDFABody)?
        .tokenize(&SpecSymbol::TLeftBrace);

    builder
        .state(&S::CDFABody)
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
        .mark_trans(&S::Whitespace, '\n')?
        .mark_trans(&S::Whitespace, '\r')?;

    builder.state(&S::Hat).accept().tokenize(&SpecSymbol::THat);

    builder.state(&S::Arrow).accept().tokenize(&SpecSymbol::TArrow);

    builder.state(&S::Range).accept().tokenize(&SpecSymbol::TRange);

    builder.state(&S::Def).accept().tokenize(&SpecSymbol::TDef);

    Ok(())
}

fn build_grammar_region(
    builder: &mut EncodedCDFABuilder<S, SpecSymbol>,
) -> Result<(), scan::CDFAError> {
    builder
        .state(&S::GrammarTag)
        .accept_to_from_all(&S::Grammar)?
        .tokenize(&SpecSymbol::TGrammar);

    builder
        .state(&S::Grammar)
        .mark_trans(&S::GrammarEntryBrace, '{')?
        .mark_trans(&S::Comment, '#')?
        .mark_trans(&S::Whitespace, ' ')?
        .mark_trans(&S::Whitespace, '\t')?
        .mark_trans(&S::Whitespace, '\n')?
        .mark_trans(&S::Whitespace, '\r')?;

    builder
        .state(&S::GrammarEntryBrace)
        .accept_to_from_all(&S::GrammarBody)?
        .tokenize(&SpecSymbol::TLeftBrace);

    builder
        .state(&S::GrammarBody)
        .mark_trans(&S::Or, '|')?
        .mark_trans(&S::Semi, ';')?
        .mark_range(&S::Id, '0', 'Z')?
        .mark_trans(&S::OptIdPartial, '[')?
        .mark_trans(&S::PatternPartial, '`')?
        .mark_trans(&S::RegionExitBrace, '}')?
        .mark_trans(&S::Comment, '#')?
        .mark_trans(&S::Whitespace, ' ')?
        .mark_trans(&S::Whitespace, '\t')?
        .mark_trans(&S::Whitespace, '\n')?
        .mark_trans(&S::Whitespace, '\r')?;

    builder
        .state(&S::OptIdPartial)
        .mark_trans(&S::OptId, ']')?
        .mark_range(&S::OptIdPartial, '_', 'Z')?;

    builder.state(&S::OptId).accept().tokenize(&SpecSymbol::TOptId);

    builder
        .state(&S::PatternPartial)
        .mark_trans(&S::Pattern, '`')?
        .default_to(&S::PatternPartial)?;

    builder
        .state(&S::Pattern)
        .accept()
        .tokenize(&SpecSymbol::TPattern);

    Ok(())
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum SpecSymbol {
    Spec,
    Regions,
    Region,
    Ignorable,
    Alphabet,
    CDFA,
    States,
    State,
    StateDeclarator,
    TransitionsOpt,
    Transitions,
    Transition,
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
    TId,
    TOr,
    TRange,
    TGrammar,
    TPattern,
    TOptId,
    TDef,
    TIgnorable,
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

impl GrammarSymbol for SpecSymbol {
}

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
        .to(vec![SpecSymbol::Ignorable])
        .to(vec![SpecSymbol::Alphabet])
        .to(vec![SpecSymbol::CDFA])
        .to(vec![SpecSymbol::Grammar]);

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

    builder.from(SpecSymbol::Targets).to(vec![SpecSymbol::TId]).to(vec![
        SpecSymbol::Targets,
        SpecSymbol::TOr,
        SpecSymbol::TId,
    ]);

    builder
        .from(SpecSymbol::TransitionsOpt)
        .to(vec![SpecSymbol::Transitions])
        .epsilon();

    builder
        .from(SpecSymbol::Transitions)
        .to(vec![SpecSymbol::Transitions, SpecSymbol::Transition])
        .to(vec![SpecSymbol::Transition]);

    builder
        .from(SpecSymbol::Transition)
        .to(vec![
            SpecSymbol::Matchers,
            SpecSymbol::TArrow,
            SpecSymbol::TransitionDestination,
        ])
        .to(vec![
            SpecSymbol::TDef,
            SpecSymbol::TArrow,
            SpecSymbol::TransitionDestination,
        ]);

    builder
        .from(SpecSymbol::TransitionDestination)
        .to(vec![SpecSymbol::TId])
        .to(vec![SpecSymbol::Acceptor]);

    builder
        .from(SpecSymbol::Matchers)
        .to(vec![SpecSymbol::Matchers, SpecSymbol::TOr, SpecSymbol::Matcher])
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

    builder
        .from(SpecSymbol::RightHandSide)
        .to(vec![SpecSymbol::TOr, SpecSymbol::Ids, SpecSymbol::PatternOpt]);

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

        let tokens = scan::def_scanner().scan(&chars[..], cdfa)?;
        let parse = parse::def_parser().parse(tokens, &*SPEC_GRAMMAR)?;
        Ok(parse)
    })
}
