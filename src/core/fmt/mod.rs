use {
    core::{
        data::map::CEHashMap,
        fmt::pattern::{Capture, Pattern, Segment},
        parse::{grammar::GrammarSymbol, Production, ProductionSymbol, Tree},
    },
    std::{collections::HashMap, error, fmt},
};

mod pattern;

/// Formatter: A utility struct used to format parse trees based on a set of `Pattern` objects.
///
/// # Type Parameters
///
/// * `Symbol` - the symbol type of the parse tree's to be formatted.
///
/// # Fields
///
/// * `pattern_map` - a map from productions to their respective patterns.
/// * `injection_map` - a map from grammar symbols to their respective injectables, used to format
/// injected symbols.
pub struct Formatter<Symbol: GrammarSymbol> {
    pattern_map: HashMap<Production<Symbol>, Pattern>,
    injection_map: HashMap<Symbol, Injectable>,
}

impl<Symbol: GrammarSymbol> Formatter<Symbol> {
    /// Returns the formatted string for the given parse tree.
    pub fn format(&self, parse: &Tree<Symbol>) -> String {
        let format_job = FormatJob {
            parse,
            pattern_map: &self.pattern_map,
            injection_map: &self.injection_map,
        };
        format_job.run()
    }
}

/// Formatter Builder: A builder for efficiently constructing `Formatter` structs.
///
/// # Type Parameters
///
/// * `Symbol` - the symbol type of the parse tree's to be formatted.
///
/// # Fields
///
/// * `pattern_map` - a map from productions to their respective patterns.
/// * `injection_map` - a map from grammar symbols to their respective injectables, used to format
/// injected symbols.
/// * `memory` - a map from pattern strings to `Pattern` objects, so that parsing can be skipped for
/// pattern strings that have already been seen elsewhere in the specification.
pub struct FormatterBuilder<Symbol: GrammarSymbol> {
    pattern_map: HashMap<Production<Symbol>, Pattern>,
    injection_map: HashMap<Symbol, Injectable>,
    memory: HashMap<String, Pattern>,
}

impl<Symbol: GrammarSymbol> FormatterBuilder<Symbol> {
    /// Returns a new `FormatterBuilder`.
    pub fn new() -> FormatterBuilder<Symbol> {
        FormatterBuilder {
            pattern_map: HashMap::new(),
            injection_map: HashMap::new(),
            memory: HashMap::new(),
        }
    }

    /// Builds a `Formatter` from this `FormatterBuilder`, consuming the builder.
    pub fn build(self) -> Formatter<Symbol> {
        Formatter {
            pattern_map: self.pattern_map,
            injection_map: self.injection_map,
        }
    }

    /// Adds a pattern to the formatter for a specific production.
    ///
    /// Returns an error if the passed pattern string cannot be built into a `Pattern`.
    ///
    /// # Parameters
    ///
    /// * `pair` - the `PatternPair` storing the pattern string and production.
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

    /// Adds a injectable specification to the formatter for a specific symbol.
    ///
    /// Returns an error if the injection pattern string cannot be built into a `Pattern`.
    ///
    /// # Parameters
    ///
    /// * `injection` - the injection specification to be added.
    pub fn add_injection(&mut self, injection: InjectableString<Symbol>) -> Result<(), BuildError> {
        if self.injection_map.contains_key(&injection.terminal) {
            return Err(BuildError::DuplicateInjectionErr(injection.terminal_string));
        }

        let pattern = match injection.pattern_string {
            None => None,
            Some(pattern_string) => {
                let production = Production::from(
                    injection.terminal.clone(),
                    vec![ProductionSymbol::symbol(injection.terminal.clone())],
                );

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

/// Build Error: Represents an error encountered while building a formatter.
///
/// # Types
///
/// * `PatternBuildErr` - Indicates that an error that occurred while building a pattern.
/// * `DuplicateInjectionErr` - Indicates that a particular symbol was specified for injection
/// more than once.
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

/// Injection: A struct representing the injection of a symbol before or after a non-injected tree.
///
/// # Type Parameters
///
/// * `Symbol` - the symbol type of the grammar used to construct the parse tree (and the symbol
/// being injected).
///
/// # Fields
///
/// * `tree` - the parse tree of the node which this injection will attach to.
/// * `pattern` - the pattern used to format the injected string.
/// * `direction` - the affinity of the injection, used to specify whether the result of evaluating
/// `pattern` should be prepended or appended to the formatted result of `tree`.
struct Injection<'scope, Symbol: GrammarSymbol> {
    tree: &'scope Tree<Symbol>,
    pattern: &'scope Option<Pattern>,
    direction: InjectionAffinity,
}

/// Format Job: A payload struct for the formatter which stores the parse tree being formatted and
/// all other data needed to carry out the formatting.
///
/// # Type Parameters
///
/// * `Symbol` - the symbol type of the grammar used to construct the parse tree.
///
/// # Fields
///
/// * `parse` - the parse tree to be formatted.
/// * `pattern_map` - a map from productions to their respective patterns.
/// * `injection_map` - a map from grammar symbols to their respective injectables, used to format
/// injected symbols.
struct FormatJob<'parse, Symbol: GrammarSymbol + 'parse> {
    parse: &'parse Tree<Symbol>,
    pattern_map: &'parse HashMap<Production<Symbol>, Pattern>,
    injection_map: &'parse HashMap<Symbol, Injectable>,
}

impl<'parse, Symbol: GrammarSymbol + 'parse> FormatJob<'parse, Symbol> {
    /// Runs the formatter on this `FormatJob`.
    ///
    /// Returns the formatted string.
    fn run(&self) -> String {
        self.recur(self.parse, &HashMap::new())
    }

    /// Returns the formatted string of the passed parse tree node.
    ///
    /// # Parameters
    ///
    /// * `node` - the current parse tree node.
    /// * `scope` - a hashmap storing the values of variables in the scope of the current node,
    /// indexed by the variable names.
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
                // Reconstruct one after the other
                let mut res = String::new();
                for child in &node.children {
                    res = if child.injected {
                        let injectable = &self.injection_map[child.lhs.kind()];
                        let injection = Injection {
                            tree: child,
                            pattern: &injectable.pattern,
                            direction: injectable.affinity.clone(),
                        };

                        format!("{}{}", res, self.injection_string(&injection, scope))
                    } else {
                        format!("{}{}", res, self.recur(child, scope))
                    };
                }
                res
            }
        }
    }

    /// Returns the formatted string for a pattern given a set of child nodes.
    ///
    /// Injections are filtered and directed at this stage of formatting. Injections are paired
    /// with their neighbours based on their preferred affinity, or their non-preferred neighbour
    /// if their preferred neighbour is not captured by the pattern (or doesn't exist).
    ///
    /// # Parameters
    ///
    /// * `pattern` - the pattern to be used when formatting the children.
    /// * `children` - a slice of parse tree nodes storing the children of the current (pattern)
    /// node.
    /// * `scope` - a hashmap storing the values of variables in the scope of the current node,
    /// indexed by the variable names.
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

    /// Returns the formatted string for a pattern given a set of child nodes.
    ///
    /// Injections can optionally be provided, in which case they will be consumed and passed to
    /// capture evaluation to be injected.
    ///
    /// # Parameters
    ///
    /// * `pattern` - the pattern to be used when formatting the children.
    /// * `children` - a slice of parse tree nodes storing the children of the current (pattern)
    /// node.
    /// * `scope` - a hashmap storing the values of variables in the scope of the current node,
    /// indexed by the variable names.
    /// * `injections_by_node_opt` - an optional map from child index (in `children`) to a vector
    /// of injections that should be performed for that child node.
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

    /// Returns the formatted string for a pattern capture given a set of child nodes.
    ///
    /// This method determines the inner scope of the capture, and then calls
    /// `evaluate_capture_internal` to perform the actual evaluation.
    ///
    /// # Parameters
    ///
    /// * `capture` - the pattern capture to be "filled".
    /// * `children` - a slice of parse tree nodes storing the children of the current (pattern)
    /// node.
    /// * `outer_scope` - a hashmap storing the values of variables in the scope of the current
    /// node, indexed by the variable names.
    /// * `injections_opt` - An optional vector of injections to perform during this capture
    /// evaluation.
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
                children[capture.child_index],
                outer_scope,
                &inner_scope,
                injections_opt,
            )
        } else {
            self.evaluate_capture_internal(
                children[capture.child_index],
                outer_scope,
                outer_scope,
                injections_opt,
            )
        }
    }

    /// Returns the formatted string for a pattern capture given the associated child node.
    ///
    /// This method calls `recur` with the inner scope of the capture to build the formatted string
    /// of the child node, then prefixes and postfixes the string with the injection strings of all
    /// passed injections.
    ///
    /// # Parameters
    ///
    /// * `child` - the child node to being captured.
    /// * `outer_scope` - a hashmap storing the values of variables in the scope of the parent
    /// node, indexed by the variable names.
    /// * `inner_scope` - a hashmap storing the values of variables in the scope of the child
    /// node, indexed by the variable names.
    /// * `injections_opt` - An optional vector of injections to perform during this capture
    /// evaluation.
    #[inline(always)]
    fn evaluate_capture_internal(
        &self,
        child: &Tree<Symbol>,
        outer_scope: &HashMap<String, String>,
        inner_scope: &HashMap<String, String>,
        injections_opt: &Option<Vec<Injection<Symbol>>>,
    ) -> String {
        let child_string = self.recur(child, &inner_scope);
        let mut prefix = String::new();
        let mut postfix = String::new();

        if let Some(injections) = injections_opt {
            injections
                .iter()
                .filter(|injection| injection.direction == InjectionAffinity::Left)
                .for_each(|injection| {
                    let injection_string = self.injection_string(injection, outer_scope);
                    postfix = format!("{}{}", postfix, injection_string);
                });

            injections
                .iter()
                .filter(|injection| injection.direction == InjectionAffinity::Right)
                .rev()
                .for_each(|injection| {
                    let injection_string = self.injection_string(injection, outer_scope);
                    prefix = format!("{}{}", injection_string, prefix);
                });
        }

        format!("{}{}{}", prefix, child_string, postfix)
    }

    /// Returns the formatted string for an injection.
    ///
    /// # Parameters
    ///
    /// * `injection` - the injection to be formatted.
    /// * `scope` - the variable scope in which to format the injection, indexed by variable names.
    #[inline(always)]
    fn injection_string(
        &self,
        injection: &Injection<Symbol>,
        scope: &HashMap<String, String>,
    ) -> String {
        match injection.pattern {
            Some(ref pattern) => self.fill_pattern_inner(pattern, &[injection.tree], scope, None),
            None => injection.tree.lhs.lexeme().clone(),
        }
    }
}

/// Pattern Pair: A pair representing an un-parsed pattern.
///
/// # Type Parameters
///
/// * `Symbol` - the symbol type of the pattern's production.
///
/// # Fields
///
/// * `production` - the grammar production which this pattern applies to.
/// * `string_production` - the string representation of `production`, useful for building errors.
/// * `pattern` - the raw pattern string, retrieved from the specification.
pub struct PatternPair<Symbol: GrammarSymbol> {
    pub production: Production<Symbol>,
    pub string_production: Production<String>,
    pub pattern: String,
}

/// Injection Affinity: Represents the direction in which an injection is preferred.
///
/// # Types
///
/// * `Left` - Indicates that a symbol prefers to be injected immediately after the previous
/// non-terminal symbol.
/// * `Right` - Indicates that a symbol prefers to be injected immediately before the next
/// non-terminal symbol.
#[derive(Clone, PartialEq)]
pub enum InjectionAffinity {
    Left,
    Right,
}

impl InjectionAffinity {
    /// Returns the opposite injection affinity.
    fn opposite(&self) -> Self {
        match self {
            InjectionAffinity::Left => InjectionAffinity::Right,
            InjectionAffinity::Right => InjectionAffinity::Left,
        }
    }
}

/// Injectable: Internal representation of an injection specification.
///
/// Note: the symbol itself is not stored in an `Injectable`, since it will already be stored as a
/// key in the injection map.
///
/// # Fields
///
/// * `pattern` - an optional pattern to use when formatting the injection string.
/// * `affinity` - the affinity of the injection.
struct Injectable {
    pattern: Option<Pattern>,
    affinity: InjectionAffinity,
}

/// Injectable String: External representation of an injection specification.
///
/// # Type Parameters
///
/// * `Symbol` - the symbol type of the terminal being considered for injection.
///
/// # Fields
///
/// * `terminal` - the terminal symbol being considered for injection.
/// * `terminal_string` - the string representation of `terminal`, useful for building errors.
/// * `pattern_string` - the pattern string to use when formatting any injections of `terminal`.
/// * `affinity` - the injection affinity of `terminal`.
pub struct InjectableString<Symbol: GrammarSymbol> {
    pub terminal: Symbol,
    pub terminal_string: String,
    pub pattern_string: Option<String>,
    pub affinity: InjectionAffinity,
}
