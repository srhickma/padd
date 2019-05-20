use {
    core::{
        data::Data,
        parse::{
            self,
            grammar::{Grammar, GrammarSymbol},
            Parser, Production, Tree,
        },
        scan::Token,
    },
    std::{
        cmp::Ordering,
        collections::{HashMap, HashSet},
        usize,
    },
};

pub struct EarleyParser;

impl<Symbol: GrammarSymbol> Parser<Symbol> for EarleyParser {
    fn parse(
        &self,
        scan: Vec<Token<Symbol>>,
        grammar: &Grammar<Symbol>,
    ) -> Result<Tree<Symbol>, parse::Error> {
        let mut chart: RChart<Symbol> = RChart::new();
        let mut parse_chart: PChart<Symbol> = PChart::new();

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
            scan_full(cursor, &scan, grammar, &mut chart, &mut parse_chart);

            cursor += 1;
        }

        fn complete_full<'inner, 'grammar: 'inner, Symbol: GrammarSymbol>(
            cursor: usize,
            grammar: &'grammar Grammar<Symbol>,
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
            grammar: &'grammar Grammar<Symbol>,
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
                if !item.ignore_next {
                    mark_completed_item(&item, cursor, parse_chart);
                }
            }
        }

        fn scan_full<'inner, 'grammar: 'inner, Symbol: GrammarSymbol>(
            cursor: usize,
            scan: &[Token<Symbol>],
            grammar: &'grammar Grammar<Symbol>,
            chart: &'inner mut RChart<'grammar, Symbol>,
            parse_chart: &mut PChart<'grammar, Symbol>,
        ) {
            if cursor == scan.len() {
                return;
            }

            let symbol = scan[cursor].kind();

            let next_row = if grammar.is_ignorable(symbol) {
                cross_shadow_symbol(
                    chart.row(cursor),
                    symbol,
                    SymbolParseMethod::Ignored,
                    grammar,
                )
            } else if grammar.is_injectable(symbol) {
                cross_shadow_symbol(
                    chart.row(cursor),
                    symbol,
                    SymbolParseMethod::Injected,
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
            grammar: &'grammar Grammar<Symbol>,
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
            grammar: &'grammar Grammar<Symbol>,
        ) -> Vec<Item<'grammar, Symbol>> {
            let mut dest: Vec<Item<Symbol>> = Vec::new();

            for item in src {
                if let Some(sym) = item.next_symbol() {
                    if sym == symbol {
                        advance_past_symbol(item, &mut dest, grammar);
                    }
                }
            }

            dest
        }

        fn cross_shadow_symbol<'inner, 'grammar: 'inner, Symbol: GrammarSymbol>(
            src: &'inner RChartRow<'grammar, Symbol>,
            symbol: &Symbol,
            spm: SymbolParseMethod,
            grammar: &'grammar Grammar<Symbol>,
        ) -> Vec<Item<'grammar, Symbol>> {
            let mut dest: Vec<Item<Symbol>> = Vec::new();

            for item in &src.incomplete.items {
                if item.next_symbol().unwrap() == symbol {
                    advance_past_symbol(item, &mut dest, grammar);
                } else {
                    dest.push(advance_via_shadow(item, symbol, spm.clone()));
                }
            }

            for item in &src.complete.items {
                dest.push(advance_via_shadow(item, symbol, spm.clone()));
            }

            dest
        }

        fn advance_past_symbol<'inner, 'grammar: 'inner, Symbol: GrammarSymbol>(
            item: &'inner Item<'grammar, Symbol>,
            dest: &mut Vec<Item<'grammar, Symbol>>,
            grammar: &'grammar Grammar<Symbol>,
        ) {
            let mut last_item = item.clone();

            loop {
                last_item.advance();

                dest.push(last_item.clone());

                match last_item.next_symbol() {
                    None => break,
                    Some(sym) => {
                        if !grammar.is_nullable_nt(sym) {
                            break;
                        }
                    }
                }
            }
        }

        fn advance_via_shadow<'inner, 'grammar: 'inner, Symbol: GrammarSymbol>(
            item: &'inner Item<'grammar, Symbol>,
            symbol: &Symbol,
            spm: SymbolParseMethod,
        ) -> Item<'grammar, Symbol> {
            let mut shadow_vec = match item.shadow {
                Some(ref shadow_vec) => shadow_vec.clone(),
                None => Vec::new(),
            };

            for i in item.shadow_top..item.next {
                shadow_vec.push(ShadowSymbol {
                    symbol: item.rule.rhs[i].clone(),
                    spm: SymbolParseMethod::Standard,
                });
            }

            shadow_vec.push(ShadowSymbol {
                symbol: symbol.clone(),
                spm,
            });

            Item {
                rule: item.rule,
                shadow: Some(shadow_vec),
                shadow_top: item.next,
                start: item.start,
                next: item.next,
                depth: item.depth,
                ignore_next: true,
            }
        }

        fn mark_completed_item<'inner, 'grammar: 'inner, Symbol: GrammarSymbol>(
            item: &'inner Item<'grammar, Symbol>,
            finish: usize,
            parse_chart: &mut PChart<'grammar, Symbol>,
        ) {
            let weight = match item.shadow {
                None => 0,
                Some(ref shadow_vec) => shadow_vec.len() - item.shadow_top + 1,
            };

            parse_chart.row_mut(item.start).add_edge(Edge {
                rule: Some(item.rule),
                shadow: item.shadow.clone(),
                shadow_top: item.shadow_top,
                start: item.start,
                finish,
                spm: SymbolParseMethod::Standard,
                weight,
                depth: item.depth,
            });
        }

        fn recognized<Symbol: GrammarSymbol>(
            grammar: &Grammar<Symbol>,
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
            if cursor - 1 == scan.len() {
                Ok(parse_tree(grammar, &scan, parse_chart))
            } else {
                Err(parse::Error {
                    message: format!(
                        "Largest parse did not consume all tokens: {} of {}",
                        cursor - 1,
                        scan.len()
                    ),
                })
            }
        } else if scan.is_empty() {
            Err(parse::Error {
                message: "No tokens scanned".to_string(),
            })
        } else if cursor - 1 == scan.len() {
            Err(parse::Error {
                message: "Recognition failed after consuming all tokens".to_string(),
            })
        } else {
            let token = &scan[cursor - 1];
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
            grammar: &'scope Grammar<Symbol>,
            scan: &'scope [Token<Symbol>],
            chart: PChart<'scope, Symbol>,
        ) -> Tree<Symbol> {
            if grammar.weighted_parse() {
                parse_bottom_up(grammar, scan, chart)
            } else {
                parse_top_down(grammar, scan, chart)
            }
        }

        fn parse_bottom_up<'scope, Symbol: GrammarSymbol>(
            grammar: &'scope Grammar<Symbol>,
            scan: &'scope [Token<Symbol>],
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
                    .filter(|edge| !edge.rule.unwrap().rhs.is_empty())
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
                grammar: &'scope Grammar<Symbol>,
                scan: &'scope [Token<Symbol>],
                nlp_map: &HashMap<&Edge<Symbol>, ParsePath<Symbol>>,
            ) -> Tree<Symbol> {
                match edge.rule {
                    None => Tree {
                        lhs: scan[edge.start].clone(),
                        children: Vec::new(),
                        injected: edge.spm == SymbolParseMethod::Injected,
                    },
                    Some(rule) => Tree {
                        lhs: Token::interior(rule.lhs.clone()),
                        children: {
                            if edge.rule.unwrap().rhs.is_empty() {
                                vec![Tree::null()]
                            } else if edge.is_terminal(grammar) {
                                vec![Tree {
                                    lhs: scan[edge.start].clone(),
                                    children: Vec::new(),
                                    injected: edge.spm == SymbolParseMethod::Injected,
                                }]
                            } else {
                                nlp_map
                                    .get(edge)
                                    .unwrap()
                                    .iter()
                                    .filter(|ref edge| edge.spm != SymbolParseMethod::Ignored)
                                    .rev()
                                    .map(|edge| link_shallow_paths(edge, grammar, scan, nlp_map))
                                    .collect()
                            }
                        },
                        injected: false,
                    },
                }
            }

            fn optimal_next_level_path<'scope, Symbol: GrammarSymbol>(
                edge: &Edge<'scope, Symbol>,
                weight_map: &HashMap<&'scope Edge<Symbol>, usize>,
                grammar: &'scope Grammar<Symbol>,
                chart: &'scope PChart<'scope, Symbol>,
            ) -> WeightedParsePath<'scope, Symbol> {
                fn df_search<'scope, Symbol: GrammarSymbol>(
                    depth: usize,
                    root: Node,
                    bottom: usize,
                    root_edge: &Edge<'scope, Symbol>,
                    weight_map: &HashMap<&'scope Edge<Symbol>, usize>,
                    grammar: &'scope Grammar<Symbol>,
                    chart: &'scope PChart<'scope, Symbol>,
                ) -> Option<WeightedParsePath<'scope, Symbol>> {
                    if depth == bottom {
                        if root == root_edge.finish {
                            Some(WeightedParsePath::empty())
                        } else {
                            None
                        }
                    } else {
                        let (symbol, spm) = root_edge.symbol_at(depth);

                        if !grammar.is_non_terminal(symbol) {
                            let edge = Edge {
                                rule: None,
                                shadow: None,
                                shadow_top: 0,
                                start: root,
                                finish: root + 1,
                                spm,
                                weight: 0,
                                depth: 0,
                            };

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
                                Some(path)
                            } else {
                                None
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

                            best_path
                        } else {
                            None
                        }
                    }
                }

                let bottom = edge.symbols_len();
                match df_search(0, edge.start, bottom, edge, weight_map, grammar, chart) {
                    None => panic!("Failed to decompose parse edge of recognized scan"),
                    Some(mut path) => {
                        path.weight += edge.weight;
                        path
                    }
                }
            }

            link_shallow_paths(best_root_edge, grammar, scan, &nlp_map)
        }

        fn parse_top_down<'scope, Symbol: GrammarSymbol>(
            grammar: &'scope Grammar<Symbol>,
            scan: &'scope [Token<Symbol>],
            chart: PChart<'scope, Symbol>,
        ) -> Tree<Symbol> {
            fn recur<'scope, Symbol: GrammarSymbol>(
                edge: &Edge<Symbol>,
                grammar: &'scope Grammar<Symbol>,
                scan: &'scope [Token<Symbol>],
                chart: &PChart<Symbol>,
            ) -> Tree<Symbol> {
                match edge.rule {
                    None => Tree {
                        //Non-empty rhs
                        lhs: scan[edge.start].clone(),
                        children: Vec::new(),
                        injected: false,
                    },
                    Some(rule) => Tree {
                        lhs: Token::interior(rule.lhs.clone()),
                        children: {
                            let mut children: Vec<Tree<Symbol>> = top_list(edge, grammar, chart)
                                .iter()
                                .filter(|ref edge| edge.spm != SymbolParseMethod::Ignored)
                                .rev()
                                .map(|ref edge| recur(&edge, grammar, scan, chart))
                                .collect();
                            if children.is_empty() {
                                //Empty rhs
                                children.push(Tree::null());
                            }
                            children
                        },
                        injected: false,
                    },
                }
            }

            fn top_list<'scope, Symbol: GrammarSymbol>(
                edge: &Edge<Symbol>,
                grammar: &'scope Grammar<Symbol>,
                chart: &'scope PChart<'scope, Symbol>,
            ) -> ParsePath<'scope, Symbol> {
                let bottom: usize = edge.symbols_len();
                let leaf = |depth: usize, node: Node| depth == bottom && node == edge.finish;
                let edges = |depth: usize, node: Node| -> Vec<Edge<Symbol>> {
                    if depth < bottom {
                        let (symbol, spm) = edge.symbol_at(depth);
                        if !grammar.is_non_terminal(symbol) {
                            return vec![Edge {
                                rule: None,
                                shadow: None,
                                shadow_top: 0,
                                start: node,
                                finish: node + 1,
                                spm,
                                weight: 0,
                                depth: 0,
                            }];
                        } else if node < chart.len() {
                            return chart
                                .row(node)
                                .edges
                                .iter()
                                .filter(|edge| edge.rule.unwrap().lhs == *symbol)
                                .cloned()
                                .collect();
                        }
                    }
                    Vec::new()
                };

                fn df_search<'scope, Symbol: GrammarSymbol>(
                    edges: &Fn(usize, Node) -> Vec<Edge<'scope, Symbol>>,
                    leaf: &Fn(usize, Node) -> bool,
                    depth: usize,
                    root: Node,
                ) -> Option<ParsePath<'scope, Symbol>> {
                    if leaf(depth, root) {
                        Some(ParsePath::new())
                    } else {
                        for edge in edges(depth, root) {
                            if let Some(mut path) = df_search(edges, leaf, depth + 1, edge.finish) {
                                path.push(edge);
                                return Some(path);
                            }
                        }
                        None
                    }
                }

                match df_search(&edges, &leaf, 0, edge.start) {
                    None => panic!("Failed to decompose parse edge of recognized scan"),
                    Some(path) => path,
                }
            }

            let finish: Node = chart.len() - 1;

            let root_edge =
                chart.row(0).edges.iter().find(|edge| {
                    edge.finish == finish && edge.rule.unwrap().lhs == *grammar.start()
                });

            match root_edge {
                None => panic!("Failed to find start item to begin parse"),
                Some(edge) => recur(edge, grammar, scan, &chart),
            }
        }
    }
}

struct RChart<'item, Symbol: GrammarSymbol + 'item> {
    rows: Vec<RChartRow<'item, Symbol>>,
}

impl<'item, Symbol: GrammarSymbol + 'item> RChart<'item, Symbol> {
    fn new() -> Self {
        RChart {
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
    fn new(scanned_items: Vec<Item<Symbol>>) -> RChartRow<Symbol> {
        let mut incomplete = Items::new();
        let mut complete = Items::new();

        for item in scanned_items {
            if item.is_complete() {
                complete.unsafe_append(item);
            } else {
                incomplete.unsafe_append(item);
            }
        }

        RChartRow {
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
    fn new() -> Items<'item, Symbol> {
        Items { items: Vec::new() }
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
}

impl<'rule, Symbol: GrammarSymbol + 'rule> Item<'rule, Symbol> {
    fn start(rule: &'rule Production<Symbol>) -> Self {
        Item {
            rule,
            shadow: None,
            shadow_top: 0,
            start: 0,
            next: 0,
            depth: 0,
            ignore_next: false,
        }
    }

    fn advance(&mut self) {
        self.next += 1;
        self.ignore_next = false;
    }

    fn advance_new(&self) -> Self {
        Item {
            rule: self.rule,
            shadow: self.shadow.clone(),
            shadow_top: self.shadow_top,
            start: self.start,
            next: self.next + 1,
            depth: self.depth,
            ignore_next: false,
        }
    }

    fn next_symbol<'scope>(&'scope self) -> Option<&'rule Symbol> {
        if self.next < self.rule.rhs.len() {
            Some(&self.rule.rhs[self.next])
        } else {
            None
        }
    }

    fn is_complete(&self) -> bool {
        self.next >= self.rule.rhs.len()
    }
}

impl<'rule, Symbol: GrammarSymbol> Data for Item<'rule, Symbol> {
    fn to_string(&self) -> String {
        let mut rule_string = format!("{:?} -> ", self.rule.lhs);
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
        PChart {
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
        PChartRow { edges: Vec::new() }
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
    start: usize,
    finish: usize,
    spm: SymbolParseMethod,
    weight: usize,
    depth: usize,
}

impl<'prod, Symbol: GrammarSymbol + 'prod> Edge<'prod, Symbol> {
    fn symbols_len(&self) -> usize {
        match self.rule {
            Some(rule) => match self.shadow {
                Some(ref shadow) => shadow.len() + rule.rhs.len() - self.shadow_top,
                None => rule.rhs.len(),
            },
            None => 0,
        }
    }

    fn symbol_at(&self, index: usize) -> (&Symbol, SymbolParseMethod) {
        if let Some(ref shadow) = self.shadow {
            if index < shadow.len() {
                let shadow_symbol = &shadow[index];
                (&shadow_symbol.symbol, shadow_symbol.spm.clone())
            } else {
                (
                    &self.rule.unwrap().rhs[index - shadow.len() + self.shadow_top],
                    SymbolParseMethod::Standard,
                )
            }
        } else {
            (&self.rule.unwrap().rhs[index], SymbolParseMethod::Standard)
        }
    }

    fn is_terminal(&self, grammar: &Grammar<Symbol>) -> bool {
        self.rule.unwrap().rhs.len() == 1 && !grammar.is_non_terminal(&self.rule.unwrap().rhs[0])
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
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
enum SymbolParseMethod {
    Standard,
    Ignored,
    Injected,
}

type Node = usize;
type ParsePath<'rule, Symbol> = Vec<Edge<'rule, Symbol>>;

struct WeightedParsePath<'rule, Symbol: GrammarSymbol> {
    path: ParsePath<'rule, Symbol>,
    weight: usize,
}

impl<'rule, Symbol: GrammarSymbol> WeightedParsePath<'rule, Symbol> {
    fn empty() -> Self {
        WeightedParsePath {
            path: Vec::new(),
            weight: 0,
        }
    }

    fn append(&mut self, edge: Edge<'rule, Symbol>, weight: usize) {
        self.weight += weight;
        self.path.push(edge);
    }
}
