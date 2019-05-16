use {
    core::{data::Data, parse::Production, util::encoder::Encoder},
    std::{
        collections::{HashMap, HashSet},
        error, fmt,
    },
};

pub struct Grammar<Symbol: Data + Default> {
    prods_by_lhs: HashMap<Symbol, Vec<Production<Symbol>>>,
    nss: HashSet<Symbol>,
    non_terminals: HashSet<Symbol>,
    terminals: HashSet<Symbol>,
    ignorable: HashSet<Symbol>,
    start: Symbol,
    symbol_string: Box<Fn(&Symbol) -> String + Send + Sync>,
}

impl<Symbol: Data + Default> Grammar<Symbol> {
    pub fn is_nullable_nt(&self, lhs: &Symbol) -> bool {
        self.nss.contains(lhs)
    }

    pub fn is_non_terminal(&self, symbol: &Symbol) -> bool {
        self.non_terminals.contains(symbol)
    }

    pub fn is_ignorable(&self, symbol: &Symbol) -> bool {
        self.ignorable.contains(symbol)
    }

    pub fn terminals(&self) -> &HashSet<Symbol> {
        &self.terminals
    }

    pub fn start(&self) -> &Symbol {
        &self.start
    }

    pub fn productions_for_lhs(&self, lhs: &Symbol) -> Option<&Vec<Production<Symbol>>> {
        self.prods_by_lhs.get(lhs)
    }

    pub fn weighted_parse(&self) -> bool {
        !self.ignorable.is_empty()
    }

    pub fn symbol_string(&self, symbol: &Symbol) -> String {
        (self.symbol_string)(symbol)
    }
}

pub trait GrammarBuilder<SymbolIn: Data + Default, SymbolOut: Data + Default> {
    fn add_optional_state(&mut self, opt_state: &SymbolIn, dest_state: &SymbolIn);
    fn add_production(&mut self, production: Production<SymbolIn>) -> Production<SymbolOut>;
    fn try_mark_start(&mut self, start: &SymbolIn);
    fn mark_ignorable(&mut self, symbol: &SymbolIn);
    fn kind_for(&mut self, token: &SymbolIn) -> SymbolOut;
    fn build(self) -> Result<Grammar<SymbolOut>, BuildError>;
}

pub struct EncodedGrammarBuilder<Symbol: Data + Default> {
    builder: SimpleGrammarBuilder<usize>,
    encoder: Encoder<Symbol>,
}

impl<Symbol: Data + Default> EncodedGrammarBuilder<Symbol> {
    pub fn new() -> Self {
        EncodedGrammarBuilder {
            builder: SimpleGrammarBuilder::new(),
            encoder: Encoder::new(),
        }
    }
}

impl<Symbol: 'static + Data + Default> GrammarBuilder<Symbol, usize> for EncodedGrammarBuilder<Symbol> {
    fn add_optional_state(&mut self, opt_state: &Symbol, dest_state: &Symbol) {
        self.builder.add_optional_state(
            &self.encoder.encode(opt_state),
            &self.encoder.encode(dest_state),
        )
    }

    fn add_production(&mut self, production: Production<Symbol>) -> Production<usize> {
        let rhs: Vec<usize> = production.rhs
            .iter()
            .map(|sym| self.encoder.encode(sym))
            .collect();

        let encoded_production = Production::from(
            self.encoder.encode(&production.lhs),
            rhs,
        );

        self.builder.add_production(encoded_production)
    }

    fn try_mark_start(&mut self, start: &Symbol) {
        self.builder.try_mark_start(&self.encoder.encode(start))
    }

    fn mark_ignorable(&mut self, symbol: &Symbol) {
        self.builder.mark_ignorable(&self.encoder.encode(symbol));
    }

    fn kind_for(&mut self, token: &Symbol) -> usize {
        self.encoder.encode(token)
    }

    fn build(self) -> Result<Grammar<usize>, BuildError> {
        match self.builder.build() {
            Ok(mut grammar) => {
                let mut encoder = self.encoder;
                grammar.symbol_string = Box::new(move |symbol| encoder.decode(*symbol).unwrap().to_string());

                Ok(grammar)
            },
            Err(BuildError::NonTerminalIgnoredErr(symbol_string)) => {
                let symbol = symbol_string.parse::<usize>().unwrap();
                Err(BuildError::NonTerminalIgnoredErr(self.encoder.decode(symbol).unwrap().to_string()))
            },
        }
    }
}

pub struct SimpleGrammarBuilder<Symbol: Data + Default> {
    prods_by_lhs: HashMap<Symbol, Vec<Production<Symbol>>>,
    ignorable: HashSet<Symbol>,
    start: Option<Symbol>,
}

impl<Symbol: Data + Default> SimpleGrammarBuilder<Symbol> {
    pub fn new() -> Self {
        SimpleGrammarBuilder {
            prods_by_lhs: HashMap::new(),
            ignorable: HashSet::new(),
            start: None,
        }
    }

    pub fn from(&mut self, lhs: Symbol) -> NonTerminalBuilder<Symbol> {
        NonTerminalBuilder::new(self, lhs)
    }

    fn build_nss(prods_by_lhs: &HashMap<Symbol, Vec<Production<Symbol>>>) -> HashSet<Symbol> {
        let mut nss: HashSet<Symbol> = HashSet::new();
        let mut prods_by_rhs: HashMap<&Symbol, Vec<&Production<Symbol>>> = HashMap::new();
        let mut work_stack: Vec<&Symbol> = Vec::new();

        prods_by_lhs
            .iter()
            .flat_map(|(_, prods)| prods)
            .for_each(|prod| {
                for s in &prod.rhs {
                    prods_by_rhs.entry(s).or_insert_with(Vec::new).push(prod);
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
                                && prod.rhs.iter().all(|sym| nss.contains(sym))
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
}

impl<Symbol: Data + Default> GrammarBuilder<Symbol, Symbol> for SimpleGrammarBuilder<Symbol> {
    fn add_optional_state(&mut self, opt_state: &Symbol, dest_state: &Symbol) {
        if !self.prods_by_lhs.contains_key(opt_state) {
            self.prods_by_lhs
                .entry(opt_state.clone())
                .or_insert_with(|| {
                    vec![
                        Production {
                            lhs: opt_state.clone(),
                            rhs: vec![dest_state.clone()],
                        },
                        Production {
                            lhs: opt_state.clone(),
                            rhs: Vec::new(),
                        },
                    ]
                });
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

    fn mark_ignorable(&mut self, symbol: &Symbol) {
        self.ignorable.insert(symbol.clone());
    }

    fn kind_for(&mut self, token: &Symbol) -> Symbol {
        token.clone()
    }

    fn build(self) -> Result<Grammar<Symbol>, BuildError> {
        if self.start.is_none() {
            panic!("No start state specified for grammar");
        }
        let start = self.start.unwrap();

        if self.prods_by_lhs.get(&start).is_none() {
            panic!("Start state has no productions");
        }

        let nss: HashSet<Symbol> = SimpleGrammarBuilder::build_nss(&self.prods_by_lhs);

        let non_terminals: HashSet<Symbol> = self
            .prods_by_lhs
            .iter()
            .map(|(lhs, _)| lhs)
            .cloned()
            .collect();

        let terminals: HashSet<Symbol> = self
            .prods_by_lhs
            .iter()
            .flat_map(|(_, prods)| prods)
            .flat_map(|prod| &prod.rhs)
            .filter(|symbol| !non_terminals.contains(*symbol))
            .cloned()
            .collect();

        for ignored in &self.ignorable {
            if non_terminals.contains(ignored) {
                return Err(BuildError::NonTerminalIgnoredErr(ignored.to_string()));
            }
        }

        Ok(Grammar {
            prods_by_lhs: self.prods_by_lhs,
            nss,
            non_terminals,
            terminals,
            ignorable: self.ignorable,
            start,
            symbol_string: Box::new(|sym| sym.to_string()),
        })
    }
}

pub struct NonTerminalBuilder<'builder, Symbol: Data + Default> {
    grammar_builder: &'builder mut SimpleGrammarBuilder<Symbol>,
    lhs: Symbol,
}

impl<'builder, Symbol: Data + Default> NonTerminalBuilder<'builder, Symbol> {
    fn new(grammar_builder: &'builder mut SimpleGrammarBuilder<Symbol>, lhs: Symbol) -> Self {
        NonTerminalBuilder {
            grammar_builder,
            lhs,
        }
    }

    pub fn to(&mut self, rhs: Vec<Symbol>) -> &mut Self {
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
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            BuildError::NonTerminalIgnoredErr(ref symbol) => {
                write!(f, "Ignored symbol '{}' is non-terminal", symbol)
            }
        }
    }
}

impl error::Error for BuildError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            BuildError::NonTerminalIgnoredErr(_) => None,
        }
    }
}
