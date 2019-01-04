use {
    core::{
        data::Data,
        fmt::pattern::{Capture, Pattern, Segment},
        parse::{Production, Tree},
    },
    std::collections::HashMap,
};

mod pattern;

pub struct Formatter {
    pattern_map: HashMap<String, Pattern>,
}

impl Formatter {
    pub fn format<Symbol: Data + Default>(&self, parse: &Tree<Symbol>) -> String {
        let format_job = FormatJob {
            parse,
            pattern_map: &self.pattern_map,
        };
        format_job.run()
    }
}

pub struct FormatterBuilder {
    pattern_map: HashMap<String, Pattern>,
    memory: HashMap<String, Pattern>,
}

impl FormatterBuilder {
    pub fn new() -> FormatterBuilder {
        FormatterBuilder {
            pattern_map: HashMap::new(),
            memory: HashMap::new(),
        }
    }

    pub fn build(self) -> Formatter {
        Formatter {
            pattern_map: self.pattern_map
        }
    }

    pub fn add_pattern<Symbol: Data + Default>(
        &mut self,
        pair: PatternPair<Symbol>
    ) -> Result<(), BuildError> {
        let key = pair.production.to_string();

        if let Some(pattern) = self.memory.get(&pair.pattern) {
            self.pattern_map.insert(key, pattern.clone());
            return Ok(());
        }

        let pattern = pattern::generate_pattern(&pair.pattern[..], &pair.production)?;
        self.memory.insert(pair.pattern, pattern.clone());
        self.pattern_map.insert(key, pattern);
        Ok(())
    }
}

pub type BuildError = pattern::BuildError;

struct FormatJob<'parse, Symbol: Data + Default + 'parse> {
    parse: &'parse Tree<Symbol>,
    pattern_map: &'parse HashMap<String, Pattern>,
}

impl<'parse, Symbol: Data + Default + 'parse> FormatJob<'parse, Symbol> {
    fn run(&self) -> String {
        self.recur(self.parse, &HashMap::new())
    }

    fn recur(&self, node: &Tree<Symbol>, scope: &HashMap<String, String>) -> String {
        if node.is_leaf() {
            if node.is_null() {
                return String::new();
            }
            return node.lhs.lexeme().clone();
        }

        let pattern = self.pattern_map.get(&node.production()[..]);
        match pattern {
            Some(ref p) => self.fill_pattern(p, &node.children, scope),
            None => { //Reconstruct one after the other
                let mut res = String::new();
                for child in &node.children {
                    res = format!("{}{}", res, self.recur(child, scope));
                }
                res
            }
        }
    }

    fn fill_pattern(
        &self,
        pattern: &Pattern,
        children: &Vec<Tree<Symbol>>,
        scope: &HashMap<String, String>,
    ) -> String {
        let mut res: String = String::new();
        for seg in &pattern.segments {
            match seg {
                &Segment::Filler(ref s) => res = format!("{}{}", res, s),
                &Segment::Substitution(ref s) => if let Some(value) = scope.get(s) {
                    res = format!("{}{}", res, value);
                },
                &Segment::Capture(ref c) => res = format!(
                    "{}{}", res, self.evaluate_capture(c, children, scope)
                ),
            };
        }
        res
    }

    fn evaluate_capture(
        &self,
        capture: &Capture,
        children: &Vec<Tree<Symbol>>,
        outer_scope: &HashMap<String, String>,
    ) -> String {
        if capture.declarations.len() > 0 {
            let mut inner_scope = outer_scope.clone();
            for decl in &capture.declarations {
                match &decl.value {
                    &Some(ref pattern) => {
                        inner_scope.insert(
                            decl.key.clone(),
                            self.fill_pattern(pattern, children, outer_scope),
                        );
                    }
                    &None => {
                        inner_scope.remove(&decl.key);
                    }
                }
            }

            match children.get(capture.child_index) {
                Some(child) => self.recur(child, &inner_scope),
                None => panic!("Pattern index out of bounds: index={} children={}", capture.child_index, children.len()),
            }
        } else {
            match children.get(capture.child_index) {
                Some(child) => self.recur(child, outer_scope),
                None => panic!("Pattern index out of bounds: index={} children={}", capture.child_index, children.len()),
            }
        }
    }
}

pub struct PatternPair<Symbol: Data + Default> {
    pub production: Production<Symbol>,
    pub pattern: String,
}
