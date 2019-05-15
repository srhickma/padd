use {
    core::{
        data::Data,
        parse::{self, grammar::Grammar, Parser, Production, Tree},
        scan::Token,
    },
    std::{
        cmp::Ordering,
        collections::{HashMap, HashSet},
        usize,
    },
};

pub struct EarleyParser;

impl<Symbol: Data + Default> Parser<Symbol> for EarleyParser {
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

        fn complete_full<'inner, 'grammar: 'inner, Symbol: Data + Default>(
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
                        chart.row(item.start).incomplete().items(),
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

        fn predict_full<'inner, 'grammar: 'inner, Symbol: Data + Default>(
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

        fn parse_mark_full<'inner, 'grammar: 'inner, Symbol: Data + Default>(
            cursor: usize,
            chart: &'inner RChart<'grammar, Symbol>,
            parse_chart: &mut PChart<'grammar, Symbol>,
        ) {
            for item in chart.row(cursor).complete().items() {
                if !item.ignore_next {
                    mark_completed_item(&item, cursor, parse_chart);
                }
            }
        }

        fn scan_full<'inner, 'grammar: 'inner, Symbol: Data + Default>(
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
                cross_ignorable(chart.row(cursor), symbol, grammar)
            } else {
                cross(chart.row(cursor).incomplete().items(), symbol, grammar)
            };

            if next_row.is_empty() {
                return;
            }

            chart.add_row(next_row);
            parse_chart.add_row();
        }

        fn predict_op<'inner, 'grammar, Symbol: Data + Default>(
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

        fn cross<'inner, 'grammar: 'inner, Symbol: Data + Default>(
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

        fn cross_ignorable<'inner, 'grammar: 'inner, Symbol: Data + Default>(
            src: &'inner RChartRow<'grammar, Symbol>,
            symbol: &Symbol,
            grammar: &'grammar Grammar<Symbol>,
        ) -> Vec<Item<'grammar, Symbol>> {
            let mut dest: Vec<Item<Symbol>> = Vec::new();

            for item in src.incomplete.items() {
                if item.next_symbol().unwrap() == symbol {
                    advance_past_symbol(item, &mut dest, grammar);
                } else {
                    dest.push(ignore_next_symbol(item, symbol));
                }
            }

            for item in src.complete.items() {
                dest.push(ignore_next_symbol(item, symbol));
            }

            dest
        }

        fn advance_past_symbol<'inner, 'grammar: 'inner, Symbol: Data + Default>(
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

        fn ignore_next_symbol<'inner, 'grammar: 'inner, Symbol: Data + Default>(
            item: &'inner Item<'grammar, Symbol>,
            symbol: &Symbol,
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
                spm: SymbolParseMethod::Ignored,
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

        fn mark_completed_item<'inner, 'grammar: 'inner, Symbol: Data + Default>(
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

        fn recognized<Symbol: Data + Default>(
            grammar: &Grammar<Symbol>,
            chart: &RChart<Symbol>,
        ) -> bool {
            chart
                .row(chart.len() - 1)
                .complete()
                .items()
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
            Err(parse::Error {
                message: format!(
                    "Recognition failed at token {}: {}",
                    cursor,
                    scan[cursor - 1].to_string()
                ),
            })
        };

        fn parse_tree<'scope, Symbol: Data + Default>(
            grammar: &'scope Grammar<Symbol>,
            scan: &'scope [Token<Symbol>],
            chart: PChart<'scope, Symbol>,
        ) -> Tree<Symbol> {
            let mut ordered_edges: Vec<&Edge<Symbol>> = Vec::new();
            for row in 0..chart.len() {
                for edge in chart.row(row).edges() {
                    ordered_edges.push(edge);
                }
            }

            ordered_edges.sort_unstable_by(|ref e1, ref e2| {
                // Order by increasing width, then decreasing depth
                match (e1.finish - e1.start).partial_cmp(&(e2.finish - e2.start)) {
                    Some(Ordering::Equal) => e2.depth.partial_cmp(&e1.depth),
                    x => x,
                }
                .unwrap()
            });

            let mut weight_map: HashMap<&Edge<Symbol>, usize> = HashMap::new();
            let mut nlp_map: HashMap<&Edge<Symbol>, ParsePath<Symbol>> = HashMap::new();

            for edge in &ordered_edges {
                let weighted_path = optimal_next_level_path(&edge, grammar, &chart, &weight_map);

                weight_map.insert(edge, weighted_path.weight);
                nlp_map.insert(edge, weighted_path.path);
            }

            let mut best_root_edge: &Edge<Symbol> = ordered_edges.last().unwrap();
            let mut best_root_weight = weight_map[best_root_edge];

            let finish: Node = chart.len() - 1;

            for edge in ordered_edges.iter().rev().skip(1) {
                if edge.finish != finish || edge.rule.unwrap().lhs != *grammar.start() {
                    break;
                }

                let minimal_tree_weight = weight_map[edge];
                if minimal_tree_weight < best_root_weight {
                    best_root_edge = edge;
                    best_root_weight = minimal_tree_weight;
                }
            }

            fn link_shallow_paths<'scope, Symbol: Data + Default>(
                edge: &Edge<Symbol>,
                grammar: &'scope Grammar<Symbol>,
                scan: &'scope [Token<Symbol>],
                nlp_map: &HashMap<&Edge<Symbol>, ParsePath<Symbol>>,
            ) -> Tree<Symbol> {
                match edge.rule {
                    None => Tree {
                        //Non-empty rhs
                        lhs: scan[edge.start].clone(),
                        children: Vec::new(),
                    },
                    Some(rule) => Tree {
                        lhs: Token::interior(rule.lhs.clone()),
                        children: {
                            let mut children: Vec<Tree<Symbol>> = nlp_map
                                .get(edge)
                                .unwrap()
                                .iter()
                                .filter(|&(_, ref edge)| edge.spm != SymbolParseMethod::Ignored)
                                .rev()
                                .map(|&(_, ref edge)| {
                                    link_shallow_paths(&edge, grammar, scan, nlp_map)
                                })
                                .collect();
                            if children.is_empty() {
                                //Empty rhs
                                children.push(Tree::null());
                            }
                            children
                        },
                    },
                }
            }

            fn optimal_next_level_path<'scope, Symbol: Data + Default>(
                edge: &Edge<Symbol>,
                grammar: &'scope Grammar<Symbol>,
                chart: &'scope PChart<'scope, Symbol>,
                weight_map: &HashMap<&'scope Edge<Symbol>, usize>,
            ) -> WeightedParsePath<'scope, Symbol> {
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
                                .edges()
                                .filter(|edge| edge.rule.unwrap().lhs == *symbol)
                                .cloned()
                                .collect();
                        }
                    }
                    Vec::new()
                };

                fn df_search<'scope, Symbol: Data + Default>(
                    edges: &Fn(usize, Node) -> Vec<Edge<'scope, Symbol>>,
                    leaf: &Fn(usize, Node) -> bool,
                    depth: usize,
                    root: Node,
                    weight_map: &HashMap<&'scope Edge<Symbol>, usize>,
                ) -> Option<WeightedParsePath<'scope, Symbol>> {
                    if leaf(depth, root) {
                        Some(WeightedParsePath::empty())
                    } else {
                        let mut best_path: Option<WeightedParsePath<'scope, Symbol>> = None;

                        for edge in edges(depth, root) {
                            if let Some(mut path) =
                                df_search(edges, leaf, depth + 1, edge.finish, weight_map)
                            {
                                let tree_weight = match weight_map.get(&edge) {
                                    Some(weight) => *weight,
                                    None => edge.weight,
                                };

                                if best_path.is_none()
                                    || path.weight + tree_weight
                                        < best_path.as_ref().unwrap().weight
                                {
                                    path.append(root, edge, tree_weight);
                                    best_path = Some(path);
                                }
                            }
                        }

                        best_path
                    }
                }

                match df_search(&edges, &leaf, 0, edge.start, weight_map) {
                    None => panic!("Failed to decompose parse edge of recognized scan"),
                    Some(mut path) => {
                        path.weight += edge.weight;
                        path
                    }
                }
            }

            link_shallow_paths(best_root_edge, grammar, scan, &nlp_map)
        }
    }
}

struct RChart<'item, Symbol: Data + Default + 'item> {
    rows: Vec<RChartRow<'item, Symbol>>,
}

impl<'item, Symbol: Data + Default + 'item> RChart<'item, Symbol> {
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
            for item in self.rows[i].incomplete().items() {
                println!("\t\t{}", item.to_string());
            }
            println!("\tCOMPLETE");
            for item in self.rows[i].complete().items() {
                println!("\t\t{}", item.to_string());
            }
        }
    }
}

struct RChartRow<'item, Symbol: Data + Default + 'item> {
    incomplete: Items<'item, Symbol>,
    complete: Items<'item, Symbol>,
}

impl<'item, Symbol: Data + Default + 'item> RChartRow<'item, Symbol> {
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

struct Items<'item, Symbol: Data + Default + 'item> {
    items: Vec<Item<'item, Symbol>>,
}

impl<'item, Symbol: Data + Default + 'item> Items<'item, Symbol> {
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

    fn items<'scope>(&'scope self) -> ItemsIterator<'scope, 'item, Symbol> {
        ItemsIterator {
            items: &self.items,
            index: 0,
        }
    }
}

struct ItemsIterator<'scope, 'item: 'scope, Symbol: Data + Default + 'item> {
    items: &'scope Vec<Item<'item, Symbol>>,
    index: usize,
}

impl<'scope, 'item: 'scope, Symbol: Data + Default + 'item> Iterator
    for ItemsIterator<'scope, 'item, Symbol>
{
    type Item = &'scope Item<'item, Symbol>;
    fn next(&mut self) -> Option<&'scope Item<'item, Symbol>> {
        self.index += 1;
        self.items.get(self.index - 1)
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
struct Item<'rule, Symbol: Data + Default + 'rule> {
    rule: &'rule Production<Symbol>,
    shadow: Option<Vec<ShadowSymbol<Symbol>>>,
    shadow_top: usize,
    start: usize,
    next: usize,
    depth: usize,
    ignore_next: bool,
}

impl<'rule, Symbol: Data + Default + 'rule> Item<'rule, Symbol> {
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

impl<'rule, Symbol: Data + Default> Data for Item<'rule, Symbol> {
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

struct PChart<'rule, Symbol: Data + Default + 'rule> {
    rows: Vec<PChartRow<'rule, Symbol>>,
}

impl<'rule, Symbol: Data + Default + 'rule> PChart<'rule, Symbol> {
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
            for edge in self.rows[i].edges() {
                println!("\t{}", edge.to_string());
            }
        }
    }
}

struct PChartRow<'rule, Symbol: Data + Default + 'rule> {
    edges: Vec<Edge<'rule, Symbol>>,
}

impl<'rule, Symbol: Data + Default + 'rule> PChartRow<'rule, Symbol> {
    fn new() -> Self {
        PChartRow { edges: Vec::new() }
    }

    fn add_edge(&mut self, edge: Edge<'rule, Symbol>) {
        self.edges.push(edge);
    }

    fn edge(&self, i: usize) -> Option<&Edge<Symbol>> {
        self.edges.get(i)
    }

    fn edges(&self) -> PChartRowIterator<Symbol> {
        PChartRowIterator {
            row: self,
            index: 0,
        }
    }
}

struct PChartRowIterator<'row, 'edge: 'row, Symbol: Data + Default + 'edge> {
    row: &'row PChartRow<'edge, Symbol>,
    index: usize,
}

impl<'row, 'edge: 'row, Symbol: Data + Default + 'edge> Iterator
    for PChartRowIterator<'row, 'edge, Symbol>
{
    type Item = &'row Edge<'row, Symbol>;
    fn next(&mut self) -> Option<&'edge Edge<'row, Symbol>> {
        self.index += 1;
        self.row.edge(self.index - 1)
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
struct Edge<'prod, Symbol: Data + Default + 'prod> {
    rule: Option<&'prod Production<Symbol>>,
    shadow: Option<Vec<ShadowSymbol<Symbol>>>,
    shadow_top: usize,
    start: usize,
    finish: usize,
    spm: SymbolParseMethod,
    weight: usize,
    depth: usize,
}

impl<'prod, Symbol: Data + Default + 'prod> Edge<'prod, Symbol> {
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
}

impl<'prod, Symbol: Data + Default + 'prod> Data for Edge<'prod, Symbol> {
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
struct ShadowSymbol<Symbol: Data + Default> {
    symbol: Symbol,
    spm: SymbolParseMethod,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
enum SymbolParseMethod {
    Standard,
    Ignored,
}

type Node = usize;
type ParsePath<'rule, Symbol> = Vec<(Node, Edge<'rule, Symbol>)>;

struct WeightedParsePath<'rule, Symbol: Data + Default> {
    path: ParsePath<'rule, Symbol>,
    weight: usize,
}

impl<'rule, Symbol: Data + Default> WeightedParsePath<'rule, Symbol> {
    fn empty() -> Self {
        WeightedParsePath {
            path: Vec::new(),
            weight: 0,
        }
    }

    fn append(&mut self, node: Node, edge: Edge<'rule, Symbol>, weight: usize) {
        self.weight += weight;
        self.path.push((node, edge));
    }
}
