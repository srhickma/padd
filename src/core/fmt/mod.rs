use {
    core::{
        data::map::CEHashMap,
        fmt::pattern::{Capture, Pattern, Segment},
        parse::{grammar::GrammarSymbol, Production, Tree},
    },
    std::{collections::HashMap, error, fmt},
};

mod pattern;

pub struct Formatter<Symbol: GrammarSymbol> {
    pattern_map: HashMap<Production<Symbol>, Pattern>,
    injection_map: HashMap<Symbol, Injectable>,
}

impl<Symbol: GrammarSymbol> Formatter<Symbol> {
    pub fn format(&self, parse: &Tree<Symbol>) -> String {
        let format_job = FormatJob {
            parse,
            pattern_map: &self.pattern_map,
            injection_map: &self.injection_map,
        };
        format_job.run()
    }
}

pub struct FormatterBuilder<Symbol: GrammarSymbol> {
    pattern_map: HashMap<Production<Symbol>, Pattern>,
    injection_map: HashMap<Symbol, Injectable>,
    memory: HashMap<String, Pattern>,
}

impl<Symbol: GrammarSymbol> FormatterBuilder<Symbol> {
    pub fn new() -> FormatterBuilder<Symbol> {
        FormatterBuilder {
            pattern_map: HashMap::new(),
            injection_map: HashMap::new(),
            memory: HashMap::new(),
        }
    }

    pub fn build(self) -> Formatter<Symbol> {
        Formatter {
            pattern_map: self.pattern_map,
            injection_map: self.injection_map,
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

    pub fn add_injection(&mut self, injection: InjectableString<Symbol>) -> Result<(), BuildError> {
        if self.injection_map.contains_key(&injection.terminal) {
            return Err(BuildError::DuplicateInjectionErr(injection.terminal_string));
        }

        let pattern = match injection.pattern_string {
            None => None,
            Some(pattern_string) => {
                let production =
                    Production::from(injection.terminal.clone(), vec![injection.terminal.clone()]);

                Some(pattern::generate_pattern(
                    &pattern_string,
                    &production,
                    &production.string_production(),
                )?)
            }
        };

        self.injection_map.insert(
            injection.terminal,
            Injectable {
                pattern,
                affinity: injection.affinity,
            },
        );

        Ok(())
    }
}

#[derive(Debug)]
pub enum BuildError {
    PatternBuildErr(pattern::BuildError),
    DuplicateInjectionErr(String),
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            BuildError::PatternBuildErr(ref err) => write!(f, "Pattern build error: {}", err),
            BuildError::DuplicateInjectionErr(ref symbol) => write!(
                f,
                "Injection specified multiple times for symbol '{}'",
                symbol
            ),
        }
    }
}

impl error::Error for BuildError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            BuildError::PatternBuildErr(ref err) => Some(err),
            BuildError::DuplicateInjectionErr(_) => None,
        }
    }
}

impl From<pattern::BuildError> for BuildError {
    fn from(err: pattern::BuildError) -> BuildError {
        BuildError::PatternBuildErr(err)
    }
}

struct Injection<'scope, Symbol: GrammarSymbol> {
    tree: &'scope Tree<Symbol>,
    pattern: &'scope Option<Pattern>,
    direction: InjectionAffinity,
}

struct FormatJob<'parse, Symbol: GrammarSymbol + 'parse> {
    parse: &'parse Tree<Symbol>,
    pattern_map: &'parse HashMap<Production<Symbol>, Pattern>,
    injection_map: &'parse HashMap<Symbol, Injectable>,
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

        let pattern = match node.production {
            Some(ref production) => self.pattern_map.get(production),
            None => None,
        };

        match pattern {
            Some(ref p) => self.fill_pattern_outer(p, &node.children, scope),
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
    fn fill_pattern_outer(
        &self,
        pattern: &Pattern,
        children: &[Tree<Symbol>],
        scope: &HashMap<String, String>,
    ) -> String {
        let mut captured_children: CEHashMap<()> = CEHashMap::new();
        for seg in &pattern.segments {
            if let Segment::Capture(ref c) = seg {
                captured_children.insert(c.child_index, ());
            }
        }

        let mut injections_by_node: CEHashMap<Vec<Injection<Symbol>>> = CEHashMap::new();
        let mut expected_children: Vec<&Tree<Symbol>> = Vec::new();

        for child in children {
            if child.injected {
                let injectable = &self.injection_map[child.lhs.kind()];

                let mut target: i64 = match &injectable.affinity {
                    InjectionAffinity::Left => expected_children.len() as i64 - 1,
                    InjectionAffinity::Right => expected_children.len() as i64,
                };

                let direction = if target < 0 || !captured_children.contains(target as usize) {
                    // Can't inject in the preferred direction, so flip
                    target += match &injectable.affinity {
                        InjectionAffinity::Left => 1,
                        InjectionAffinity::Right => -1,
                    };

                    injectable.affinity.opposite()
                } else {
                    injectable.affinity.clone()
                };

                if target < 0 || !captured_children.contains(target as usize) {
                    // Can't inject in either direction, so skip
                    continue;
                }
                let target = target as usize;

                let injection = Injection {
                    tree: child,
                    pattern: &injectable.pattern,
                    direction,
                };

                if !injections_by_node.contains(target) {
                    injections_by_node.insert(target, vec![injection]);
                } else {
                    injections_by_node.get_mut(target).unwrap().push(injection);
                }
            } else {
                expected_children.push(child)
            }
        }

        self.fill_pattern_inner(
            pattern,
            &expected_children[..],
            scope,
            Some(injections_by_node),
        )
    }

    #[inline(always)]
    fn fill_pattern_inner(
        &self,
        pattern: &Pattern,
        children: &[&Tree<Symbol>],
        scope: &HashMap<String, String>,
        mut injections_by_node_opt: Option<CEHashMap<Vec<Injection<Symbol>>>>,
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
                    let injections_opt = match &mut injections_by_node_opt {
                        Some(injections_by_node) => injections_by_node.remove(c.child_index),
                        None => None,
                    };

                    res = format!(
                        "{}{}",
                        res,
                        self.evaluate_capture(c, children, scope, &injections_opt)
                    );
                }
            };
        }

        res
    }

    #[inline(always)]
    fn evaluate_capture(
        &self,
        capture: &Capture,
        children: &[&Tree<Symbol>],
        outer_scope: &HashMap<String, String>,
        injections_opt: &Option<Vec<Injection<Symbol>>>,
    ) -> String {
        if !capture.declarations.is_empty() {
            let mut inner_scope = outer_scope.clone();
            for decl in &capture.declarations {
                match decl.value {
                    Some(ref pattern) => {
                        inner_scope.insert(
                            decl.key.clone(),
                            self.fill_pattern_inner(pattern, children, outer_scope, None),
                        );
                    }
                    None => {
                        inner_scope.remove(&decl.key);
                    }
                }
            }

            self.evaluate_capture_internal(
                capture,
                children,
                outer_scope,
                &inner_scope,
                injections_opt,
            )
        } else {
            self.evaluate_capture_internal(
                capture,
                children,
                outer_scope,
                outer_scope,
                injections_opt,
            )
        }
    }

    #[inline(always)]
    fn evaluate_capture_internal(
        &self,
        capture: &Capture,
        children: &[&Tree<Symbol>],
        outer_scope: &HashMap<String, String>,
        inner_scope: &HashMap<String, String>,
        injections_opt: &Option<Vec<Injection<Symbol>>>,
    ) -> String {
        let child = children[capture.child_index];
        let child_string = self.recur(child, &inner_scope);
        let mut prefix = String::new();
        let mut postfix = String::new();

        if let Some(injections) = injections_opt {
            for injection in injections.iter().rev() {
                let injection_string = match injection.pattern {
                    Some(ref pattern) => {
                        self.fill_pattern_inner(pattern, &[injection.tree], outer_scope, None)
                    }
                    None => injection.tree.lhs.lexeme().clone(),
                };

                match injection.direction {
                    InjectionAffinity::Left => postfix = format!("{}{}", postfix, injection_string),
                    InjectionAffinity::Right => prefix = format!("{}{}", injection_string, prefix),
                }
            }
        }

        format!("{}{}{}", prefix, child_string, postfix)
    }
}

pub struct PatternPair<Symbol: GrammarSymbol> {
    pub production: Production<Symbol>,
    pub string_production: Production<String>,
    pub pattern: String,
}

#[derive(Clone, PartialEq)]
pub enum InjectionAffinity {
    Left,
    Right,
}

impl InjectionAffinity {
    fn opposite(&self) -> Self {
        match self {
            InjectionAffinity::Left => InjectionAffinity::Right,
            InjectionAffinity::Right => InjectionAffinity::Left,
        }
    }
}

pub struct Injectable {
    pattern: Option<Pattern>,
    affinity: InjectionAffinity,
}

pub struct InjectableString<Symbol: GrammarSymbol> {
    pub terminal: Symbol,
    pub terminal_string: String,
    pub pattern_string: Option<String>,
    pub affinity: InjectionAffinity,
}
