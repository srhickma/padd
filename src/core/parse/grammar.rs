use {
    core::{
        data::Data,
        fmt::InjectionAffinity,
        parse::{Production, ProductionSymbol},
        util::encoder::Encoder,
    },
    std::{
        collections::{HashMap, HashSet},
        error, fmt,
    },
};

pub trait GrammarSymbol: Data + Default {}

impl GrammarSymbol for usize {}

impl GrammarSymbol for String {}

pub trait Grammar<Symbol: GrammarSymbol>: Send + Sync {
    fn is_nullable_nt(&self, lhs: &Symbol) -> bool;
    fn is_non_terminal(&self, symbol: &Symbol) -> bool;
    fn is_injectable(&self, symbol: &Symbol) -> bool;
    fn injection_affinity(&self, symbol: &Symbol) -> Option<&InjectionAffinity>;
    fn is_ignorable(&self, symbol: &Symbol) -> bool;
    fn terminals(&self) -> &HashSet<Symbol>;
    fn start(&self) -> &Symbol;
    fn productions_for_lhs(&self, lhs: &Symbol) -> Option<&Vec<Production<Symbol>>>;
    fn weighted_parse(&self) -> bool;
    fn symbol_string(&self, symbol: &Symbol) -> String;
}

pub trait GrammarBuilder<SymbolIn: GrammarSymbol, SymbolOut: GrammarSymbol, GrammarType> {
    fn add_optional_state(&mut self, opt_state: &SymbolIn, dest_state: &SymbolIn);
    fn add_production(&mut self, production: Production<SymbolIn>) -> Production<SymbolOut>;
    fn try_mark_start(&mut self, start: &SymbolIn);
    fn mark_injectable(&mut self, symbol: &SymbolIn, affinity: InjectionAffinity);
    fn mark_ignorable(&mut self, symbol: &SymbolIn);
    fn kind_for(&mut self, token: &SymbolIn) -> SymbolOut;
    fn build(self) -> Result<GrammarType, BuildError>;
}

pub struct SimpleGrammar<Symbol: GrammarSymbol> {
    prods_by_lhs: HashMap<Symbol, Vec<Production<Symbol>>>,
    nss: HashSet<Symbol>,
    non_terminals: HashSet<Symbol>,
    terminals: HashSet<Symbol>,
    injectable: HashMap<Symbol, InjectionAffinity>,
    ignorable: HashSet<Symbol>,
    start: Symbol,
}

impl<Symbol: GrammarSymbol> Grammar<Symbol> for SimpleGrammar<Symbol> {
    fn is_nullable_nt(&self, lhs: &Symbol) -> bool {
        self.nss.contains(lhs)
    }

    fn is_non_terminal(&self, symbol: &Symbol) -> bool {
        self.non_terminals.contains(symbol)
    }

    fn is_injectable(&self, symbol: &Symbol) -> bool {
        self.injectable.contains_key(symbol)
    }

    fn injection_affinity(&self, symbol: &Symbol) -> Option<&InjectionAffinity> {
        self.injectable.get(symbol)
    }

    fn is_ignorable(&self, symbol: &Symbol) -> bool {
        self.ignorable.contains(symbol)
    }

    fn terminals(&self) -> &HashSet<Symbol> {
        &self.terminals
    }

    fn start(&self) -> &Symbol {
        &self.start
    }

    fn productions_for_lhs(&self, lhs: &Symbol) -> Option<&Vec<Production<Symbol>>> {
        self.prods_by_lhs.get(lhs)
    }

    fn weighted_parse(&self) -> bool {
        !self.ignorable.is_empty() || !self.injectable.is_empty()
    }

    fn symbol_string(&self, symbol: &Symbol) -> String {
        symbol.to_string()
    }
}

pub struct SimpleGrammarBuilder<Symbol: GrammarSymbol> {
    prods_by_lhs: HashMap<Symbol, Vec<Production<Symbol>>>,
    injectable: HashMap<Symbol, InjectionAffinity>,
    ignorable: HashSet<Symbol>,
    start: Option<Symbol>,
}

impl<Symbol: GrammarSymbol> SimpleGrammarBuilder<Symbol> {
    pub fn new() -> Self {
        Self {
            prods_by_lhs: HashMap::new(),
            injectable: HashMap::new(),
            ignorable: HashSet::new(),
            start: None,
        }
    }

    pub fn from(
        &mut self,
        lhs: Symbol,
    ) -> NonTerminalBuilder<Symbol, Symbol, SimpleGrammar<Symbol>> {
        NonTerminalBuilder::new(self, lhs)
    }

    fn build_no_check(self) -> SimpleGrammar<Symbol> {
        if self.start.is_none() {
            panic!("No start state specified for grammar");
        }
        let start = self.start.unwrap();

        if self.prods_by_lhs.get(&start).is_none() {
            panic!("Start state has no productions");
        }

        let nss = build_nss(&self.prods_by_lhs);
        let non_terminals = build_non_terminals(&self.prods_by_lhs);
        let terminals = build_terminals(&self.prods_by_lhs, &non_terminals);

        SimpleGrammar {
            prods_by_lhs: self.prods_by_lhs,
            nss,
            non_terminals,
            terminals,
            injectable: self.injectable,
            ignorable: self.ignorable,
            start,
        }
    }
}

impl<Symbol: GrammarSymbol> GrammarBuilder<Symbol, Symbol, SimpleGrammar<Symbol>>
    for SimpleGrammarBuilder<Symbol>
{
    fn add_optional_state(&mut self, opt_state: &Symbol, dest_state: &Symbol) {
        if !self.prods_by_lhs.contains_key(&opt_state) {
            self.prods_by_lhs.insert(
                opt_state.clone(),
                vec![
                    Production::from(
                        opt_state.clone(),
                        vec![ProductionSymbol::symbol(dest_state.clone())],
                    ),
                    Production::epsilon(opt_state.clone()),
                ],
            );
        }
    }

    fn add_production(&mut self, production: Production<Symbol>) -> Production<Symbol> {
        if let Some(vec) = self.prods_by_lhs.get_mut(&production.lhs) {
            vec.push(production.clone());
            return production;
        }

        self.prods_by_lhs
            .insert(production.lhs.clone(), vec![production.clone()]);

        production
    }

    fn try_mark_start(&mut self, start: &Symbol) {
        if self.start.is_some() {
            return;
        }

        self.start = Some(start.clone());
    }

    fn mark_injectable(&mut self, symbol: &Symbol, affinity: InjectionAffinity) {
        self.injectable.insert(symbol.clone(), affinity);
    }

    fn mark_ignorable(&mut self, symbol: &Symbol) {
        self.ignorable.insert(symbol.clone());
    }

    fn kind_for(&mut self, token: &Symbol) -> Symbol {
        token.clone()
    }

    fn build(self) -> Result<SimpleGrammar<Symbol>, BuildError> {
        let grammar = self.build_no_check();
        check_grammar(&grammar, &|symbol| symbol.to_string())?;
        Ok(grammar)
    }
}

pub struct EncodedGrammar<SymbolIn: GrammarSymbol> {
    grammar: SimpleGrammar<usize>,
    encoder: Encoder<SymbolIn>,
}

impl<SymbolIn: GrammarSymbol> Grammar<usize> for EncodedGrammar<SymbolIn> {
    fn is_nullable_nt(&self, lhs: &usize) -> bool {
        self.grammar.is_nullable_nt(lhs)
    }

    fn is_non_terminal(&self, symbol: &usize) -> bool {
        self.grammar.is_non_terminal(symbol)
    }

    fn is_injectable(&self, symbol: &usize) -> bool {
        self.grammar.is_injectable(symbol)
    }

    fn injection_affinity(&self, symbol: &usize) -> Option<&InjectionAffinity> {
        self.grammar.injection_affinity(symbol)
    }

    fn is_ignorable(&self, symbol: &usize) -> bool {
        self.grammar.is_ignorable(symbol)
    }

    fn terminals(&self) -> &HashSet<usize> {
        self.grammar.terminals()
    }

    fn start(&self) -> &usize {
        self.grammar.start()
    }

    fn productions_for_lhs(&self, lhs: &usize) -> Option<&Vec<Production<usize>>> {
        self.grammar.productions_for_lhs(lhs)
    }

    fn weighted_parse(&self) -> bool {
        self.grammar.weighted_parse()
    }

    fn symbol_string(&self, symbol: &usize) -> String {
        self.encoder.decode(*symbol).unwrap().to_string()
    }
}

pub struct EncodedGrammarBuilder<SymbolIn: GrammarSymbol> {
    builder: SimpleGrammarBuilder<usize>,
    encoder: Encoder<SymbolIn>,
}

impl<SymbolIn: GrammarSymbol> EncodedGrammarBuilder<SymbolIn> {
    pub fn new() -> Self {
        Self {
            builder: SimpleGrammarBuilder::new(),
            encoder: Encoder::new(),
        }
    }
}

impl<SymbolIn: GrammarSymbol> GrammarBuilder<SymbolIn, usize, EncodedGrammar<SymbolIn>>
    for EncodedGrammarBuilder<SymbolIn>
{
    fn add_optional_state(&mut self, opt_state: &SymbolIn, dest_state: &SymbolIn) {
        let opt_state_encoded = self.encoder.encode(opt_state);
        let dest_state_encoded = self.encoder.encode(dest_state);

        self.builder
            .add_optional_state(&opt_state_encoded, &dest_state_encoded);
    }

    fn add_production(&mut self, production: Production<SymbolIn>) -> Production<usize> {
        let rhs: Vec<ProductionSymbol<usize>> = production
            .rhs
            .iter()
            .map(|sym| ProductionSymbol {
                symbol: self.encoder.encode(&sym.symbol),
                is_list: sym.is_list,
            })
            .collect();

        let encoded_production = Production::from(self.encoder.encode(&production.lhs), rhs);

        self.builder.add_production(encoded_production)
    }

    fn try_mark_start(&mut self, start: &SymbolIn) {
        self.builder.try_mark_start(&self.encoder.encode(start));
    }

    fn mark_injectable(&mut self, symbol: &SymbolIn, affinity: InjectionAffinity) {
        self.builder
            .mark_injectable(&self.encoder.encode(symbol), affinity);
    }

    fn mark_ignorable(&mut self, symbol: &SymbolIn) {
        self.builder.mark_ignorable(&self.encoder.encode(symbol));
    }

    fn kind_for(&mut self, token: &SymbolIn) -> usize {
        self.encoder.encode(token)
    }

    fn build(self) -> Result<EncodedGrammar<SymbolIn>, BuildError> {
        let encoder = self.encoder;
        let grammar = self.builder.build_no_check();
        check_grammar(&grammar, &|symbol| {
            encoder.decode(*symbol).unwrap().to_string()
        })?;
        Ok(EncodedGrammar { grammar, encoder })
    }
}

fn check_grammar<Symbol: GrammarSymbol>(
    grammar: &SimpleGrammar<Symbol>,
    symbol_decoder: &dyn Fn(&Symbol) -> String,
) -> Result<(), BuildError> {
    for ignored in &grammar.ignorable {
        if grammar.non_terminals.contains(ignored) {
            return Err(BuildError::NonTerminalIgnoredErr(symbol_decoder(ignored)));
        }
    }

    for injected in grammar.injectable.keys() {
        if grammar.non_terminals.contains(injected) {
            return Err(BuildError::NonTerminalInjectedErr(symbol_decoder(injected)));
        }
        if grammar.ignorable.contains(injected) {
            return Err(BuildError::IgnoredAndInjectedErr(symbol_decoder(injected)));
        }
    }

    Ok(())
}

fn build_non_terminals<Symbol: GrammarSymbol>(
    prods_by_lhs: &HashMap<Symbol, Vec<Production<Symbol>>>,
) -> HashSet<Symbol> {
    prods_by_lhs.iter().map(|(lhs, _)| lhs).cloned().collect()
}

fn build_terminals<Symbol: GrammarSymbol>(
    prods_by_lhs: &HashMap<Symbol, Vec<Production<Symbol>>>,
    non_terminals: &HashSet<Symbol>,
) -> HashSet<Symbol> {
    let mut terminals: HashSet<Symbol> = HashSet::new();

    for prods in prods_by_lhs.values() {
        for prod in prods {
            for sym in &prod.rhs {
                let symbol = &sym.symbol;
                if !non_terminals.contains(symbol) {
                    terminals.insert(symbol.clone());
                }
            }
        }
    }

    terminals
}

fn build_nss<Symbol: GrammarSymbol>(
    prods_by_lhs: &HashMap<Symbol, Vec<Production<Symbol>>>,
) -> HashSet<Symbol> {
    let mut nss: HashSet<Symbol> = HashSet::new();
    let mut prods_by_rhs: HashMap<&Symbol, Vec<&Production<Symbol>>> = HashMap::new();
    let mut work_stack: Vec<&Symbol> = Vec::new();

    prods_by_lhs
        .iter()
        .flat_map(|(_, prods)| prods)
        .for_each(|prod| {
            for sym in &prod.rhs {
                prods_by_rhs
                    .entry(&sym.symbol)
                    .or_insert_with(Vec::new)
                    .push(prod);
            }

            if prod.rhs.is_empty() {
                nss.insert(prod.lhs.clone());
                work_stack.push(&prod.lhs);
            }
        });

    loop {
        match work_stack.pop() {
            None => break,
            Some(work_symbol) => {
                if let Some(prods) = prods_by_rhs.get(work_symbol) {
                    for prod in prods {
                        if !nss.contains(&prod.lhs)
                            && prod.rhs.iter().all(|sym| nss.contains(&sym.symbol))
                        {
                            nss.insert(prod.lhs.clone());
                            work_stack.push(&prod.lhs);
                        }
                    }
                }
            }
        };
    }

    nss
}

pub struct NonTerminalBuilder<
    'builder,
    SymbolIn: GrammarSymbol,
    SymbolOut: GrammarSymbol,
    GrammarType,
> {
    grammar_builder: &'builder mut dyn GrammarBuilder<SymbolIn, SymbolOut, GrammarType>,
    lhs: SymbolIn,
}

// TODO(shane) support inline lists here as well.
impl<'builder, SymbolIn: GrammarSymbol, SymbolOut: GrammarSymbol, GrammarType>
    NonTerminalBuilder<'builder, SymbolIn, SymbolOut, GrammarType>
{
    fn new(
        grammar_builder: &'builder mut dyn GrammarBuilder<SymbolIn, SymbolOut, GrammarType>,
        lhs: SymbolIn,
    ) -> Self {
        Self {
            grammar_builder,
            lhs,
        }
    }

    pub fn to(&mut self, rhs: Vec<SymbolIn>) -> &mut Self {
        let rhs = rhs.into_iter().map(ProductionSymbol::symbol).collect();

        self.grammar_builder
            .add_production(Production::from(self.lhs.clone(), rhs));

        self
    }

    pub fn epsilon(&mut self) -> &mut Self {
        self.grammar_builder
            .add_production(Production::epsilon(self.lhs.clone()));

        self
    }
}

#[derive(Debug)]
pub enum BuildError {
    NonTerminalIgnoredErr(String),
    NonTerminalInjectedErr(String),
    IgnoredAndInjectedErr(String),
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::NonTerminalIgnoredErr(ref symbol) => {
                write!(f, "Ignored symbol '{}' is non-terminal", symbol)
            }
            Self::NonTerminalInjectedErr(ref symbol) => {
                write!(f, "Injected symbol '{}' is non-terminal", symbol)
            }
            Self::IgnoredAndInjectedErr(ref symbol) => {
                write!(f, "Symbol '{}' is both ignored and injected", symbol)
            }
        }
    }
}

impl error::Error for BuildError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Self::NonTerminalIgnoredErr(_) => None,
            Self::NonTerminalInjectedErr(_) => None,
            Self::IgnoredAndInjectedErr(_) => None,
        }
    }
}
