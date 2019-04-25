use {
    core::{
        data::Data,
        fmt::{Formatter, FormatterBuilder, PatternPair},
        parse::{
            grammar::{Grammar, GrammarBuilder},
            Production, Tree,
        },
        scan::{
            ecdfa::{EncodedCDFA, EncodedCDFABuilder},
            CDFABuilder, Kind, State, CDFA,
        },
        spec::{
            self,
            lang::Symbol,
            region::{self, RegionType},
        },
        util::string_utils,
    },
    std::collections::HashSet,
};

pub fn generate_spec(
    parse: &Tree<Symbol>,
) -> Result<(EncodedCDFA<Kind>, Grammar<Kind>, Formatter), spec::GenError> {
    let mut ecdfa_builder: EncodedCDFABuilder<String, Kind> = EncodedCDFABuilder::new();
    let mut grammar_builder = GrammarBuilder::new();
    let mut formatter_builder = FormatterBuilder::new();

    traverse_spec_regions(
        parse.get_child(0),
        &mut ecdfa_builder,
        &mut grammar_builder,
        &mut formatter_builder,
    )?;

    let ecdfa = ecdfa_builder.build()?;
    let grammar = grammar_builder.build();

    orphan_check(&ecdfa, &grammar)?;

    Ok((ecdfa, grammar, formatter_builder.build()))
}

fn traverse_spec_regions<CDFABuilderType, CDFAType>(
    regions_node: &Tree<Symbol>,
    cdfa_builder: &mut CDFABuilderType,
    grammar_builder: &mut GrammarBuilder<Kind>,
    formatter_builder: &mut FormatterBuilder,
) -> Result<(), spec::GenError>
where
    CDFAType: CDFA<usize, Kind>,
    CDFABuilderType: CDFABuilder<String, Kind, CDFAType>,
{
    let mut region_handler = |inner_node: &Tree<Symbol>, region_type: &RegionType| {
        match region_type {
            RegionType::Alphabet => traverse_alphabet_region(inner_node, cdfa_builder),
            RegionType::CDFA => traverse_cdfa_region(inner_node, cdfa_builder)?,
            RegionType::Grammar => {
                traverse_grammar_region(inner_node, grammar_builder, formatter_builder)?
            }
        }

        Ok(())
    };

    region::traverse(regions_node, &mut region_handler)
}

fn traverse_alphabet_region<CDFABuilderType, CDFAType>(
    alphabet_node: &Tree<Symbol>,
    cdfa_builder: &mut CDFABuilderType,
) where
    CDFAType: CDFA<usize, String>,
    CDFABuilderType: CDFABuilder<String, Kind, CDFAType>,
{
    let escaped_alphabet = alphabet_node.get_child(1).lhs.lexeme().trim_matches('\'');
    let alphabet = string_utils::replace_escapes(&escaped_alphabet);

    cdfa_builder.set_alphabet(alphabet.chars());
}

fn traverse_cdfa_region<CDFABuilderType, CDFAType>(
    cdfa_node: &Tree<Symbol>,
    cdfa_builder: &mut CDFABuilderType,
) -> Result<(), spec::GenError>
where
    CDFAType: CDFA<usize, Kind>,
    CDFABuilderType: CDFABuilder<String, Kind, CDFAType>,
{
    generate_cdfa_states(cdfa_node.get_child(2), cdfa_builder)
}

fn traverse_grammar_region(
    grammar_node: &Tree<Symbol>,
    grammar_builder: &mut GrammarBuilder<Kind>,
    formatter_builder: &mut FormatterBuilder,
) -> Result<(), spec::GenError> {
    generate_grammar_prods(
        grammar_node.get_child(2),
        grammar_builder,
        formatter_builder,
    )
}

fn generate_cdfa_states<CDFABuilderType, CDFAType>(
    states_node: &Tree<Symbol>,
    builder: &mut CDFABuilderType,
) -> Result<(), spec::GenError>
where
    CDFAType: CDFA<usize, Kind>,
    CDFABuilderType: CDFABuilder<String, Kind, CDFAType>,
{
    let state_node = states_node.get_child(states_node.children.len() - 1);

    let sdec_node = state_node.get_child(0);

    let targets_node = sdec_node.get_child(0);
    let head_state = targets_node
        .get_child(targets_node.children.len() - 1)
        .lhs
        .lexeme();

    let mut states: Vec<&State> = vec![head_state];
    if targets_node.children.len() == 3 {
        generate_cdfa_targets(targets_node.get_child(0), &mut states);
    }

    if sdec_node.children.len() == 2 {
        let acceptor_node = sdec_node.get_child(1);
        let id_or_def_node = acceptor_node.get_child(1);
        let token = &id_or_def_node.get_child(0).lhs.lexeme();

        for state in &states {
            add_cdfa_tokenizer(acceptor_node, *state, None, token, builder)?;
        }
    }

    let transopt_node = state_node.get_child(1);
    if !transopt_node.is_empty() {
        generate_cdfa_trans(transopt_node.get_child(0), &states, builder)?;
    }

    if states_node.children.len() == 2 {
        generate_cdfa_states(states_node.get_child(0), builder)
    } else {
        builder.mark_start(head_state);
        Ok(())
    }
}

fn generate_cdfa_targets<'tree>(
    targets_node: &'tree Tree<Symbol>,
    accumulator: &mut Vec<&'tree State>,
) {
    accumulator.push(
        &targets_node
            .get_child(targets_node.children.len() - 1)
            .lhs
            .lexeme(),
    );
    if targets_node.children.len() == 3 {
        generate_cdfa_targets(targets_node.get_child(0), accumulator);
    }
}

fn generate_cdfa_trans<CDFABuilderType, CDFAType>(
    trans_node: &Tree<Symbol>,
    sources: &[&State],
    builder: &mut CDFABuilderType,
) -> Result<(), spec::GenError>
where
    CDFAType: CDFA<usize, Kind>,
    CDFABuilderType: CDFABuilder<String, Kind, CDFAType>,
{
    let tran_node = trans_node.get_child(trans_node.children.len() - 1);

    let trand_node = tran_node.get_child(2);

    let dest = match trand_node.get_child(0).lhs.kind() {
        &Symbol::TId => trand_node.get_child(0).lhs.lexeme(),
        &Symbol::Acceptor => {
            let acceptor_node = trand_node.get_child(0);
            let id_or_def_node = acceptor_node.get_child(1);
            let token = id_or_def_node.get_child(0).lhs.lexeme();

            // Immediate state pass-through
            for source in sources {
                add_cdfa_tokenizer(acceptor_node, token, Some(*source), token, builder)?;
            }

            token
        }
        symbol => panic!("Unexpected transition destination symbol: {:?}", symbol),
    };

    let matcher = tran_node.get_child(0);
    match matcher.lhs.kind() {
        Symbol::Matchers => {
            generate_cdfa_mtcs(matcher, sources, dest, builder)?;
        }
        Symbol::TDef => {
            for source in sources {
                builder.default_to(source, dest)?;
            }
        }
        _ => panic!("Transition map input is neither Matchers nor TDef"),
    }

    if trans_node.children.len() == 2 {
        generate_cdfa_trans(trans_node.get_child(0), sources, builder)
    } else {
        Ok(())
    }
}

#[allow(clippy::ptr_arg)]
fn generate_cdfa_mtcs<CDFABuilderType, CDFAType>(
    mtcs_node: &Tree<Symbol>,
    sources: &[&State],
    dest: &State,
    builder: &mut CDFABuilderType,
) -> Result<(), spec::GenError>
where
    CDFAType: CDFA<usize, Kind>,
    CDFABuilderType: CDFABuilder<String, Kind, CDFAType>,
{
    let mtc_node = mtcs_node.children.last().unwrap();

    if mtc_node.children.len() == 1 {
        let matcher = mtc_node.get_child(0);
        let matcher_string: String = matcher
            .lhs
            .lexeme()
            .chars()
            .skip(1)
            .take(matcher.lhs.lexeme().len() - 2)
            .collect();
        let matcher_cleaned = string_utils::replace_escapes(&matcher_string);
        if matcher_cleaned.len() == 1 {
            for source in sources {
                builder.mark_trans(source, dest, matcher_cleaned.chars().next().unwrap())?;
            }
        } else {
            for source in sources {
                builder.mark_chain(source, dest, matcher_cleaned.chars())?;
            }
        }
    } else {
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

        builder.mark_range_for_all(sources.iter(), dest, range_start, range_end)?;
    }

    if mtcs_node.children.len() == 3 {
        generate_cdfa_mtcs(mtcs_node.get_child(0), sources, dest, builder)
    } else {
        Ok(())
    }
}

#[allow(clippy::ptr_arg)]
fn add_cdfa_tokenizer<CDFABuilderType, CDFAType, Symbol: Data + Default>(
    acceptor_node: &Tree<Symbol>,
    state: &State,
    from: Option<&State>,
    kind: &Kind,
    builder: &mut CDFABuilderType,
) -> Result<(), spec::GenError>
where
    CDFAType: CDFA<usize, Kind>,
    CDFABuilderType: CDFABuilder<String, Kind, CDFAType>,
{
    let accd_opt_node = acceptor_node.get_child(2);
    if accd_opt_node.is_empty() {
        builder.accept(state);
    } else {
        let acceptor_destination = &accd_opt_node.get_child(1).lhs.lexeme();
        match from {
            None => builder.accept_to_from_all(state, acceptor_destination)?,
            Some(from_state) => builder.accept_to(state, from_state, acceptor_destination)?,
        };
    }

    if kind != spec::DEF_MATCHER {
        builder.tokenize(state, kind);
    }
    Ok(())
}

fn generate_grammar_prods(
    prods_node: &Tree<Symbol>,
    grammar_builder: &mut GrammarBuilder<Kind>,
    formatter_builder: &mut FormatterBuilder,
) -> Result<(), spec::GenError> {
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

fn generate_grammar_rhss(
    rhss_node: &Tree<Symbol>,
    lhs: &str,
    def_pattern_node: &Tree<Symbol>,
    grammar_builder: &mut GrammarBuilder<Kind>,
    formatter_builder: &mut FormatterBuilder,
) -> Result<(), spec::GenError> {
    let rhs_node = rhss_node.get_child(rhss_node.children.len() - 1);

    let mut ids: Vec<String> = Vec::new();
    generate_grammar_ids(rhs_node.get_child(1), &mut ids, grammar_builder);

    let production = Production {
        lhs: lhs.to_string(),
        rhs: ids,
    };

    grammar_builder.try_mark_start(&production.lhs);
    grammar_builder.add_production(production.clone());

    let mut pattopt_node = rhs_node.get_child(2);
    if pattopt_node.is_empty() {
        pattopt_node = def_pattern_node
    }

    if !pattopt_node.is_empty() {
        let pattc = &pattopt_node.get_child(0).lhs.lexeme();
        let pattern_string = &pattc[..].trim_matches('`');
        let pattern = string_utils::replace_escapes(pattern_string);

        formatter_builder.add_pattern(PatternPair {
            production,
            pattern,
        })?;
    }

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

fn generate_grammar_ids(
    ids_node: &Tree<Symbol>,
    ids_accumulator: &mut Vec<String>,
    grammar_builder: &mut GrammarBuilder<Kind>,
) {
    if !ids_node.is_empty() {
        generate_grammar_ids(ids_node.get_child(0), ids_accumulator, grammar_builder);

        let id_node = ids_node.get_child(1);
        let id = match id_node.lhs.kind() {
            Symbol::TId => id_node.lhs.lexeme().clone(),
            Symbol::TOptId => {
                let lex = &id_node.lhs.lexeme()[..];
                let dest = &lex[1..lex.len() - 1].to_string();
                let opt_state: String = format!("opt#{}", dest);

                grammar_builder.add_optional_state(&opt_state, dest);

                opt_state
            }
            _ => panic!("Production identifier is neither a TId nor a TOptId"),
        };

        ids_accumulator.push(id);
    }
}

fn orphan_check<Symbol: Data + Default>(
    ecdfa: &EncodedCDFA<Symbol>,
    grammar: &Grammar<Symbol>,
) -> Result<(), spec::GenError> {
    let mut ecdfa_products: HashSet<&Symbol> = HashSet::new();
    for product in ecdfa.produces() {
        ecdfa_products.insert(product);
    }

    for symbol in grammar.terminals() {
        if !ecdfa_products.contains(symbol) {
            return Err(spec::GenError::MappingErr(format!(
                "Orphaned terminal '{}' is not tokenized by the ECDFA",
                symbol.to_string()
            )));
        }
    }

    Ok(())
}
