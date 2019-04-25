use core::{
    data::Data,
    parse::{
        self,
        grammar::{Grammar, GrammarBuilder},
        Production, Tree,
    },
    scan::{
        self,
        ecdfa::{EncodedCDFA, EncodedCDFABuilder},
        CDFABuilder,
    },
    spec,
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
        .mark_trans(&S::Whitespace, '\n')?;

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
        .mark_trans(&S::Whitespace, '\n')?;

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
        .mark_trans(&S::Whitespace, '\n')?;

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
        .mark_trans(&S::Whitespace, '\n')?;

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
        .mark_trans(&S::Whitespace, '\n')?;

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
        .mark_trans(&S::Whitespace, '\n')?;

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
    //TODO create macros to make this declaration simpler
    let productions: Vec<Production<Symbol>> = vec![
        Production::from(Symbol::Spec, vec![Symbol::Regions]),
        Production::from(Symbol::Regions, vec![Symbol::Regions, Symbol::Region]),
        Production::from(Symbol::Regions, vec![Symbol::Region]),
        Production::from(Symbol::Region, vec![Symbol::Alphabet]),
        Production::from(Symbol::Region, vec![Symbol::CDFA]),
        Production::from(Symbol::Region, vec![Symbol::Grammar]),
        Production::from(Symbol::Alphabet, vec![Symbol::TAlphabet, Symbol::TCil]),
        Production::from(
            Symbol::CDFA,
            vec![
                Symbol::TCDFA,
                Symbol::TLeftBrace,
                Symbol::States,
                Symbol::TRightBrace,
            ],
        ),
        Production::from(Symbol::States, vec![Symbol::States, Symbol::State]),
        Production::from(Symbol::States, vec![Symbol::State]),
        Production::from(
            Symbol::State,
            vec![
                Symbol::StateDeclarator,
                Symbol::TransitionsOpt,
                Symbol::TSemi,
            ],
        ),
        Production::from(Symbol::StateDeclarator, vec![Symbol::Targets]),
        Production::from(
            Symbol::StateDeclarator,
            vec![Symbol::Targets, Symbol::Acceptor],
        ),
        Production::from(
            Symbol::Acceptor,
            vec![
                Symbol::THat,
                Symbol::IdOrDef,
                Symbol::AcceptorDestinationOpt,
            ],
        ),
        Production::from(
            Symbol::AcceptorDestinationOpt,
            vec![Symbol::TArrow, Symbol::TId],
        ),
        Production::epsilon(Symbol::AcceptorDestinationOpt),
        Production::from(Symbol::Targets, vec![Symbol::TId]),
        Production::from(
            Symbol::Targets,
            vec![Symbol::Targets, Symbol::TOr, Symbol::TId],
        ),
        Production::from(Symbol::TransitionsOpt, vec![Symbol::Transitions]),
        Production::epsilon(Symbol::TransitionsOpt),
        Production::from(
            Symbol::Transitions,
            vec![Symbol::Transitions, Symbol::Transition],
        ),
        Production::from(Symbol::Transitions, vec![Symbol::Transition]),
        Production::from(
            Symbol::Transition,
            vec![
                Symbol::Matchers,
                Symbol::TArrow,
                Symbol::TransitionDestination,
            ],
        ),
        Production::from(
            Symbol::Transition,
            vec![Symbol::TDef, Symbol::TArrow, Symbol::TransitionDestination],
        ),
        Production::from(Symbol::TransitionDestination, vec![Symbol::TId]),
        Production::from(Symbol::TransitionDestination, vec![Symbol::Acceptor]),
        Production::from(
            Symbol::Matchers,
            vec![Symbol::Matchers, Symbol::TOr, Symbol::Matcher],
        ),
        Production::from(Symbol::Matchers, vec![Symbol::Matcher]),
        Production::from(Symbol::Matcher, vec![Symbol::TCil]),
        Production::from(
            Symbol::Matcher,
            vec![Symbol::TCil, Symbol::TRange, Symbol::TCil],
        ),
        Production::from(
            Symbol::Grammar,
            vec![
                Symbol::TGrammar,
                Symbol::TLeftBrace,
                Symbol::Productions,
                Symbol::TRightBrace,
            ],
        ),
        Production::from(
            Symbol::Productions,
            vec![Symbol::Productions, Symbol::Production],
        ),
        Production::from(Symbol::Productions, vec![Symbol::Production]),
        Production::from(
            Symbol::Production,
            vec![
                Symbol::TId,
                Symbol::PatternOpt,
                Symbol::RightHandSides,
                Symbol::TSemi,
            ],
        ),
        Production::from(
            Symbol::RightHandSides,
            vec![Symbol::RightHandSides, Symbol::RightHandSide],
        ),
        Production::from(Symbol::RightHandSides, vec![Symbol::RightHandSide]),
        Production::from(
            Symbol::RightHandSide,
            vec![Symbol::TOr, Symbol::Ids, Symbol::PatternOpt],
        ),
        Production::from(Symbol::PatternOpt, vec![Symbol::TPattern]),
        Production::epsilon(Symbol::PatternOpt),
        Production::from(Symbol::Ids, vec![Symbol::Ids, Symbol::TId]),
        Production::from(Symbol::Ids, vec![Symbol::Ids, Symbol::TOptId]),
        Production::epsilon(Symbol::Ids),
        Production::from(Symbol::IdOrDef, vec![Symbol::TId]),
        Production::from(Symbol::IdOrDef, vec![Symbol::TDef]),
    ];

    let mut builder = GrammarBuilder::new();
    builder.try_mark_start(&productions.first().unwrap().lhs);
    builder.add_productions(productions.clone());
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
