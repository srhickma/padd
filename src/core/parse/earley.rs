use {
    core::{
        data::Data,
        parse::{
            self,
            grammar::Grammar,
            Parser,
            Production,
            Tree,
        },
        scan::Token,
    }
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

        grammar.productions_for_lhs(grammar.start()).unwrap().iter()
            .for_each(|prod| {
                chart.row_mut(0).unsafe_append(Item {
                    rule: prod,
                    start: 0,
                    next: 0,
                });
            });

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
                let item = chart.row(cursor).complete().item(i).clone();

                let accumulator = cross(
                    chart.row(item.start).incomplete().items(),
                    &item.rule.lhs,
                    grammar,
                );

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

            let mut i = 0;
            while i < chart.row(cursor).incomplete().len() {
                let item = chart.row(cursor).incomplete().item(i).clone();
                let symbol = (&item).next_symbol().unwrap();

                if !grammar.is_terminal(symbol) {
                    if grammar.is_nullable_nt(symbol) {
                        let new_item = Item {
                            rule: item.rule,
                            start: item.start,
                            next: item.next + 1,
                        };

                        if !chart.row(cursor).contains(&new_item) {
                            chart.row_mut(cursor).unsafe_append(new_item);
                        }
                    }

                    predict_op(cursor, symbol, grammar, chart);
                }
                i += 1;
            }
        }

        fn parse_mark_full<'inner, 'grammar, Symbol: Data + Default>(
            cursor: usize,
            chart: &'inner RChart<'grammar, Symbol>,
            parse_chart: &mut PChart<'grammar, Symbol>,
        ) {
            for item in chart.row(cursor).complete().items() {
                mark_completed_item(&item, cursor, parse_chart);
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

            let next_row = cross(
                chart.row(cursor).incomplete().items(),
                scan[cursor].kind(),
                grammar,
            );

            if next_row.is_empty() {
                return;
            }

            chart.add_row(next_row);
            parse_chart.add_row();
        }

        fn predict_op<'inner, 'grammar, Symbol: Data + Default>(
            cursor: usize,
            symbol: &'grammar Symbol,
            grammar: &'grammar Grammar<Symbol>,
            chart: &'inner mut RChart<'grammar, Symbol>,
        ) {
            let mut items_to_add = Vec::new();

            for prod in grammar.productions_for_lhs(symbol).unwrap() {
                let new_item = Item {
                    rule: prod,
                    start: cursor,
                    next: 0,
                };

                if !chart.row(cursor).contains(&new_item) {
                    items_to_add.push(new_item);
                }
            }

            for new_item in items_to_add {
                chart.row_mut(cursor).unsafe_append(new_item);
            }
        }

        fn cross<'inner, 'grammar: 'inner, Symbol: Data + Default>(
            src: impl Iterator<Item=&'inner Item<'grammar, Symbol>>,
            symbol: &Symbol,
            grammar: &'grammar Grammar<Symbol>,
        ) -> Vec<Item<'grammar, Symbol>> {
            let mut dest: Vec<Item<Symbol>> = Vec::new();

            for item in src {
                if let Some(sym) = item.next_symbol() {
                    if sym == symbol {
                        let mut last_item = item.clone();

                        loop {
                            last_item = Item {
                                rule: last_item.rule,
                                start: last_item.start,
                                next: last_item.next + 1,
                            };

                            dest.push(last_item.clone());

                            match last_item.next_symbol() {
                                None => break,
                                Some(sym) => if !grammar.is_nullable_nt(sym) {
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            dest
        }

        fn mark_completed_item<'inner, 'grammar: 'inner, Symbol: Data + Default>(
            item: &'inner Item<'grammar, Symbol>,
            finish: usize,
            parse_chart: &mut PChart<'grammar, Symbol>,
        ) {
            parse_chart.row_mut(item.start).add_edge(Edge {
                rule: Some(item.rule),
                finish,
            });
        }

        fn recognized<Symbol: Data + Default>(grammar: &Grammar<Symbol>, chart: &RChart<Symbol>) -> bool {
            chart.row(chart.len() - 1).complete().items()
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
            fn recur<'scope, Symbol: Data + Default>(
                start: Node,
                edge: &Edge<Symbol>,
                grammar: &'scope Grammar<Symbol>,
                scan: &'scope [Token<Symbol>],
                chart: &PChart<Symbol>,
            ) -> Tree<Symbol> {
                match edge.rule {
                    None => Tree { //Non-empty rhs
                        lhs: scan[start].clone(),
                        children: Vec::new(),
                    },
                    Some(rule) => Tree {
                        lhs: Token::interior(rule.lhs.clone()),
                        children: {
                            let mut children: Vec<Tree<Symbol>> =
                                top_list(start, edge, grammar, scan, chart).iter().rev()
                                    .map(|&(node, ref edge)| recur(
                                        node,
                                        &edge,
                                        grammar,
                                        scan,
                                        chart,
                                    ))
                                    .collect();
                            if children.is_empty() { //Empty rhs
                                children.push(Tree::null());
                            }
                            children
                        },
                    }
                }
            }

            let finish: Node = chart.len() - 1;

            let first_edge = chart.row(0).edges()
                .find(|edge| edge.finish == finish && edge.rule.unwrap().lhs == *grammar.start());
            match first_edge {
                None => panic!("Failed to find start item to begin parse"),
                Some(edge) => recur(0, edge, grammar, scan, &chart)
            }
        }

        fn top_list<'scope, Symbol: Data + Default>(
            start: Node,
            edge: &Edge<Symbol>,
            grammar: &'scope Grammar<Symbol>,
            scan: &'scope [Token<Symbol>],
            chart: &'scope PChart<'scope, Symbol>,
        ) -> Vec<(Node, Edge<'scope, Symbol>)> {
            let symbols: &Vec<Symbol> = &edge.rule.unwrap().rhs;
            let bottom: usize = symbols.len();
            let leaf = |depth: usize, node: Node| depth == bottom && node == edge.finish;
            let edges = |depth: usize, node: Node| -> Vec<Edge<Symbol>> {
                if depth < bottom {
                    let symbol = &symbols[depth];
                    if grammar.is_terminal(symbol) {
                        if *scan[node].kind() == *symbol {
                            return vec![Edge {
                                rule: None,
                                finish: node + 1,
                            }];
                        }
                    } else {
                        return chart.row(node).edges()
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
            ) -> Option<Vec<(Node, Edge<'scope, Symbol>)>> {
                if leaf(depth, root) {
                    Some(Vec::new())
                } else {
                    for edge in edges(depth, root) {
                        if let Some(mut path) = df_search(edges, leaf, depth + 1, edge.finish) {
                            path.push((root, edge));
                            return Some(path);
                        }
                    }
                    None
                }
            }

            match df_search(&edges, &leaf, 0, start) {
                None => panic!("Failed to decompose parse edge of recognized scan"),
                Some(path) => path
            }
        }
    }
}

struct RChart<'item, Symbol: Data + Default + 'item> {
    rows: Vec<RChartRow<'item, Symbol>>
}

impl<'item, Symbol: Data + Default + 'item> RChart<'item, Symbol> {
    fn new() -> Self {
        RChart {
            rows: vec![RChartRow::new(Vec::new())]
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
    items: Vec<Item<'item, Symbol>>
}

impl<'item, Symbol: Data + Default + 'item> Items<'item, Symbol> {
    fn new() -> Items<'item, Symbol> {
        Items {
            items: Vec::new()
        }
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

impl<'scope, 'item: 'scope, Symbol: Data + Default + 'item> Iterator for ItemsIterator<'scope, 'item, Symbol> {
    type Item = &'scope Item<'item, Symbol>;
    fn next(&mut self) -> Option<&'scope Item<'item, Symbol>> {
        self.index += 1;
        self.items.get(self.index - 1)
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
struct Item<'rule, Symbol: Data + Default + 'rule> {
    rule: &'rule Production<Symbol>,
    start: usize,
    next: usize,
}

impl<'rule, Symbol: Data + Default + 'rule> Item<'rule, Symbol> {
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
        format!("{} ({:?})", rule_string, self.start)
    }
}

struct PChart<'edge, Symbol: Data + Default + 'edge> {
    rows: Vec<PChartRow<'edge, Symbol>>
}

impl<'edge, Symbol: Data + Default + 'edge> PChart<'edge, Symbol> {
    fn new() -> Self {
        PChart {
            rows: vec![PChartRow::new()]
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

    fn row_mut(&mut self, i: usize) -> &mut PChartRow<'edge, Symbol> {
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

struct PChartRow<'edge, Symbol: Data + Default + 'edge> {
    edges: Vec<Edge<'edge, Symbol>>
}

impl<'edge, Symbol: Data + Default + 'edge> PChartRow<'edge, Symbol> {
    fn new() -> Self {
        PChartRow {
            edges: Vec::new()
        }
    }

    fn add_edge(&mut self, edge: Edge<'edge, Symbol>) {
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

impl<'row, 'edge: 'row, Symbol: Data + Default + 'edge> Iterator for PChartRowIterator<'row, 'edge, Symbol> {
    type Item = &'row Edge<'row, Symbol>;
    fn next(&mut self) -> Option<&'edge Edge<'row, Symbol>> {
        self.index += 1;
        self.row.edge(self.index - 1)
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
struct Edge<'prod, Symbol: Data + Default + 'prod> {
    rule: Option<&'prod Production<Symbol>>,
    finish: usize,
}

impl<'prod, Symbol: Data + Default + 'prod> Data for Edge<'prod, Symbol> {
    fn to_string(&self) -> String {
        match self.rule {
            None => format!("NONE ({})", self.finish),
            Some(rule) => {
                let mut rule_string = format!("{:?} -> ", rule.lhs);
                for i in 0..rule.rhs.len() {
                    rule_string = format!("{}{:?} ", rule_string, rule.rhs[i]);
                }
                format!("{} ({})", rule_string, self.finish)
            }
        }
    }
}

type Node = usize;
