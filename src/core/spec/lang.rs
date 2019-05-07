use core::{
    data::Data,
    parse::{
        self,
        grammar::{Grammar, GrammarBuilder},
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
    static SPEC_ECDFA: EncodedCDFA<Symbol> = build_spec_ecdfa().unwrap();
}

fn build_spec_ecdfa() -> Result<EncodedCDFA<Symbol>, scan::CDFAError> {
    let mut builder: EncodedCDFABuilder<S, Symbol> = EncodedCDFABuilder::new();

    builder
        .set_alphabet(SPEC_ALPHABET.chars())
        .mark_start(&S::Start);

    builder
        .state(&S::Start)
        .mark_chain(&S::AlphabetTag, "alphabet".chars())?
        .mark_chain(&S::CDFATag, "cdfa".chars())?
        .mark_chain(&S::GrammarTag, "grammar".chars())?
        .mark_trans(&S::Comment, '#')?
        .mark_trans(&S::Whitespace, ' ')?
        .mark_trans(&S::Whitespace, '\t')?
        .mark_trans(&S::Whitespace, '\n')?
        .mark_trans(&S::Whitespace, '\r')?;

    // Alphabet

    builder
        .state(&S::AlphabetTag)
        .accept_to_from_all(&S::Alphabet)?
        .tokenize(&Symbol::TAlphabet);

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
        .tokenize(&Symbol::TCil);

    builder
        .state(&S::AlphabetStringEscaped)
        .default_to(&S::AlphabetStringPartial)?;

    // CDFA

    builder
        .state(&S::CDFATag)
        .accept_to_from_all(&S::CDFA)?
        .tokenize(&Symbol::TCDFA);

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
        .tokenize(&Symbol::TLeftBrace);

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

    builder.state(&S::Hat).accept().tokenize(&Symbol::THat);

    builder.state(&S::Arrow).accept().tokenize(&Symbol::TArrow);

    builder.state(&S::Range).accept().tokenize(&Symbol::TRange);

    builder.state(&S::Def).accept().tokenize(&Symbol::TDef);

    // Grammar

    builder
        .state(&S::GrammarTag)
        .accept_to_from_all(&S::Grammar)?
        .tokenize(&Symbol::TGrammar);

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
        .tokenize(&Symbol::TLeftBrace);

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

    builder.state(&S::OptId).accept().tokenize(&Symbol::TOptId);

    builder
        .state(&S::PatternPartial)
        .mark_trans(&S::Pattern, '`')?
        .default_to(&S::PatternPartial)?;

    builder
        .state(&S::Pattern)
        .accept()
        .tokenize(&Symbol::TPattern);

    // Shared

    builder.state(&S::Whitespace).accept();

    builder
        .state(&S::RegionExitBrace)
        .accept_to_from_all(&S::Start)?
        .tokenize(&Symbol::TRightBrace);

    builder.state(&S::Or).accept().tokenize(&Symbol::TOr);

    builder.state(&S::Semi).accept().tokenize(&Symbol::TSemi);

    builder
        .state(&S::CilPartial)
        .mark_trans(&S::Cil, '\'')?
        .mark_trans(&S::CilEscaped, '\\')?
        .default_to(&S::CilPartial)?;

    builder.state(&S::Cil).accept().tokenize(&Symbol::TCil);

    builder.state(&S::CilEscaped).default_to(&S::CilPartial)?;

    builder
        .state(&S::Id)
        .mark_range(&S::Id, '_', 'Z')?
        .accept()
        .tokenize(&Symbol::TId);

    builder
        .state(&S::Comment)
        .mark_trans(&S::Fail, '\n')?
        .default_to(&S::Comment)?
        .accept();

    builder.build()
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum Symbol {
    Spec,
    Regions,
    Region,
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
}

impl Default for Symbol {
    fn default() -> Symbol {
        Symbol::Spec
    }
}

impl Data for Symbol {
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

lazy_static! {
    static ref SPEC_GRAMMAR: Grammar<Symbol> = build_spec_grammar();
}

fn build_spec_grammar() -> Grammar<Symbol> {
    let mut builder = GrammarBuilder::new();
    builder.try_mark_start(&Symbol::Spec);

    builder.from(Symbol::Spec).to(vec![Symbol::Regions]);

    builder
        .from(Symbol::Regions)
        .to(vec![Symbol::Regions, Symbol::Region])
        .to(vec![Symbol::Region]);

    builder
        .from(Symbol::Region)
        .to(vec![Symbol::Alphabet])
        .to(vec![Symbol::CDFA])
        .to(vec![Symbol::Grammar]);

    builder
        .from(Symbol::Alphabet)
        .to(vec![Symbol::TAlphabet, Symbol::TCil]);

    builder.from(Symbol::CDFA).to(vec![
        Symbol::TCDFA,
        Symbol::TLeftBrace,
        Symbol::States,
        Symbol::TRightBrace,
    ]);

    builder
        .from(Symbol::States)
        .to(vec![Symbol::States, Symbol::State])
        .to(vec![Symbol::State]);

    builder.from(Symbol::State).to(vec![
        Symbol::StateDeclarator,
        Symbol::TransitionsOpt,
        Symbol::TSemi,
    ]);

    builder
        .from(Symbol::StateDeclarator)
        .to(vec![Symbol::Targets])
        .to(vec![Symbol::Targets, Symbol::Acceptor]);

    builder.from(Symbol::Acceptor).to(vec![
        Symbol::THat,
        Symbol::IdOrDef,
        Symbol::AcceptorDestinationOpt,
    ]);

    builder
        .from(Symbol::AcceptorDestinationOpt)
        .to(vec![Symbol::TArrow, Symbol::TId])
        .epsilon();

    builder.from(Symbol::Targets).to(vec![Symbol::TId]).to(vec![
        Symbol::Targets,
        Symbol::TOr,
        Symbol::TId,
    ]);

    builder
        .from(Symbol::TransitionsOpt)
        .to(vec![Symbol::Transitions])
        .epsilon();

    builder
        .from(Symbol::Transitions)
        .to(vec![Symbol::Transitions, Symbol::Transition])
        .to(vec![Symbol::Transition]);

    builder
        .from(Symbol::Transition)
        .to(vec![
            Symbol::Matchers,
            Symbol::TArrow,
            Symbol::TransitionDestination,
        ])
        .to(vec![
            Symbol::TDef,
            Symbol::TArrow,
            Symbol::TransitionDestination,
        ]);

    builder
        .from(Symbol::TransitionDestination)
        .to(vec![Symbol::TId])
        .to(vec![Symbol::Acceptor]);

    builder
        .from(Symbol::Matchers)
        .to(vec![Symbol::Matchers, Symbol::TOr, Symbol::Matcher])
        .to(vec![Symbol::Matcher]);

    builder
        .from(Symbol::Matcher)
        .to(vec![Symbol::TCil])
        .to(vec![Symbol::TCil, Symbol::TRange, Symbol::TCil]);

    builder.from(Symbol::Grammar).to(vec![
        Symbol::TGrammar,
        Symbol::TLeftBrace,
        Symbol::Productions,
        Symbol::TRightBrace,
    ]);

    builder
        .from(Symbol::Productions)
        .to(vec![Symbol::Productions, Symbol::Production])
        .to(vec![Symbol::Production]);

    builder.from(Symbol::Production).to(vec![
        Symbol::TId,
        Symbol::PatternOpt,
        Symbol::RightHandSides,
        Symbol::TSemi,
    ]);

    builder
        .from(Symbol::RightHandSides)
        .to(vec![Symbol::RightHandSides, Symbol::RightHandSide])
        .to(vec![Symbol::RightHandSide]);

    builder
        .from(Symbol::RightHandSide)
        .to(vec![Symbol::TOr, Symbol::Ids, Symbol::PatternOpt]);

    builder
        .from(Symbol::PatternOpt)
        .to(vec![Symbol::TPattern])
        .epsilon();

    builder
        .from(Symbol::Ids)
        .to(vec![Symbol::Ids, Symbol::TId])
        .to(vec![Symbol::Ids, Symbol::TOptId])
        .epsilon();

    builder
        .from(Symbol::IdOrDef)
        .to(vec![Symbol::TId])
        .to(vec![Symbol::TDef]);

    builder.build()
}

pub fn parse_spec(input: &str) -> Result<Tree<Symbol>, spec::ParseError> {
    SPEC_ECDFA.with(|cdfa| -> Result<Tree<Symbol>, spec::ParseError> {
        let chars: Vec<char> = input.chars().collect();

        let tokens = scan::def_scanner().scan(&chars[..], cdfa)?;
        let parse = parse::def_parser().parse(tokens, &SPEC_GRAMMAR)?;
        Ok(parse)
    })
}
