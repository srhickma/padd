use {
    core::{
        fmt::pattern::{Capture, Pattern, Segment},
        parse::{grammar::GrammarSymbol, Production, Tree},
    },
    std::collections::HashMap,
};

mod pattern;

pub struct Formatter<Symbol: GrammarSymbol> {
    pattern_map: HashMap<Production<Symbol>, Pattern>,
}

impl<Symbol: GrammarSymbol> Formatter<Symbol> {
    pub fn format(&self, parse: &Tree<Symbol>) -> String {
        let format_job = FormatJob {
            parse,
            pattern_map: &self.pattern_map,
        };
        format_job.run()
    }
}

pub struct FormatterBuilder<Symbol: GrammarSymbol> {
    pattern_map: HashMap<Production<Symbol>, Pattern>,
    memory: HashMap<String, Pattern>,
}

impl<Symbol: GrammarSymbol> FormatterBuilder<Symbol> {
    pub fn new() -> FormatterBuilder<Symbol> {
        FormatterBuilder {
            pattern_map: HashMap::new(),
            memory: HashMap::new(),
        }
    }

    pub fn build(self) -> Formatter<Symbol> {
        Formatter {
            pattern_map: self.pattern_map,
        }
    }

    pub fn add_pattern(&mut self, pair: PatternPair<Symbol>) -> Result<(), BuildError> {
        if let Some(pattern) = self.memory.get(&pair.pattern) {
            self.pattern_map.insert(pair.production, pattern.clone());
            return Ok(());
        }

        let pattern = pattern::generate_pattern(
            &pair.pattern[..],
            &pair.production,
            &pair.string_production,
        )?;
        self.memory.insert(pair.pattern, pattern.clone());
        self.pattern_map.insert(pair.production, pattern);
        Ok(())
    }
}

pub type BuildError = pattern::BuildError;

struct FormatJob<'parse, Symbol: GrammarSymbol + 'parse> {
    parse: &'parse Tree<Symbol>,
    pattern_map: &'parse HashMap<Production<Symbol>, Pattern>,
}

impl<'parse, Symbol: GrammarSymbol + 'parse> FormatJob<'parse, Symbol> {
    fn run(&self) -> String {
        self.recur(self.parse, &HashMap::new())
    }

    #[inline(always)]
    fn recur(&self, node: &Tree<Symbol>, scope: &HashMap<String, String>) -> String {
        if node.is_leaf() {
            if node.is_null() {
                return String::new();
            }
            return node.lhs.lexeme().clone();
        }

        let pattern = self.pattern_map.get(&node.production());
        match pattern {
            Some(ref p) => self.fill_pattern(p, &node.children, scope),
            None => {
                //Reconstruct one after the other
                let mut res = String::new();
                for child in &node.children {
                    res = format!("{}{}", res, self.recur(child, scope));
                }
                res
            }
        }
    }

    #[inline(always)]
    fn fill_pattern(
        &self,
        pattern: &Pattern,
        children: &[Tree<Symbol>],
        scope: &HashMap<String, String>,
    ) -> String {
        let mut res: String = String::new();
        for seg in &pattern.segments {
            match *seg {
                Segment::Filler(ref s) => res = format!("{}{}", res, s),
                Segment::Substitution(ref s) => {
                    if let Some(value) = scope.get(s) {
                        res = format!("{}{}", res, value);
                    }
                }
                Segment::Capture(ref c) => {
                    res = format!("{}{}", res, self.evaluate_capture(c, children, scope))
                }
            };
        }
        res
    }

    #[inline(always)]
    fn evaluate_capture(
        &self,
        capture: &Capture,
        children: &[Tree<Symbol>],
        outer_scope: &HashMap<String, String>,
    ) -> String {
        if !capture.declarations.is_empty() {
            let mut inner_scope = outer_scope.clone();
            for decl in &capture.declarations {
                match decl.value {
                    Some(ref pattern) => {
                        inner_scope.insert(
                            decl.key.clone(),
                            self.fill_pattern(pattern, children, outer_scope),
                        );
                    }
                    None => {
                        inner_scope.remove(&decl.key);
                    }
                }
            }

            match children.get(capture.child_index) {
                Some(child) => self.recur(child, &inner_scope),
                None => panic!(
                    "Pattern index out of bounds: index={} children={}",
                    capture.child_index,
                    children.len()
                ),
            }
        } else {
            match children.get(capture.child_index) {
                Some(child) => self.recur(child, outer_scope),
                None => panic!(
                    "Pattern index out of bounds: index={} children={}",
                    capture.child_index,
                    children.len()
                ),
            }
        }
    }
}

pub struct PatternPair<Symbol: GrammarSymbol> {
    pub production: Production<Symbol>,
    pub string_production: Production<String>,
    pub pattern: String,
}
