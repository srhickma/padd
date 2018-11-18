use std::collections::HashMap;
use std::collections::HashSet;

use core::parse::Production;

pub struct Grammar {
    prods_by_lhs: HashMap<String, Vec<Production>>,
    nss: HashSet<String>,
    #[allow(dead_code)]
    non_terminals: HashSet<String>,
    terminals: HashSet<String>,
    start: String,
}

impl Grammar {
    pub fn is_nullable(&self, prod: &Production) -> bool {
        self.nss.contains(&prod.lhs)
    }

    pub fn is_terminal(&self, symbol: &str) -> bool {
        self.terminals.contains(symbol)
    }

    pub fn start(&self) -> &String {
        &self.start
    }

    pub fn productions_for_lhs(&self, lhs: &str) -> Option<&Vec<Production>> {
        self.prods_by_lhs.get(lhs)
    }
}

pub struct GrammarBuilder {
    prods_by_lhs: HashMap<String, Vec<Production>>,
    start: Option<String>,
}

impl GrammarBuilder {
    pub fn new() -> Self {
        GrammarBuilder {
            prods_by_lhs: HashMap::new(),
            start: None,
        }
    }

    pub fn add_optional_state(&mut self, dest: &str) -> String {
        let opt_state: String = format!("opt#{}", dest);

        if !self.prods_by_lhs.contains_key(&opt_state) {
            self.prods_by_lhs.entry(opt_state.clone())
                .or_insert(vec![
                    Production {
                        lhs: opt_state.clone(),
                        rhs: vec![String::from(dest)],
                    },
                    Production {
                        lhs: opt_state.clone(),
                        rhs: Vec::new(),
                    }
                ]);
        }

        opt_state
    }

    pub fn add_production(&mut self, production: Production) {
        if let Some(vec) = self.prods_by_lhs.get_mut(&production.lhs[..]) {
            vec.push(production);
            return;
        }

        self.prods_by_lhs.insert(production.lhs.clone(), vec![production]);
    }

    pub fn add_productions(&mut self, productions: Vec<Production>) {
        for prod in productions {
            self.add_production(prod);
        }
    }

    pub fn try_mark_start(&mut self, start: &str) {
        if self.start.is_some() {
            return;
        }

        self.start = Some(start.to_string());
    }

    pub fn build(self) -> Grammar {
        if self.start.is_none() {
            panic!("No start state specified for grammar");
        }
        let start = self.start.unwrap();

        if self.prods_by_lhs.get(&start).is_none() {
            panic!("Start state has no productions");
        }

        let nss: HashSet<String> = GrammarBuilder::build_nss(&self.prods_by_lhs);

        let non_terminals: HashSet<String> = self.prods_by_lhs.iter()
            .map(|(lhs, _)| lhs)
            .cloned()
            .collect();

        let terminals: HashSet<String> = self.prods_by_lhs.iter()
            .flat_map(|(_, prods)| prods)
            .flat_map(|prod| &prod.rhs)
            .filter(|symbol| !non_terminals.contains(*symbol))
            .cloned()
            .collect();

        Grammar {
            prods_by_lhs: self.prods_by_lhs,
            nss,
            non_terminals,
            terminals,
            start,
        }
    }

    fn build_nss(prods_by_lhs: &HashMap<String, Vec<Production>>) -> HashSet<String> {
        let mut nss: HashSet<String> = HashSet::new();
        let mut prods_by_rhs: HashMap<&String, Vec<&Production>> = HashMap::new();
        let mut work_stack: Vec<&String> = Vec::new();

        prods_by_lhs.iter()
            .flat_map(|(_, prods)| prods)
            .for_each(|prod| {
                for s in &prod.rhs {
                    prods_by_rhs.entry(s)
                        .or_insert(Vec::new())
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
                    match prods_by_rhs.get(work_symbol) {
                        None => {}
                        Some(prods) => {
                            for prod in prods {
                                if !nss.contains(&prod.lhs)
                                    && prod.rhs.iter().all(|sym| nss.contains(sym)) {
                                    nss.insert(prod.lhs.clone());
                                    work_stack.push(&prod.lhs);
                                }
                            }
                        }
                    }
                }
            };
        }

        nss
    }
}
