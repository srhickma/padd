use {
    core::{data::Data, parse::Production},
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
}

pub struct GrammarBuilder<Symbol: Data + Default> {
    prods_by_lhs: HashMap<Symbol, Vec<Production<Symbol>>>,
    ignorable: HashSet<Symbol>,
    start: Option<Symbol>,
}

impl<Symbol: Data + Default> GrammarBuilder<Symbol> {
    pub fn new() -> Self {
        GrammarBuilder {
            prods_by_lhs: HashMap::new(),
            ignorable: HashSet::new(),
            start: None,
        }
    }

    pub fn add_optional_state(&mut self, opt_state: &Symbol, dest_state: &Symbol) {
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

    pub fn from(&mut self, lhs: Symbol) -> NonTerminalBuilder<Symbol> {
        NonTerminalBuilder::new(self, lhs)
    }

    pub fn add_production(&mut self, production: Production<Symbol>) {
        if let Some(vec) = self.prods_by_lhs.get_mut(&production.lhs) {
            vec.push(production);
            return;
        }

        self.prods_by_lhs
            .insert(production.lhs.clone(), vec![production]);
    }

    #[allow(unused)]
    pub fn add_productions(&mut self, productions: Vec<Production<Symbol>>) {
        for prod in productions {
            self.add_production(prod);
        }
    }

    pub fn try_mark_start(&mut self, start: &Symbol) {
        if self.start.is_some() {
            return;
        }

        self.start = Some(start.clone());
    }

    pub fn mark_ignorable(&mut self, symbol: &Symbol) {
        self.ignorable.insert(symbol.clone());
    }

    pub fn build(self) -> Result<Grammar<Symbol>, BuildError> {
        if self.start.is_none() {
            panic!("No start state specified for grammar");
        }
        let start = self.start.unwrap();

        if self.prods_by_lhs.get(&start).is_none() {
            panic!("Start state has no productions");
        }

        let nss: HashSet<Symbol> = GrammarBuilder::build_nss(&self.prods_by_lhs);

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
        })
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

pub struct NonTerminalBuilder<'builder, Symbol: Data + Default> {
    grammar_builder: &'builder mut GrammarBuilder<Symbol>,
    lhs: Symbol,
}

impl<'builder, Symbol: Data + Default> NonTerminalBuilder<'builder, Symbol> {
    fn new(grammar_builder: &'builder mut GrammarBuilder<Symbol>, lhs: Symbol) -> Self {
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
