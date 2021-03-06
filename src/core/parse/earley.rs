use {
    core::{
        data::Data,
        fmt::InjectionAffinity,
        lex::Token,
        parse::{
            self,
            grammar::{Grammar, GrammarSymbol},
            Parser, Production, ProductionSymbol, SymbolParseMethod, Tree,
        },
    },
    std::{
        cmp::Ordering,
        collections::{HashMap, HashSet, LinkedList},
        usize,
    },
};

pub struct EarleyParser;

impl<Symbol: GrammarSymbol> Parser<Symbol> for EarleyParser {
    fn parse(
        &self,
        lex: Vec<Token<Symbol>>,
        grammar: &dyn Grammar<Symbol>,
    ) -> Result<Tree<Symbol>, parse::Error> {
        let mut chart: RChart<Symbol> = RChart::new();
        let mut parse_chart: PChart<Symbol> = PChart::new();

        let final_required_token = {
            let mut index = lex.len();
            for token in lex.iter().rev() {
                index -= 1;
                if !grammar.is_injectable(token.kind()) && !grammar.is_ignorable(token.kind()) {
                    break;
                }
            }
            index
        };

        grammar
            .productions_for_lhs(grammar.start())
            .unwrap()
            .iter()
            .for_each(|prod| chart.row_mut(0).unsafe_append(Item::start(prod)));

        let mut cursor = 0;
        while cursor < chart.len() {
            complete_full(cursor, grammar, &mut chart);
            predict_full(grammar, &mut chart);
            parse_mark_full(cursor, &chart, &mut parse_chart);
            scan_full(
                cursor,
                final_required_token,
                &lex,
                grammar,
                &mut chart,
                &mut parse_chart,
            );

            cursor += 1;
        }

        fn complete_full<'inner, 'grammar: 'inner, Symbol: GrammarSymbol>(
            cursor: usize,
            grammar: &'grammar dyn Grammar<Symbol>,
            chart: &'inner mut RChart<'grammar, Symbol>,
        ) {
            let mut i = 0;
            while i < chart.row(cursor).complete().len() {
                let accumulator = {
                    let item = chart.row(cursor).complete().item(i);

                    if item.ignore_next {
                        i += 1;
                        continue;
                    }

                    cross(
                        chart.row(item.start).incomplete().items.iter(),
                        &item.rule.lhs,
                        grammar,
                    )
                };

                let mut items_to_add = Vec::new();
                for completed_item in accumulator {
                    if !chart.row(cursor).contains(&completed_item) {
                        items_to_add.push(completed_item);
                    }
                }

                for new_item in items_to_add {
                    chart.row_mut(cursor).unsafe_append(new_item);
                }
                i += 1;
            }
        }

        fn predict_full<'inner, 'grammar: 'inner, Symbol: GrammarSymbol>(
            grammar: &'grammar dyn Grammar<Symbol>,
            chart: &'inner mut RChart<'grammar, Symbol>,
        ) {
            let cursor = chart.len() - 1;
            let mut symbols: HashSet<Symbol> = HashSet::new();

            let mut i = 0;
            while i < chart.row(cursor).incomplete().len() {
                let item = {
                    let item = chart.row(cursor).incomplete().item(i);
                    if item.ignore_next {
                        i += 1;
                        continue;
                    }
                    item.clone()
                };

                let symbol = item.next_symbol().unwrap();

                if grammar.is_non_terminal(symbol) {
                    if grammar.is_nullable_nt(symbol) {
                        let new_item = item.advance_new();

                        if !chart.row(cursor).contains(&new_item) {
                            chart.row_mut(cursor).unsafe_append(new_item);
                        }
                    }

                    if !symbols.contains(symbol) {
                        predict_op(cursor, item.depth + 1, symbol, grammar, chart);
                        symbols.insert(symbol.clone());
                    }
                }
                i += 1;
            }
        }

        fn parse_mark_full<'inner, 'grammar: 'inner, Symbol: GrammarSymbol>(
            cursor: usize,
            chart: &'inner RChart<'grammar, Symbol>,
            parse_chart: &mut PChart<'grammar, Symbol>,
        ) {
            for item in &chart.row(cursor).complete().items {
                if !item.ignore_next || cursor == chart.len() - 1 {
                    mark_completed_item(&item, cursor, parse_chart);
                }
            }
        }

        fn scan_full<'inner, 'grammar: 'inner, Symbol: GrammarSymbol>(
            cursor: usize,
            final_required_token: usize,
            lex: &[Token<Symbol>],
            grammar: &'grammar dyn Grammar<Symbol>,
            chart: &'inner mut RChart<'grammar, Symbol>,
            parse_chart: &mut PChart<'grammar, Symbol>,
        ) {
            if cursor == lex.len() {
                return;
            }

            let more_required_tokens = cursor <= final_required_token;
            let symbol = lex[cursor].kind();

            let next_row = if grammar.is_ignorable(symbol) {
                cross_shadow_symbol(
                    chart.row(cursor),
                    symbol,
                    SymbolParseMethod::Ignored,
                    &InjectionAffinity::Right,
                    more_required_tokens,
                    grammar,
                )
            } else if grammar.is_injectable(symbol) {
                cross_shadow_symbol(
                    chart.row(cursor),
                    symbol,
                    SymbolParseMethod::Injected,
                    grammar.injection_affinity(symbol).unwrap(),
                    more_required_tokens,
                    grammar,
                )
            } else {
                cross(chart.row(cursor).incomplete().items.iter(), symbol, grammar)
            };

            if next_row.is_empty() {
                return;
            }

            chart.add_row(next_row);
            parse_chart.add_row();
        }

        fn predict_op<'inner, 'grammar, Symbol: GrammarSymbol>(
            cursor: usize,
            depth: usize,
            symbol: &Symbol,
            grammar: &'grammar dyn Grammar<Symbol>,
            chart: &'inner mut RChart<'grammar, Symbol>,
        ) {
            let mut items_to_add = Vec::new();

            for prod in grammar.productions_for_lhs(symbol).unwrap() {
                let new_item = Item {
                    rule: prod,
                    shadow: None,
                    shadow_top: 0,
                    start: cursor,
                    next: 0,
                    depth,
                    ignore_next: false,
                    weight: 0,
                };

                if symbol != grammar.start() || !chart.row(cursor).contains(&new_item) {
                    items_to_add.push(new_item);
                }
            }

            for new_item in items_to_add {
                chart.row_mut(cursor).unsafe_append(new_item);
            }
        }

        fn cross<'inner, 'grammar: 'inner, Symbol: GrammarSymbol>(
            src: impl Iterator<Item = &'inner Item<'grammar, Symbol>>,
            symbol: &Symbol,
            grammar: &'grammar dyn Grammar<Symbol>,
        ) -> Vec<Item<'grammar, Symbol>> {
            let mut dest: Vec<Item<Symbol>> = Vec::new();

            for item in src {
                if let Some(sym) = item.next_prod_symbol() {
                    if sym.symbol == *symbol {
                        advance_on_matching_symbol(item, sym, &mut dest, grammar);
                    }
                }
            }

            dest
        }

        fn cross_shadow_symbol<'inner, 'grammar: 'inner, Symbol: GrammarSymbol>(
            src: &'inner RChartRow<'grammar, Symbol>,
            symbol: &Symbol,
            spm: SymbolParseMethod,
            affinity: &InjectionAffinity,
            more_required_tokens: bool,
            grammar: &'grammar dyn Grammar<Symbol>,
        ) -> Vec<Item<'grammar, Symbol>> {
            let mut dest: Vec<Item<Symbol>> = Vec::new();

            for item in &src.incomplete.items {
                let sym = item.next_prod_symbol().unwrap();
                if sym.symbol == *symbol {
                    advance_on_matching_symbol(item, sym, &mut dest, grammar);
                } else {
                    advance_via_shadow(
                        item,
                        symbol,
                        spm.clone(),
                        affinity,
                        &mut dest,
                        more_required_tokens,
                        grammar,
                    );
                }
            }

            for item in &src.complete.items {
                advance_via_shadow(
                    item,
                    symbol,
                    spm.clone(),
                    affinity,
                    &mut dest,
                    more_required_tokens,
                    grammar,
                );
            }

            dest
        }

        fn advance_on_matching_symbol<'inner, 'grammar: 'inner, Symbol: GrammarSymbol>(
            item: &'inner Item<'grammar, Symbol>,
            sym: &ProductionSymbol<Symbol>,
            dest: &mut Vec<Item<'grammar, Symbol>>,
            grammar: &'grammar dyn Grammar<Symbol>,
        ) {
            if sym.is_list {
                advance_list_via_shadow(item, &sym.symbol, dest, grammar);
            } else {
                advance_past_symbol(item, dest, grammar);
            }
        }

        fn advance_past_symbol<'inner, 'grammar: 'inner, Symbol: GrammarSymbol>(
            item: &'inner Item<'grammar, Symbol>,
            dest: &mut Vec<Item<'grammar, Symbol>>,
            grammar: &'grammar dyn Grammar<Symbol>,
        ) {
            let mut next_item = item.clone();
            next_item.advance();
            advance_over_nullable_symbols(next_item, dest, grammar);
        }

        fn advance_via_shadow<'inner, 'grammar: 'inner, Symbol: GrammarSymbol>(
            item: &'inner Item<'grammar, Symbol>,
            symbol: &Symbol,
            spm: SymbolParseMethod,
            affinity: &InjectionAffinity,
            dest: &mut Vec<Item<'grammar, Symbol>>,
            more_required_tokens: bool,
            grammar: &'grammar dyn Grammar<Symbol>,
        ) {
            let ignore_next = *affinity != InjectionAffinity::Left;
            let mut weight = item.weight + 1;

            let terminal_before = {
                let prev_symbol = item.prev_symbol();
                prev_symbol.is_some() && !grammar.is_non_terminal(prev_symbol.unwrap())
            };

            let terminal_after = {
                let next_symbol = item.next_symbol();
                next_symbol.is_some() && !grammar.is_non_terminal(next_symbol.unwrap())
            };

            let satisfied = match affinity {
                InjectionAffinity::Left => terminal_before,
                InjectionAffinity::Right => terminal_after,
            };

            if !satisfied {
                weight += 1;

                let at_start = item.start == 0 && item.prev_symbol().is_none();
                let at_end = !more_required_tokens;

                if !at_start && !at_end {
                    // It is guaranteed that there is at least one satisfying parse for any
                    // injectable which is not at the ends of the lex, and this isn't it.
                    return;
                }
            }

            if !terminal_before && !terminal_after && !item.rule.rhs.is_empty() {
                // As long as there is at least one non-injected/non-ignored terminal, we can build
                // a parse tree where every injectable/ignorable is adjacent to another terminal,
                // so ignore any that aren't.
                return;
            }

            if item.can_extend_list(symbol) {
                // Never inject when an inline list can (and will) be extended.
                return;
            }

            let mut shadow_vec = item.shadow_copy();
            item.extend_shadow(&mut shadow_vec);

            shadow_vec.push(ShadowSymbol {
                symbol: symbol.clone(),
                spm,
                reps: 1,
            });

            let new_item = Item {
                rule: item.rule,
                shadow: Some(shadow_vec),
                shadow_top: item.next,
                start: item.start,
                next: item.next,
                depth: item.depth,
                ignore_next,
                weight,
            };

            advance_over_nullable_symbols(new_item, dest, grammar);
        }

        fn advance_list_via_shadow<'inner, 'grammar: 'inner, Symbol: GrammarSymbol>(
            item: &'inner Item<'grammar, Symbol>,
            symbol: &Symbol,
            dest: &mut Vec<Item<'grammar, Symbol>>,
            grammar: &'grammar dyn Grammar<Symbol>,
        ) {
            let mut shadow_vec = item.shadow_copy();

            if item.can_extend_list(symbol) {
                shadow_vec.last_mut().unwrap().reps += 1;
            } else {
                item.extend_shadow(&mut shadow_vec);

                shadow_vec.push(ShadowSymbol {
                    symbol: symbol.clone(),
                    spm: SymbolParseMethod::Repeated,
                    reps: 1,
                });
            }

            let mut new_item = Item {
                rule: item.rule,
                shadow: Some(shadow_vec),
                shadow_top: item.next + 1,
                start: item.start,
                next: item.next,
                depth: item.depth,
                ignore_next: false,
                weight: item.weight,
            };

            dest.push(new_item.clone());
            new_item.advance();
            advance_over_nullable_symbols(new_item, dest, grammar);
        }

        fn advance_over_nullable_symbols<'grammar, Symbol: GrammarSymbol>(
            item: Item<'grammar, Symbol>,
            dest: &mut Vec<Item<'grammar, Symbol>>,
            grammar: &'grammar dyn Grammar<Symbol>,
        ) {
            let mut last_item = item;

            loop {
                dest.push(last_item.clone());

                match last_item.next_prod_symbol() {
                    None => break,
                    Some(sym) => {
                        if !grammar.is_nullable_nt(&sym.symbol) {
                            break;
                        }
                    }
                }

                last_item.advance();
            }
        }

        fn mark_completed_item<'inner, 'grammar: 'inner, Symbol: GrammarSymbol>(
            item: &'inner Item<'grammar, Symbol>,
            finish: usize,
            parse_chart: &mut PChart<'grammar, Symbol>,
        ) {
            parse_chart.row_mut(item.start).add_edge(Edge {
                rule: Some(item.rule),
                shadow: item.shadow.clone(),
                shadow_top: item.shadow_top,
                shadow_len: Edge::shadow_len(&item.shadow),
                start: item.start,
                finish,
                ignored: false,
                weight: item.weight,
                depth: item.depth,
            });
        }

        fn recognized<Symbol: GrammarSymbol>(
            grammar: &dyn Grammar<Symbol>,
            chart: &RChart<Symbol>,
        ) -> bool {
            chart
                .row(chart.len() - 1)
                .complete()
                .items
                .iter()
                .any(|item| item.rule.lhs == *grammar.start() && item.start == 0)
        }

        return if recognized(grammar, &chart) {
            if cursor - 1 == lex.len() {
                Ok(parse_tree(grammar, &lex, parse_chart))
            } else {
                Err(parse::Error {
                    message: format!(
                        "Largest parse did not consume all tokens: {} of {}",
                        cursor - 1,
                        lex.len()
                    ),
                })
            }
        } else if lex.is_empty() {
            Err(parse::Error {
                message: "No symbols tokenized".to_string(),
            })
        } else if cursor - 1 == lex.len() {
            Err(parse::Error {
                message: "Recognition failed after consuming all tokens".to_string(),
            })
        } else {
            let token = &lex[cursor - 1];
            Err(parse::Error {
                message: format!(
                    "Recognition failed at token {}: {} <- '{}'",
                    cursor,
                    grammar.symbol_string(token.kind()),
                    token.lexeme_escaped(),
                ),
            })
        };

        fn parse_tree<'scope, Symbol: GrammarSymbol>(
            grammar: &'scope dyn Grammar<Symbol>,
            lex: &'scope [Token<Symbol>],
            chart: PChart<'scope, Symbol>,
        ) -> Tree<Symbol> {
            let tree = if grammar.weighted_parse() {
                parse_bottom_up(grammar, lex, chart)
            } else {
                parse_top_down(grammar, lex, chart)
            };

            push_down_inline_lists(tree)
        }

        fn push_down_inline_lists<Symbol: GrammarSymbol>(mut root: Tree<Symbol>) -> Tree<Symbol> {
            if root.is_leaf() {
                return root;
            }

            let mut children: Vec<Tree<Symbol>> = Vec::with_capacity(root.children.len());

            while !root.children.is_empty() {
                let child = &root.children[root.children.len() - 1];

                if child.spm == SymbolParseMethod::Repeated && root.children.len() > 1 {
                    let mut top = root.children.len() - 1;
                    let mut end = top - 1;

                    loop {
                        let other_child = &root.children[end];
                        if *other_child.lhs.kind() == *child.lhs.kind()
                            && other_child.spm == SymbolParseMethod::Repeated
                        {
                            top = end;
                        } else if other_child.spm != SymbolParseMethod::Injected {
                            break;
                        }

                        if end == 0 {
                            break;
                        }

                        end -= 1;
                    }

                    if top == root.children.len() - 1 {
                        // Do not push down single nodes
                        children.push(push_down_inline_lists(root.children.pop().unwrap()))
                    } else {
                        let sub_children = root
                            .children
                            .drain(top..)
                            .map(|mut t| {
                                t.spm = SymbolParseMethod::Standard;
                                push_down_inline_lists(t)
                            })
                            .collect();

                        children.push(Tree {
                            lhs: Token::null(),
                            children: sub_children,
                            production: None,
                            spm: SymbolParseMethod::Repeated,
                        });
                    }
                } else {
                    children.push(push_down_inline_lists(root.children.pop().unwrap()))
                }
            }

            children.reverse();
            root.children = children;
            root
        }

        fn parse_bottom_up<'scope, Symbol: GrammarSymbol>(
            grammar: &'scope dyn Grammar<Symbol>,
            lex: &'scope [Token<Symbol>],
            chart: PChart<'scope, Symbol>,
        ) -> Tree<Symbol> {
            let mut weight_map: HashMap<&Edge<Symbol>, usize> = HashMap::new();
            let mut nlp_map: HashMap<&Edge<Symbol>, ParsePath<Symbol>> = HashMap::new();

            let mut ordered_edges: Vec<&Edge<Symbol>> = Vec::new();
            for row in 0..chart.len() {
                chart
                    .row(row)
                    .edges
                    .iter()
                    .filter(|edge| !edge.is_empty())
                    .filter(|edge| !edge.is_terminal(grammar))
                    .for_each(|edge| {
                        ordered_edges.push(edge);
                    })
            }

            ordered_edges.sort_unstable_by(|ref e1, ref e2| {
                // Order by increasing width, then decreasing depth
                match (e1.finish - e1.start).partial_cmp(&(e2.finish - e2.start)) {
                    Some(Ordering::Equal) => e2.depth.partial_cmp(&e1.depth),
                    x => x,
                }
                .unwrap()
            });

            for edge in &ordered_edges {
                let weighted_path = optimal_next_level_path(&edge, &weight_map, grammar, &chart);

                weight_map.insert(edge, weighted_path.weight);
                nlp_map.insert(edge, weighted_path.path);
            }

            let mut best_root_edge: &Edge<Symbol> = if ordered_edges.is_empty() {
                chart
                    .row(0)
                    .edges
                    .iter()
                    .find(|edge| edge.depth == 0)
                    .unwrap()
            } else {
                ordered_edges.last().unwrap()
            };

            let mut best_root_weight = *weight_map
                .get(best_root_edge)
                .unwrap_or(&best_root_edge.weight);

            let finish: Node = chart.len() - 1;

            for edge in ordered_edges.iter().rev().skip(1) {
                if edge.finish != finish || edge.rule.unwrap().lhs != *grammar.start() {
                    break;
                }

                let minimal_tree_weight = *weight_map.get(edge).unwrap_or(&edge.weight);

                if minimal_tree_weight < best_root_weight {
                    best_root_edge = edge;
                    best_root_weight = minimal_tree_weight;
                }
            }

            fn link_shallow_paths<'scope, Symbol: GrammarSymbol>(
                edge: &Edge<Symbol>,
                spm: SymbolParseMethod,
                grammar: &'scope dyn Grammar<Symbol>,
                lex: &'scope [Token<Symbol>],
                nlp_map: &HashMap<&Edge<Symbol>, ParsePath<Symbol>>,
            ) -> Tree<Symbol> {
                match edge.rule {
                    None => Tree {
                        lhs: lex[edge.start].clone(),
                        children: Vec::new(),
                        production: None,
                        spm,
                    },
                    Some(rule) => Tree {
                        lhs: Token::interior(rule.lhs.clone()),
                        children: {
                            if edge.is_empty() {
                                vec![Tree::null()]
                            } else if edge.is_terminal(grammar) {
                                vec![Tree {
                                    lhs: lex[edge.start].clone(),
                                    children: Vec::new(),
                                    production: None,
                                    spm: spm.clone(),
                                }]
                            } else {
                                let path = nlp_map.get(edge).unwrap();
                                let edges = path.len();
                                path.iter()
                                    .enumerate()
                                    .filter(|(_, ref inner_edge)| !inner_edge.ignored)
                                    .rev()
                                    .map(|(i, ref inner_edge)| {
                                        let (_, spm, _) = edge.symbol_at(edges - i - 1);
                                        link_shallow_paths(inner_edge, spm, grammar, lex, nlp_map)
                                    })
                                    .collect()
                            }
                        },
                        production: match edge.rule {
                            Some(production) => Some(production.clone()),
                            None => None,
                        },
                        spm,
                    },
                }
            }

            fn optimal_next_level_path<'scope, Symbol: GrammarSymbol>(
                edge: &Edge<'scope, Symbol>,
                weight_map: &HashMap<&'scope Edge<Symbol>, usize>,
                grammar: &'scope dyn Grammar<Symbol>,
                chart: &'scope PChart<'scope, Symbol>,
            ) -> WeightedParsePath<'scope, Symbol> {
                fn df_search<'scope, Symbol: GrammarSymbol>(
                    depth: usize,
                    root: Node,
                    bottom: usize,
                    root_edge: &Edge<'scope, Symbol>,
                    weight_map: &HashMap<&'scope Edge<Symbol>, usize>,
                    grammar: &'scope dyn Grammar<Symbol>,
                    chart: &'scope PChart<'scope, Symbol>,
                ) -> Option<WeightedParsePath<'scope, Symbol>> {
                    if depth == bottom {
                        if root == root_edge.finish {
                            Some(WeightedParsePath::empty())
                        } else {
                            None
                        }
                    } else {
                        let (symbol, spm, reps) = root_edge.symbol_at(depth);

                        if reps > 1 {
                            // For repeated symbols, perform the DFS iteratively to avoid stack
                            // overflow on long lists.
                            if !grammar.is_non_terminal(symbol) {
                                let mut edge = Edge::terminal(root + reps, spm);

                                if let Some(mut path) = df_search(
                                    depth + reps,
                                    root + reps,
                                    bottom,
                                    root_edge,
                                    weight_map,
                                    grammar,
                                    chart,
                                ) {
                                    for _ in 0..reps {
                                        edge.start -= 1;
                                        edge.finish -= 1;
                                        path.append(edge.clone(), 0);
                                    }
                                    return Some(path);
                                }
                            } else if root < chart.len() {
                                let mut work_stack: LinkedList<(
                                    usize,
                                    usize,
                                    Option<&Edge<Symbol>>,
                                )> = LinkedList::new();
                                let mut edge_stack: Vec<&Edge<Symbol>> = Vec::with_capacity(reps);

                                work_stack.push_front((0, root, None));

                                let mut best_path: Option<WeightedParsePath<'scope, Symbol>> = None;

                                while !work_stack.is_empty() {
                                    let (count, finish, edge_opt) = work_stack.pop_front().unwrap();
                                    if count <= edge_stack.len() && count > 0 {
                                        edge_stack.drain(count - 1..);
                                    }
                                    if let Some(edge) = edge_opt {
                                        edge_stack.push(edge);
                                    }

                                    if count == reps {
                                        if let Some(mut path) = df_search(
                                            depth + reps,
                                            finish,
                                            bottom,
                                            root_edge,
                                            weight_map,
                                            grammar,
                                            chart,
                                        ) {
                                            let tree_list_weight: usize = edge_stack
                                                .iter()
                                                .map(|edge| {
                                                    *weight_map.get(edge).unwrap_or(&edge.weight)
                                                })
                                                .sum();

                                            if best_path.is_none()
                                                || path.weight + tree_list_weight
                                                    < best_path.as_ref().unwrap().weight
                                            {
                                                edge_stack.reverse();
                                                for edge in &edge_stack {
                                                    path.append((*edge).clone(), 0)
                                                }
                                                edge_stack.clear();
                                                path.weight += tree_list_weight;
                                                best_path = Some(path);
                                            }
                                        }
                                    } else {
                                        for edge in &chart.row(finish).edges {
                                            if edge.rule.unwrap().lhs == *symbol {
                                                work_stack.push_front((
                                                    count + 1,
                                                    edge.finish,
                                                    Some(edge),
                                                ));
                                            }
                                        }
                                    }
                                }

                                return best_path;
                            }
                        } else if !grammar.is_non_terminal(symbol) {
                            let edge = Edge::terminal(root, spm);

                            if let Some(mut path) = df_search(
                                depth + 1,
                                edge.finish,
                                bottom,
                                root_edge,
                                weight_map,
                                grammar,
                                chart,
                            ) {
                                path.append(edge, 0);
                                return Some(path);
                            }
                        } else if root < chart.len() {
                            let mut best_path: Option<WeightedParsePath<'scope, Symbol>> = None;

                            chart
                                .row(root)
                                .edges
                                .iter()
                                .filter(|edge| edge.rule.unwrap().lhs == *symbol)
                                .for_each(|edge| {
                                    if let Some(mut path) = df_search(
                                        depth + 1,
                                        edge.finish,
                                        bottom,
                                        root_edge,
                                        weight_map,
                                        grammar,
                                        chart,
                                    ) {
                                        let tree_weight =
                                            *weight_map.get(&edge).unwrap_or(&edge.weight);

                                        if best_path.is_none()
                                            || path.weight + tree_weight
                                                < best_path.as_ref().unwrap().weight
                                        {
                                            path.append(edge.clone(), tree_weight);
                                            best_path = Some(path);
                                        }
                                    }
                                });

                            return best_path;
                        }

                        None
                    }
                }

                let bottom = edge.symbols_len();
                match df_search(0, edge.start, bottom, edge, weight_map, grammar, chart) {
                    None => panic!("Failed to decompose parse edge of recognized lex"),
                    Some(mut path) => {
                        path.weight += edge.weight;
                        path
                    }
                }
            }

            link_shallow_paths(
                best_root_edge,
                SymbolParseMethod::Standard,
                grammar,
                lex,
                &nlp_map,
            )
        }

        fn parse_top_down<'scope, Symbol: GrammarSymbol>(
            grammar: &'scope dyn Grammar<Symbol>,
            lex: &'scope [Token<Symbol>],
            chart: PChart<'scope, Symbol>,
        ) -> Tree<Symbol> {
            fn recur<'scope, Symbol: GrammarSymbol>(
                edge: &Edge<Symbol>,
                spm: SymbolParseMethod,
                grammar: &'scope dyn Grammar<Symbol>,
                lex: &'scope [Token<Symbol>],
                chart: &PChart<Symbol>,
            ) -> Tree<Symbol> {
                match edge.rule {
                    None => Tree {
                        //Non-empty rhs
                        lhs: lex[edge.start].clone(),
                        children: Vec::new(),
                        production: None,
                        spm,
                    },
                    Some(rule) => Tree {
                        lhs: Token::interior(rule.lhs.clone()),
                        children: {
                            let path = top_list(edge, grammar, chart);
                            let edges = path.len();
                            let mut children: Vec<Tree<Symbol>> = path
                                .into_iter()
                                .enumerate()
                                .rev()
                                .map(|(i, ref inner_edge)| {
                                    let (_, spm, _) = edge.symbol_at(edges - i - 1);
                                    recur(&inner_edge, spm, grammar, lex, chart)
                                })
                                .collect();
                            if children.is_empty() {
                                //Empty rhs
                                children.push(Tree::null());
                            }
                            children
                        },
                        production: match edge.rule {
                            Some(production) => Some(production.clone()),
                            None => None,
                        },
                        spm,
                    },
                }
            }

            fn top_list<'scope, Symbol: GrammarSymbol>(
                edge: &'scope Edge<Symbol>,
                grammar: &'scope dyn Grammar<Symbol>,
                chart: &'scope PChart<'scope, Symbol>,
            ) -> ParsePath<'scope, Symbol> {
                fn df_search<'scope, Symbol: GrammarSymbol>(
                    depth: usize,
                    root: Node,
                    bottom: usize,
                    root_edge: &Edge<'scope, Symbol>,
                    grammar: &'scope dyn Grammar<Symbol>,
                    chart: &'scope PChart<'scope, Symbol>,
                ) -> Option<ParsePath<'scope, Symbol>> {
                    if depth == bottom {
                        if root == root_edge.finish {
                            Some(ParsePath::new())
                        } else {
                            None
                        }
                    } else {
                        let (symbol, spm, reps) = root_edge.symbol_at(depth);

                        if reps > 1 {
                            // For repeated symbols, perform the DFS iteratively to avoid stack
                            // overflow on long lists.
                            if !grammar.is_non_terminal(symbol) {
                                let mut edge = Edge::terminal(root + reps, spm);

                                if let Some(mut path) = df_search(
                                    depth + reps,
                                    root + reps,
                                    bottom,
                                    root_edge,
                                    grammar,
                                    chart,
                                ) {
                                    for _ in 0..reps {
                                        edge.start -= 1;
                                        edge.finish -= 1;
                                        path.push(edge.clone());
                                    }
                                    return Some(path);
                                }
                            } else if root < chart.len() {
                                let mut work_stack: LinkedList<(
                                    usize,
                                    usize,
                                    Option<&Edge<Symbol>>,
                                )> = LinkedList::new();
                                let mut edge_stack: Vec<&Edge<Symbol>> = Vec::with_capacity(reps);

                                work_stack.push_front((0, root, None));

                                while !work_stack.is_empty() {
                                    let (count, finish, edge_opt) = work_stack.pop_front().unwrap();
                                    if count <= edge_stack.len() && count > 0 {
                                        edge_stack.drain(count - 1..);
                                    }
                                    if let Some(edge) = edge_opt {
                                        edge_stack.push(edge);
                                    }

                                    if count == reps {
                                        if let Some(mut path) = df_search(
                                            depth + reps,
                                            finish,
                                            bottom,
                                            root_edge,
                                            grammar,
                                            chart,
                                        ) {
                                            edge_stack.reverse();
                                            for edge in edge_stack {
                                                path.push(edge.clone())
                                            }
                                            return Some(path);
                                        }
                                    } else {
                                        for edge in &chart.row(finish).edges {
                                            if edge.rule.unwrap().lhs == *symbol {
                                                work_stack.push_front((
                                                    count + 1,
                                                    edge.finish,
                                                    Some(edge),
                                                ));
                                            }
                                        }
                                    }
                                }
                            }
                        } else if !grammar.is_non_terminal(symbol) {
                            if let Some(mut path) =
                                df_search(depth + 1, root + 1, bottom, root_edge, grammar, chart)
                            {
                                path.push(Edge::terminal(root, spm));
                                return Some(path);
                            }
                        } else if root < chart.len() {
                            for edge in &chart.row(root).edges {
                                if edge.rule.unwrap().lhs == *symbol {
                                    if let Some(mut path) = df_search(
                                        depth + 1,
                                        edge.finish,
                                        bottom,
                                        root_edge,
                                        grammar,
                                        chart,
                                    ) {
                                        path.push(edge.clone());
                                        return Some(path);
                                    }
                                }
                            }
                        }

                        None
                    }
                }

                let bottom = edge.symbols_len();
                df_search(0, edge.start, bottom, edge, grammar, chart)
                    .expect("Failed to decompose parse edge of recognized lex")
            }

            let finish: Node = chart.len() - 1;

            let root_edge =
                chart.row(0).edges.iter().find(|edge| {
                    edge.finish == finish && edge.rule.unwrap().lhs == *grammar.start()
                });

            match root_edge {
                None => panic!("Failed to find start item to begin parse"),
                Some(edge) => recur(edge, SymbolParseMethod::Standard, grammar, lex, &chart),
            }
        }
    }
}

struct RChart<'item, Symbol: GrammarSymbol + 'item> {
    rows: Vec<RChartRow<'item, Symbol>>,
}

impl<'item, Symbol: GrammarSymbol + 'item> RChart<'item, Symbol> {
    fn new() -> Self {
        Self {
            rows: vec![RChartRow::new(Vec::new())],
        }
    }

    fn len(&self) -> usize {
        self.rows.len()
    }

    fn add_row(&mut self, items: Vec<Item<'item, Symbol>>) {
        self.rows.push(RChartRow::new(items));
    }

    fn row(&self, i: usize) -> &RChartRow<'item, Symbol> {
        &self.rows[i]
    }

    fn row_mut(&mut self, i: usize) -> &mut RChartRow<'item, Symbol> {
        &mut self.rows[i]
    }

    #[allow(dead_code)]
    fn print(&self) {
        for i in 0..self.rows.len() {
            println!("ROW {}", i);
            println!("\tINCOMPLETE");
            for item in &self.rows[i].incomplete().items {
                println!("\t\t{}", item.to_string());
            }
            println!("\tCOMPLETE");
            for item in &self.rows[i].complete().items {
                println!("\t\t{}", item.to_string());
            }
        }
    }
}

struct RChartRow<'item, Symbol: GrammarSymbol + 'item> {
    incomplete: Items<'item, Symbol>,
    complete: Items<'item, Symbol>,
}

impl<'item, Symbol: GrammarSymbol + 'item> RChartRow<'item, Symbol> {
    fn new(scanned_items: Vec<Item<'item, Symbol>>) -> Self {
        let mut incomplete = Items::new();
        let mut complete = Items::new();

        for item in scanned_items {
            if item.is_complete() {
                complete.unsafe_append(item);
            } else {
                incomplete.unsafe_append(item);
            }
        }

        Self {
            incomplete,
            complete,
        }
    }

    fn unsafe_append(&mut self, item: Item<'item, Symbol>) {
        if item.is_complete() {
            self.complete.unsafe_append(item);
        } else {
            self.incomplete.unsafe_append(item);
        }
    }

    fn contains(&self, item: &Item<'item, Symbol>) -> bool {
        if item.is_complete() {
            self.complete.contains(item)
        } else {
            self.incomplete.contains(item)
        }
    }

    fn incomplete(&self) -> &Items<'item, Symbol> {
        &self.incomplete
    }

    fn complete(&self) -> &Items<'item, Symbol> {
        &self.complete
    }
}

struct Items<'item, Symbol: GrammarSymbol + 'item> {
    items: Vec<Item<'item, Symbol>>,
}

impl<'item, Symbol: GrammarSymbol + 'item> Items<'item, Symbol> {
    fn new() -> Self {
        Self { items: Vec::new() }
    }

    fn len(&self) -> usize {
        self.items.len()
    }

    fn unsafe_append(&mut self, item: Item<'item, Symbol>) {
        self.items.push(item);
    }

    fn item(&self, i: usize) -> &Item<'item, Symbol> {
        &self.items[i]
    }

    fn contains(&self, item: &Item<'item, Symbol>) -> bool {
        self.items.contains(item)
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
struct Item<'rule, Symbol: GrammarSymbol + 'rule> {
    rule: &'rule Production<Symbol>,
    shadow: Option<Vec<ShadowSymbol<Symbol>>>,
    shadow_top: usize,
    start: usize,
    next: usize,
    depth: usize,
    ignore_next: bool,
    weight: usize,
}

impl<'rule, Symbol: GrammarSymbol + 'rule> Item<'rule, Symbol> {
    fn start(rule: &'rule Production<Symbol>) -> Self {
        Self {
            rule,
            shadow: None,
            shadow_top: 0,
            start: 0,
            next: 0,
            depth: 0,
            ignore_next: false,
            weight: 0,
        }
    }

    fn advance(&mut self) {
        self.next += 1;
        self.ignore_next = false;
    }

    fn advance_new(&self) -> Self {
        Self {
            rule: self.rule,
            shadow: self.shadow.clone(),
            shadow_top: self.shadow_top,
            start: self.start,
            next: self.next + 1,
            depth: self.depth,
            ignore_next: false,
            weight: self.weight,
        }
    }

    fn next_symbol<'scope>(&'scope self) -> Option<&'rule Symbol> {
        if self.next < self.rule.rhs.len() {
            Some(&self.rule.rhs[self.next].symbol)
        } else {
            None
        }
    }

    fn prev_symbol<'scope>(&'scope self) -> Option<&'rule Symbol> {
        if self.next > 0 {
            self.rule.rhs.get(self.next - 1).map(|sym| &sym.symbol)
        } else {
            None
        }
    }

    fn next_prod_symbol<'scope>(&'scope self) -> Option<&'rule ProductionSymbol<Symbol>> {
        if self.next < self.rule.rhs.len() {
            Some(&self.rule.rhs[self.next])
        } else {
            None
        }
    }

    fn is_complete(&self) -> bool {
        self.next >= self.rule.rhs.len()
    }

    fn shadow_copy(&self) -> Vec<ShadowSymbol<Symbol>> {
        match self.shadow {
            Some(ref shadow_vec) => shadow_vec.clone(),
            None => Vec::new(),
        }
    }

    fn extend_shadow(&self, shadow: &mut Vec<ShadowSymbol<Symbol>>) {
        for i in self.shadow_top..self.next {
            shadow.push(ShadowSymbol {
                symbol: self.rule.rhs[i].symbol.clone(),
                spm: SymbolParseMethod::Standard,
                reps: 1,
            });
        }
    }

    fn can_extend_list(&self, symbol: &Symbol) -> bool {
        if self.shadow_top >= self.next && self.shadow.is_some() {
            if let Some(shadow_vec) = &self.shadow {
                let previous = shadow_vec.last().unwrap();
                if previous.symbol == *symbol && previous.spm == SymbolParseMethod::Repeated {
                    return true;
                }
            }
        }

        false
    }
}

impl<'rule, Symbol: GrammarSymbol> Data for Item<'rule, Symbol> {
    fn to_string(&self) -> String {
        let mut rule_string = format!("{:?} -{}-> ", self.rule.lhs, self.weight);
        for i in 0..self.rule.rhs.len() {
            if i == self.next {
                rule_string.push_str(". ");
            }
            rule_string = format!("{}{:?} ", rule_string, self.rule.rhs[i]);
        }
        if self.next == self.rule.rhs.len() {
            rule_string.push_str(". ");
        }
        format!(
            "{} ({:?}) shadow:{:?} at {}",
            rule_string, self.start, self.shadow, self.shadow_top
        )
    }
}

struct PChart<'rule, Symbol: GrammarSymbol + 'rule> {
    rows: Vec<PChartRow<'rule, Symbol>>,
}

impl<'rule, Symbol: GrammarSymbol + 'rule> PChart<'rule, Symbol> {
    fn new() -> Self {
        Self {
            rows: vec![PChartRow::new()],
        }
    }

    fn len(&self) -> usize {
        self.rows.len()
    }

    fn add_row(&mut self) {
        self.rows.push(PChartRow::new());
    }

    fn row(&self, i: usize) -> &PChartRow<Symbol> {
        &self.rows[i]
    }

    fn row_mut(&mut self, i: usize) -> &mut PChartRow<'rule, Symbol> {
        &mut self.rows[i]
    }

    #[allow(dead_code)]
    fn print(&self) {
        for i in 0..self.rows.len() {
            println!("ROW {}", i);
            for edge in &self.rows[i].edges {
                println!("\t{}", edge.to_string());
            }
        }
    }
}

struct PChartRow<'rule, Symbol: GrammarSymbol + 'rule> {
    edges: Vec<Edge<'rule, Symbol>>,
}

impl<'rule, Symbol: GrammarSymbol + 'rule> PChartRow<'rule, Symbol> {
    fn new() -> Self {
        Self { edges: Vec::new() }
    }

    fn add_edge(&mut self, edge: Edge<'rule, Symbol>) {
        self.edges.push(edge);
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
struct Edge<'prod, Symbol: GrammarSymbol + 'prod> {
    rule: Option<&'prod Production<Symbol>>,
    shadow: Option<Vec<ShadowSymbol<Symbol>>>,
    shadow_top: usize,
    shadow_len: usize,
    start: usize,
    finish: usize,
    ignored: bool,
    weight: usize,
    depth: usize,
}

impl<'prod, Symbol: GrammarSymbol + 'prod> Edge<'prod, Symbol> {
    fn terminal(start: usize, spm: SymbolParseMethod) -> Self {
        Self {
            rule: None,
            shadow: None,
            shadow_top: 0,
            shadow_len: 0,
            start,
            finish: start + 1,
            ignored: spm == SymbolParseMethod::Ignored,
            weight: 0,
            depth: 0,
        }
    }

    fn symbols_len(&self) -> usize {
        match self.rule {
            Some(rule) => self.shadow_len + rule.rhs.len() - self.shadow_top,
            None => 0,
        }
    }

    fn shadow_len(shadow: &Option<Vec<ShadowSymbol<Symbol>>>) -> usize {
        match shadow {
            Some(ref shadow) => {
                let mut repeated = 0;
                for item in shadow {
                    if item.spm == SymbolParseMethod::Repeated {
                        repeated += (item.reps - 1) as usize;
                    }
                }

                shadow.len() + repeated
            }
            None => 0,
        }
    }

    fn symbol_at(&self, index: usize) -> (&Symbol, SymbolParseMethod, usize) {
        if index < self.shadow_len {
            self.shadow_symbol_at(index)
        } else {
            (
                &self.rule.unwrap().rhs[index - self.shadow_len + self.shadow_top].symbol,
                SymbolParseMethod::Standard,
                1,
            )
        }
    }

    fn shadow_symbol_at(&self, index: usize) -> (&Symbol, SymbolParseMethod, usize) {
        let shadow = self.shadow.as_ref().unwrap();

        // Fast-path when no repeating symbols.
        if shadow.len() == self.shadow_len {
            let shadow_symbol = &shadow[index];
            return (
                &shadow_symbol.symbol,
                shadow_symbol.spm.clone(),
                shadow_symbol.reps,
            );
        }

        let mut expanded_index = 0;
        for (real_index, item) in shadow.iter().enumerate() {
            expanded_index += item.reps;

            if expanded_index > index {
                let shadow_symbol = &shadow[real_index];
                return (
                    &shadow_symbol.symbol,
                    shadow_symbol.spm.clone(),
                    shadow_symbol.reps,
                );
            }
        }

        panic!(
            "Shadow symbol index {} out of bounds, length is {}",
            index, self.shadow_len
        );
    }

    fn is_terminal(&self, grammar: &dyn Grammar<Symbol>) -> bool {
        self.rule.unwrap().rhs.len() == 1
            && !grammar.is_non_terminal(&self.rule.unwrap().rhs[0].symbol)
            && self.shadow.is_none()
    }

    fn is_empty(&self) -> bool {
        let empty_normal = match self.rule {
            Some(rule) => rule.rhs.is_empty(),
            None => true,
        };

        let empty_shadow = match &self.shadow {
            Some(shadow) => shadow
                .iter()
                .find(|sym| sym.spm != SymbolParseMethod::Ignored)
                .is_none(),
            None => true,
        };

        empty_normal && empty_shadow
    }
}

impl<'prod, Symbol: GrammarSymbol + 'prod> Data for Edge<'prod, Symbol> {
    fn to_string(&self) -> String {
        match self.rule {
            None => format!("NONE ({})", self.finish),
            Some(rule) => {
                let mut rule_string = format!("{:?} -{}-> ", rule.lhs, self.weight);
                for i in 0..rule.rhs.len() {
                    rule_string = format!("{}{:?} ", rule_string, rule.rhs[i]);
                }
                format!(
                    "{} ({}) shadow: {:?} at {}",
                    rule_string, self.finish, self.shadow, self.shadow_top
                )
            }
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
struct ShadowSymbol<Symbol: GrammarSymbol> {
    symbol: Symbol,
    spm: SymbolParseMethod,
    reps: usize,
}

type Node = usize;
type ParsePath<'rule, Symbol> = Vec<Edge<'rule, Symbol>>;

struct WeightedParsePath<'rule, Symbol: GrammarSymbol> {
    path: ParsePath<'rule, Symbol>,
    weight: usize,
}

impl<'rule, Symbol: GrammarSymbol> WeightedParsePath<'rule, Symbol> {
    fn empty() -> Self {
        Self {
            path: Vec::new(),
            weight: 0,
        }
    }

    fn append(&mut self, edge: Edge<'rule, Symbol>, weight: usize) {
        self.weight += weight;
        self.path.push(edge);
    }
}
