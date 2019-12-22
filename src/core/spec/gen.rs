use {
    core::{
        fmt::{FormatterBuilder, InjectableString, InjectionAffinity, PatternPair},
        lex::{
            ecdfa::{EncodedCDFA, EncodedCDFABuilder},
            CDFABuilder, ConsumerStrategy, TransitBuilder, CDFA,
        },
        parse::{
            grammar::{Grammar, GrammarBuilder, GrammarSymbol},
            Production, ProductionSymbol, Tree,
        },
        spec::{
            self,
            lang::SpecSymbol,
            region::{self, RegionType},
            SpecGenResult,
        },
        util::string_utils,
    },
    std::collections::HashSet,
};

/// Builds a specification from a parse of the specification grammar.
///
/// Returns the specification if successful, otherwise an error.
///
/// # Parameters
///
/// * `parse` - the parse tree generated for the specification.
/// * `grammar_builder` - a grammar builder with which to construct the specification's grammar.
pub fn generate_spec<Symbol: 'static + GrammarSymbol, GrammarType, GrammarBuilderType>(
    parse: &Tree<SpecSymbol>,
    mut grammar_builder: GrammarBuilderType,
) -> Result<SpecGenResult<Symbol>, spec::GenError>
where
    GrammarType: 'static + Grammar<Symbol>,
    GrammarBuilderType: GrammarBuilder<String, Symbol, GrammarType>,
{
    let mut ecdfa_builder: EncodedCDFABuilder<String, Symbol> = EncodedCDFABuilder::new();
    let mut formatter_builder = FormatterBuilder::new();

    traverse_spec_regions(
        parse.get_child(0),
        &mut ecdfa_builder,
        &mut grammar_builder,
        &mut formatter_builder,
    )?;

    let ecdfa = ecdfa_builder.build()?;
    let grammar = grammar_builder.build()?;

    orphan_check(&ecdfa, &grammar)?;

    Ok((
        Box::new(ecdfa),
        Box::new(grammar),
        formatter_builder.build(),
    ))
}

/// Recursively traverses the different regions of a specification parse, and calls the associated
/// region-specific handlers to traverse further.
///
/// An error is returned if the traversal of any specification region results in an error.
///
/// # Parameters
///
/// * `regions_node` - the root `SpecSymbol::Regions` node of the parse tree.
/// * `cdfa_builder` - the CDFA builder for the specification.
/// * `grammar_builder` - the grammar builder for the specification.
/// * `formatter_builder` - the formatter builder for the specification.
fn traverse_spec_regions<CDFABuilderType, CDFAType, Symbol: GrammarSymbol, GrammarType>(
    regions_node: &Tree<SpecSymbol>,
    cdfa_builder: &mut CDFABuilderType,
    grammar_builder: &mut dyn GrammarBuilder<String, Symbol, GrammarType>,
    formatter_builder: &mut FormatterBuilder<Symbol>,
) -> Result<(), spec::GenError>
where
    CDFAType: CDFA<usize, Symbol>,
    CDFABuilderType: CDFABuilder<String, Symbol, CDFAType>,
    GrammarType: Grammar<Symbol>,
{
    let mut region_handler = |inner_node: &Tree<SpecSymbol>, region_type: &RegionType| {
        match region_type {
            RegionType::Injectable => {
                traverse_injectable_region(inner_node, grammar_builder, formatter_builder)?
            }
            RegionType::Ignorable => traverse_ignorable_region(inner_node, grammar_builder),
            RegionType::Alphabet => traverse_alphabet_region(inner_node, cdfa_builder),
            RegionType::CDFA => traverse_cdfa_region(inner_node, cdfa_builder, grammar_builder)?,
            RegionType::Grammar => {
                traverse_grammar_region(inner_node, grammar_builder, formatter_builder)?
            }
        }

        Ok(())
    };

    region::traverse(regions_node, &mut region_handler)
}

/// Traverses an injectable symbol specification region, marking the associated terminal symbol as
/// injectable in the grammar, parsing its pattern, and storing the injection in the formatter.
///
/// An error is returned if the injectable region conflicts with the existing grammar or formatter
/// specifications.
///
/// # Parameters
///
/// * `injectable_node` - the `SpecSymbol::Injectable` being traversed.
/// * `grammar_builder` - the grammar builder for the specification.
/// * `formatter_builder` - the formatter builder for the specification.
fn traverse_injectable_region<Symbol: GrammarSymbol, GrammarType>(
    injectable_node: &Tree<SpecSymbol>,
    grammar_builder: &mut dyn GrammarBuilder<String, Symbol, GrammarType>,
    formatter_builder: &mut FormatterBuilder<Symbol>,
) -> Result<(), spec::GenError>
where
    GrammarType: Grammar<Symbol>,
{
    let terminal_string = injectable_node.get_child(2).lhs.lexeme();

    let affinity = match &injectable_node.get_child(1).lhs.lexeme()[..] {
        "left" => InjectionAffinity::Left,
        "right" => InjectionAffinity::Right,
        aff => panic!("Unexpected injection affinity: '{}'", aff),
    };

    grammar_builder.mark_injectable(terminal_string, affinity.clone());

    let pattern_string = match injectable_node.get_opt(3) {
        Some(patt_node) => {
            let pattc = &patt_node.get_child(0).lhs.lexeme();
            let pattern_string = &pattc[..].trim_matches('`');
            Some(string_utils::replace_escapes(pattern_string))
        }
        None => None,
    };

    formatter_builder.add_injection(InjectableString {
        terminal: grammar_builder.kind_for(terminal_string),
        terminal_string: terminal_string.clone(),
        pattern_string,
        affinity,
    })?;

    Ok(())
}

/// Traverses an ignorable symbol specification region, marking the associated terminal symbol as
/// ignorable in the grammar.
///
/// # Parameters
///
/// * `ignorable_node` - the `SpecSymbol::Ignorable` being traversed.
/// * `grammar_builder` - the grammar builder for the specification.
fn traverse_ignorable_region<Symbol: GrammarSymbol, GrammarType>(
    ignorable_node: &Tree<SpecSymbol>,
    grammar_builder: &mut dyn GrammarBuilder<String, Symbol, GrammarType>,
) where
    GrammarType: Grammar<Symbol>,
{
    let terminal = ignorable_node.get_child(1).lhs.lexeme();
    grammar_builder.mark_ignorable(terminal);
}

/// Traverses an alphabet region of a specification parse and extracts the alphabet into the CDFA
/// being built.
///
/// # Parameters
///
/// * `alphabet_node` - the `SpecSymbol::Alphabet` node of the parse tree.
/// * `cdfa_builder` - the CDFA builder for the specification.
fn traverse_alphabet_region<CDFABuilderType, CDFAType, Symbol: GrammarSymbol>(
    alphabet_node: &Tree<SpecSymbol>,
    cdfa_builder: &mut CDFABuilderType,
) where
    CDFAType: CDFA<usize, Symbol>,
    CDFABuilderType: CDFABuilder<String, Symbol, CDFAType>,
{
    let escaped_alphabet = alphabet_node.get_child(1).lhs.lexeme().trim_matches('\'');
    let alphabet = string_utils::replace_escapes(&escaped_alphabet);

    cdfa_builder.set_alphabet(alphabet.chars());
}

/// Traverses a CDFA specification region.
///
/// An error is returned if the CDFA or grammar cannot be built for the region.
///
/// # Parameters
///
/// * `cdfa_node` - the `SpecSymbol::CDFA` node of the parse tree to traverse.
/// * `cdfa_builder` - the CDFA builder for the specification.
/// * `grammar_builder` - the grammar builder for the specification.
fn traverse_cdfa_region<CDFABuilderType, CDFAType, Symbol: GrammarSymbol, GrammarType>(
    cdfa_node: &Tree<SpecSymbol>,
    cdfa_builder: &mut CDFABuilderType,
    grammar_builder: &mut dyn GrammarBuilder<String, Symbol, GrammarType>,
) -> Result<(), spec::GenError>
where
    CDFAType: CDFA<usize, Symbol>,
    CDFABuilderType: CDFABuilder<String, Symbol, CDFAType>,
    GrammarType: Grammar<Symbol>,
{
    generate_cdfa_states(cdfa_node.get_child(2), cdfa_builder, grammar_builder)
}

/// Traverses a grammar specification region.
///
/// An error is returned if the grammar or formatter cannot be built for the region.
///
/// # Parameters
///
/// * `grammar_node` - the root `SpecSymbol::Grammar` node of the parse tree.
/// * `grammar_builder` - the grammar builder for the specification.
/// * `formatter_builder` - the formatter builder for the specification.
fn traverse_grammar_region<Symbol: GrammarSymbol, GrammarType>(
    grammar_node: &Tree<SpecSymbol>,
    grammar_builder: &mut dyn GrammarBuilder<String, Symbol, GrammarType>,
    formatter_builder: &mut FormatterBuilder<Symbol>,
) -> Result<(), spec::GenError>
where
    GrammarType: Grammar<Symbol>,
{
    generate_grammar_prods(
        grammar_node.get_child(2),
        grammar_builder,
        formatter_builder,
    )
}

/// Recursively traverses `SpecSymbol::States` nodes to build CDFA state definitions.
///
/// Returns an error if a state definition cannot be built.
///
/// # Parameters
///
/// * `states_node` - the `SpecSymbol::States` node of the parse tree to traverse.
/// * `builder` - the CDFA builder for the specification.
/// * `grammar_builder` - the grammar builder for the specification.
fn generate_cdfa_states<CDFABuilderType, CDFAType, Symbol: GrammarSymbol, GrammarType>(
    states_node: &Tree<SpecSymbol>,
    builder: &mut CDFABuilderType,
    grammar_builder: &mut dyn GrammarBuilder<String, Symbol, GrammarType>,
) -> Result<(), spec::GenError>
where
    CDFAType: CDFA<usize, Symbol>,
    CDFABuilderType: CDFABuilder<String, Symbol, CDFAType>,
    GrammarType: Grammar<Symbol>,
{
    let state_node = states_node.get_child(states_node.children.len() - 1);

    let sdec_node = state_node.get_child(0);

    let targets_node = sdec_node.get_child(0);
    let head_state = targets_node
        .get_child(targets_node.children.len() - 1)
        .lhs
        .lexeme();

    // Generate list of source-states for this state definition.
    let mut states: Vec<&String> = vec![head_state];
    if targets_node.children.len() == 3 {
        generate_cdfa_targets(targets_node.get_child(0), &mut states);
    }

    // Process the source-state acceptor, if it exists.
    if sdec_node.children.len() == 2 {
        let acceptor_node = sdec_node.get_child(1);
        let id_or_def_node = acceptor_node.get_child(1);
        let token = id_or_def_node.get_child(0).lhs.lexeme();
        let kind = grammar_builder.kind_for(token);

        for state in &states {
            add_cdfa_state_tokenizer(acceptor_node, *state, &kind, builder, grammar_builder);
        }
    }

    // If the source-states have transitions, build them.
    if let Some(trans_node) = state_node.get_opt(1) {
        generate_cdfa_trans(trans_node.get_child(0), &states, builder, grammar_builder)?;
    }

    // Recurse if we have more state definitions.
    if states_node.children.len() == 2 {
        generate_cdfa_states(states_node.get_child(0), builder, grammar_builder)
    } else {
        // If this is the last definition, then we are in the start state.
        builder.mark_start(head_state);
        Ok(())
    }
}

/// Recursively traverses `SpecSymbol::Targets` nodes to build the list of source CDFA states
/// to add transitions out of. Target lists are specific to a particular state definition.
///
/// # Parameters
///
/// * `targets_node` - the `SpecSymbol::Targets` node of the parse tree to traverse.
/// * `accumulator` - an accumulator into which discovered targets will be added.
fn generate_cdfa_targets<'tree>(
    targets_node: &'tree Tree<SpecSymbol>,
    accumulator: &mut Vec<&'tree String>,
) {
    accumulator.push(
        &targets_node
            .get_child(targets_node.children.len() - 1)
            .lhs
            .lexeme(),
    );

    // Recurse if we have more source-states.
    if targets_node.children.len() == 3 {
        generate_cdfa_targets(targets_node.get_child(0), accumulator);
    }
}

/// Recursively traverses `SpecSymbol::Transitions` nodes to build the set of state transitions
/// of a CDFA state definition.
///
/// Returns an error if any of the state transitions cannot be built.
///
/// Returns an error if any of the state transitions cannot be built.
///
/// # Parameters
///
/// * `trans_node` - the `SpecSymbol::Transitions` node of the parse tree to traverse.
/// * `sources` - the source state names to add the visited transitions out of.
/// * `builder` - the CDFA builder for the specification.
/// * `grammar_builder` - the grammar builder for the specification.
fn generate_cdfa_trans<CDFABuilderType, CDFAType, Symbol: GrammarSymbol, GrammarType>(
    trans_node: &Tree<SpecSymbol>,
    sources: &[&String],
    builder: &mut CDFABuilderType,
    grammar_builder: &mut dyn GrammarBuilder<String, Symbol, GrammarType>,
) -> Result<(), spec::GenError>
where
    CDFAType: CDFA<usize, Symbol>,
    CDFABuilderType: CDFABuilder<String, Symbol, CDFAType>,
    GrammarType: Grammar<Symbol>,
{
    let tran_node = trans_node.get_child(trans_node.children.len() - 1);

    let destination = tran_node.get_child(2).get_child(0);
    let mut transit_builder = match destination.lhs.kind() {
        &SpecSymbol::TId => TransitBuilder::to(destination.lhs.lexeme().clone()),
        &SpecSymbol::Acceptor => {
            let id_or_def_node = destination.get_child(1);
            let dest = id_or_def_node.get_child(0).lhs.lexeme();
            let mut transit_builder = TransitBuilder::to(dest.clone());

            builder.accept(dest);

            // If the accepted state has an acceptor destination, record it.
            if let Some(accd_node) = destination.get_opt(2) {
                let acceptor_destination = &accd_node.get_child(1).lhs.lexeme();
                transit_builder.accept_to((*acceptor_destination).clone());
            }

            // If the accepted state is not the default matcher, tokenize it.
            if *dest != *spec::DEF_MATCHER {
                builder.tokenize(dest, &grammar_builder.kind_for(dest));
            }

            transit_builder
        }
        symbol => panic!("Unexpected transition destination symbol: {:?}", symbol),
    };

    let consumer = match tran_node.get_child(1).get_child(0).lhs.kind() {
        SpecSymbol::TArrow => ConsumerStrategy::All,
        SpecSymbol::TDoubleArrow => ConsumerStrategy::None,
        s => panic!("Unexpected transition consumer: {:?}", s),
    };

    transit_builder.consumer(consumer);

    let matcher = tran_node.get_child(0).get_child(0);
    match matcher.lhs.kind() {
        SpecSymbol::Matchers => {
            generate_cdfa_mtcs(matcher, sources, &transit_builder, builder)?;
        }
        SpecSymbol::TDef => {
            for source in sources {
                builder.default_to(source, transit_builder.build())?;
            }
        }
        _ => panic!("Transition map input is neither Matchers nor TDef"),
    }

    // Recurse if there are more transitions in this state definition.
    if trans_node.children.len() == 2 {
        generate_cdfa_trans(trans_node.get_child(0), sources, builder, grammar_builder)
    } else {
        Ok(())
    }
}

/// Recursively traverses `SpecSymbol::Matchers` nodes to build a set of matchers for a particular
/// CDFA state transition.
///
/// Returns an error if the matchers cannot be built.
///
/// # Parameters
///
/// * `mtcs_nodes` - the `SpecSymbol::Matchers` node of the parse tree to traverse.
/// * `sources` - the source states of the associated transition.
/// * `transit_builder` - the builder of the associated transition transit.
/// * `builder` - the CDFA builder for the specification.
#[allow(clippy::ptr_arg)]
fn generate_cdfa_mtcs<CDFABuilderType, CDFAType, Symbol: GrammarSymbol>(
    mtcs_node: &Tree<SpecSymbol>,
    sources: &[&String],
    transit_builder: &TransitBuilder<String>,
    builder: &mut CDFABuilderType,
) -> Result<(), spec::GenError>
where
    CDFAType: CDFA<usize, Symbol>,
    CDFABuilderType: CDFABuilder<String, Symbol, CDFAType>,
{
    let mtc_node = mtcs_node.children.last().unwrap();

    if mtc_node.children.len() == 1 {
        // This is a simple or chain matcher.

        let matcher = mtc_node.get_child(0);
        let matcher_string: String = matcher
            .lhs
            .lexeme()
            .chars()
            .skip(1)
            .take(matcher.lhs.lexeme().len() - 2)
            .collect();
        let matcher_cleaned = string_utils::replace_escapes(&matcher_string);

        let is_simple = matcher_cleaned.len() == 1;
        for source in sources {
            if is_simple {
                builder.mark_trans(
                    source,
                    transit_builder.build(),
                    matcher_cleaned.chars().next().unwrap(),
                )?;
            } else {
                builder.mark_chain(source, transit_builder.build(), matcher_cleaned.chars())?;
            }
        }
    } else {
        // This is a range matcher.

        let range_start_node = mtc_node.get_child(0);
        let range_end_node = mtc_node.get_child(2);

        let escaped_range_start_string: String = range_start_node
            .lhs
            .lexeme()
            .chars()
            .skip(1)
            .take(range_start_node.lhs.lexeme().len() - 2)
            .collect();
        let range_start_string = string_utils::replace_escapes(&escaped_range_start_string);

        if range_start_string.len() > 1 {
            return Err(spec::GenError::MatcherErr(format!(
                "Range start must be one character, but was '{}'",
                range_start_string
            )));
        }

        let escaped_range_end_string: String = range_end_node
            .lhs
            .lexeme()
            .chars()
            .skip(1)
            .take(range_end_node.lhs.lexeme().len() - 2)
            .collect();
        let range_end_string: String = string_utils::replace_escapes(&escaped_range_end_string);

        if range_end_string.len() > 1 {
            return Err(spec::GenError::MatcherErr(format!(
                "Range end must be one character, but was '{}'",
                range_end_string
            )));
        }

        let range_start = range_start_string.chars().next().unwrap();
        let range_end = range_end_string.chars().next().unwrap();

        builder.mark_range_for_all(
            sources.iter(),
            transit_builder.build(),
            range_start,
            range_end,
        )?;
    }

    // Recurse if there are more matchers for this transition.
    if mtcs_node.children.len() == 3 {
        generate_cdfa_mtcs(mtcs_node.get_child(0), sources, transit_builder, builder)
    } else {
        Ok(())
    }
}

/// Generates CDFA state or transition acceptance and tokenization information from a
/// `SpecSymbol::Acceptor` node.
///
/// # Parameters
///
/// * `acceptor_node` - the `SpecSymbol::Acceptor` node of the parse tree to traverse.
/// * `state` - the name of the CDFA state to accept and possibly tokenize.
/// * `kind` - the grammar symbol to tokenize the state to, or the default matcher symbol if the
/// state should not produce a token.
/// * `builder` - the CDFA builder for the specification.
/// * `grammar_builder` - the grammar builder for the specification.
#[allow(clippy::ptr_arg)]
fn add_cdfa_state_tokenizer<CDFABuilderType, CDFAType, Symbol: GrammarSymbol, GrammarType>(
    acceptor_node: &Tree<SpecSymbol>,
    state: &String,
    kind: &Symbol,
    builder: &mut CDFABuilderType,
    grammar_builder: &mut dyn GrammarBuilder<String, Symbol, GrammarType>,
) where
    CDFAType: CDFA<usize, Symbol>,
    CDFABuilderType: CDFABuilder<String, Symbol, CDFAType>,
    GrammarType: Grammar<Symbol>,
{
    match acceptor_node.get_opt(2) {
        Some(accd_node) => {
            let acceptor_destination = &accd_node.get_child(1).lhs.lexeme();
            builder.accept_to(state, acceptor_destination);
        }
        None => {
            builder.accept(state);
        }
    }

    // Only tokenize an accepted state if it is not the default matcher.
    if *kind != grammar_builder.kind_for(&spec::DEF_MATCHER) {
        builder.tokenize(state, kind);
    }
}

/// Recursively traverse `SpecSymbol::Productions` nodes to build the set of grammar productions
/// and formatter patterns of a grammar region.
///
/// Returns an error if a production cannot be built.
///
/// # Parameters
///
/// * `prods_node` - the `SpecSymbol::Productions` node of the parse tree to traverse.
/// * `grammar_builder` - the grammar builder for the specification.
/// * `formatter_builder` - the formatter builder for the specification.
fn generate_grammar_prods<Symbol: GrammarSymbol, GrammarType>(
    prods_node: &Tree<SpecSymbol>,
    grammar_builder: &mut dyn GrammarBuilder<String, Symbol, GrammarType>,
    formatter_builder: &mut FormatterBuilder<Symbol>,
) -> Result<(), spec::GenError>
where
    GrammarType: Grammar<Symbol>,
{
    // Recurse if there are more productions in this grammar region.
    if prods_node.children.len() == 2 {
        generate_grammar_prods(prods_node.get_child(0), grammar_builder, formatter_builder)?;
    }

    let prod_node = prods_node.get_child(prods_node.children.len() - 1);

    let id = &prod_node.get_child(0).lhs.lexeme();

    let def_pattern_node = &prod_node.get_child(1);

    generate_grammar_rhss(
        prod_node.get_child(2),
        id,
        def_pattern_node,
        grammar_builder,
        formatter_builder,
    )
}

/// Recursively traverse `SpecSymbol::RightHandSides` nodes to build the set of grammar productions
/// of a single production definition.
///
/// Returns an error if a production cannot be built.
///
/// Returns an error if a production cannot be built.
///
/// # Parameters
///
/// * `rhss_node` - the `SpecSymbol::RightHandSides` node of the parse tree to traverse.
/// * `lhs` - the left-hand-side symbol common to each production in this definition.
/// * `def_pattern_node` - the default formatter pattern for this production definition.
/// * `grammar_builder` - the grammar builder for the specification.
/// * `formatter_builder` - the formatter builder for the specification.
#[allow(clippy::ptr_arg)]
fn generate_grammar_rhss<Symbol: GrammarSymbol, GrammarType>(
    rhss_node: &Tree<SpecSymbol>,
    lhs: &String,
    def_pattern_node: &Tree<SpecSymbol>,
    grammar_builder: &mut dyn GrammarBuilder<String, Symbol, GrammarType>,
    formatter_builder: &mut FormatterBuilder<Symbol>,
) -> Result<(), spec::GenError>
where
    GrammarType: Grammar<Symbol>,
{
    let rhs_node = rhss_node.get_child(rhss_node.children.len() - 1);

    // Build list of symbols representing the right-hand-side of this production.
    let mut ids: Vec<ProductionSymbol<String>> = Vec::new();
    generate_grammar_ids(rhs_node.get_child(1), &mut ids, grammar_builder);

    // Try to mark this symbol as the start symbol of the grammar.
    // This will only succeed for the first caller.
    grammar_builder.try_mark_start(lhs);

    let string_production = Production::from(lhs.clone(), ids);
    let production = grammar_builder.add_production(string_production.clone());

    // If this production does not have a pattern, use the default one.
    let mut pattopt_node = rhs_node.get_child(2);
    if pattopt_node.is_empty() {
        pattopt_node = def_pattern_node
    }

    if !pattopt_node.is_empty() {
        let pattc = &pattopt_node.get_child(0).lhs.lexeme();
        let pattern_str = &pattc[..].trim_matches('`');

        formatter_builder.add_pattern(PatternPair {
            production,
            string_production,
            pattern: (*pattern_str).to_string(),
        })?;
    }

    // Recurse if there are more production right-hand-sides.
    if rhss_node.children.len() == 2 {
        generate_grammar_rhss(
            rhss_node.get_child(0),
            lhs,
            def_pattern_node,
            grammar_builder,
            formatter_builder,
        )?;
    }

    Ok(())
}

/// Recursively traverse `SpecSymbol::Ids` nodes to build the list of production symbols of a
/// production right-hand-side.
///
/// # Parameters
///
/// * `ids_node` - the `SpecSymbol::Ids` node of the parse tree to traverse.
/// * `ids_accumulator` - a vector to store the discovered production symbols.
/// * `grammar_builder` - the grammar builder for the specification.
fn generate_grammar_ids<Symbol: GrammarSymbol, GrammarType>(
    ids_node: &Tree<SpecSymbol>,
    ids_accumulator: &mut Vec<ProductionSymbol<String>>,
    grammar_builder: &mut dyn GrammarBuilder<String, Symbol, GrammarType>,
) where
    GrammarType: Grammar<Symbol>,
{
    if ids_node.is_empty() {
        return;
    }

    generate_grammar_ids(ids_node.get_child(0), ids_accumulator, grammar_builder);

    let id_node = ids_node.get_child(1);
    let symbol = match id_node.lhs.kind() {
        SpecSymbol::TId => ProductionSymbol::symbol(id_node.lhs.lexeme().clone()),
        SpecSymbol::TOptId => {
            let lex = &id_node.lhs.lexeme()[..];
            let dest = &lex[1..lex.len() - 1].to_string();

            // Add (hidden) intermediate optional state to grammar.
            let opt_state: String = format!("opt#{}", dest);
            grammar_builder.add_optional_state(&opt_state, dest);

            ProductionSymbol::symbol(opt_state)
        }
        SpecSymbol::TListId => {
            let lex = &id_node.lhs.lexeme()[..];
            let target = lex[1..lex.len() - 1].to_string();

            ProductionSymbol::symbol_list(target)
        }
        _ => panic!("Unexpected production identifier type"),
    };

    ids_accumulator.push(symbol);
}

/// Returns an error if there are any terminal symbols in the grammar which are not tokenized by the
/// CDFA. Such symbols can never be produced, so any productions involving them are meaningless,
/// and as such they are an indicator of possible grammar or CDFA specification errors.
fn orphan_check<Symbol: GrammarSymbol>(
    ecdfa: &EncodedCDFA<Symbol>,
    grammar: &dyn Grammar<Symbol>,
) -> Result<(), spec::GenError> {
    let mut ecdfa_products: HashSet<&Symbol> = HashSet::new();
    for product in ecdfa.produces() {
        ecdfa_products.insert(product);
    }

    for symbol in grammar.terminals() {
        if !ecdfa_products.contains(symbol) {
            return Err(spec::GenError::MappingErr(format!(
                "Orphaned terminal '{}' is not tokenized by the ECDFA",
                grammar.symbol_string(symbol),
            )));
        }
    }

    Ok(())
}
