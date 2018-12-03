use core::data::Data;
use core::parse;
use core::parse::grammar::Grammar;
use core::parse::Parser;
use core::parse::Production;
use core::parse::Tree;
use core::scan::Token;

pub struct EarleyParser;

impl Parser for EarleyParser {
    fn parse(&self, scan: Vec<Token<String>>, grammar: &Grammar) -> Result<Tree, parse::Error> {
        let mut chart: RChart = RChart::new();
        let mut parse_chart: PChart = PChart::new();

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

        fn complete_full<'inner, 'grammar: 'inner>(
            cursor: usize,
            grammar: &'grammar Grammar,
            chart: &'inner mut RChart<'grammar>,
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

        fn predict_full<'inner, 'grammar: 'inner>(
            grammar: &'grammar Grammar,
            chart: &'inner mut RChart<'grammar>,
        ) {
            let cursor = chart.len() - 1;

            let mut i = 0;
            while i < chart.row(cursor).incomplete().len() {
                let item = chart.row(cursor).incomplete().item(i).clone();
                let next = (&item).next_symbol();
                match next {
                    None => {}
                    Some(symbol) => {
                        if !grammar.is_terminal(symbol) {
                            predict_op(&item, cursor, symbol, grammar, chart);
                        }
                    }
                }
                i += 1;
            }
        }

        fn parse_mark_full<'inner, 'grammar>(
            cursor: usize,
            chart: &'inner RChart<'grammar>,
            parse_chart: &mut PChart<'grammar>,
        ) {
            for item in chart.row(cursor).complete().items() {
                mark_completed_item(&item, cursor, parse_chart);
            }
        }

        fn scan_full<'inner, 'grammar: 'inner>(
            cursor: usize,
            scan: &Vec<Token<String>>,
            grammar: &'grammar Grammar,
            chart: &'inner mut RChart<'grammar>,
            parse_chart: &mut PChart<'grammar>,
        ) {
            if cursor == scan.len() {
                return;
            }

            let next_row = cross(
                chart.row(cursor).incomplete().items(),
                &scan[cursor].kind,
                grammar,
            );

            if next_row.is_empty() {
                return;
            }

            chart.add_row(next_row);
            parse_chart.add_row();
        }

        fn predict_op<'inner, 'grammar>(
            item: &Item<'grammar>,
            i: usize,
            symbol: &'grammar str,
            grammar: &'grammar Grammar,
            chart: &'inner mut RChart<'grammar>,
        ) {
            let mut nullable_found = false;
            let mut items_to_add = Vec::new();

            for prod in grammar.productions_for_lhs(symbol).unwrap() {
                let new_item = Item {
                    rule: prod,
                    start: i,
                    next: 0,
                };

                if !chart.row(i).contains(&new_item) {
                    items_to_add.push(new_item);
                }

                if !nullable_found && grammar.is_nullable(&prod) {
                    nullable_found = true;
                }
            }

            for new_item in items_to_add {
                chart.row_mut(i).unsafe_append(new_item);
            }

            if nullable_found {
                let new_item = Item {
                    rule: item.rule,
                    start: item.start,
                    next: item.next + 1,
                };

                if !chart.row(i).contains(&new_item) {
                    chart.row_mut(i).unsafe_append(new_item);
                }
            }
        }

        fn cross<'inner, 'grammar: 'inner>(
            src: impl Iterator<Item=&'inner Item<'grammar>>,
            symbol: &String,
            grammar: &'grammar Grammar,
        ) -> Vec<Item<'grammar>> {
            let mut dest: Vec<Item> = Vec::new();

            for item in src {
                match item.next_symbol() {
                    None => {}
                    Some(sym) => {
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
                                    Some(sym) => {
                                        let sym_string = sym.to_string();
                                        if !grammar.is_nullable_nt(&sym_string) {
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            dest
        }

        fn mark_completed_item<'inner, 'grammar: 'inner>(
            item: &'inner Item<'grammar>,
            finish: usize,
            parse_chart: &mut PChart<'grammar>,
        ) {
            parse_chart.row_mut(item.start).add_edge(Edge {
                rule: Some(item.rule),
                finish,
            });
        }

        fn recognized(grammar: &Grammar, chart: &RChart) -> bool {
            chart.row(chart.len() - 1).complete().items()
                .any(|item| item.rule.lhs == *grammar.start() && item.start == 0)
        }

//        println!("-----------------------------------------------------");
//        for i in 0..chart.len() {
//            println!("SET {}", i);
//            for j in 0..chart[i].len() {
//                println!("{}", chart[i][j].to_string());
//            }
//            println!();
//        }
//        println!("-----------------------------------------------------");

        return if recognized(grammar, &chart) {
            if cursor - 1 == scan.len() {
                Ok(parse_tree(grammar, &scan, parse_chart))
            } else {
                Err(parse::Error {
                    message: format!("Largest parse did not consume all tokens: {} of {}", cursor - 1, scan.len()),
                })
            }
        } else {
            if scan.len() == 0 {
                Err(parse::Error {
                    message: "No tokens scanned".to_string(),
                })
            } else if cursor - 1 == scan.len() {
                Err(parse::Error {
                    message: format!("Recognition failed after consuming all tokens"),
                })
            } else {
                Err(parse::Error {
                    message: format!("Recognition failed at token {}: {}", cursor, scan[cursor - 1].to_string()),
                })
            }
        };

        //TODO refactor to reduce long and duplicated parameter lists
        fn parse_tree<'a>(
            grammar: &'a Grammar,
            scan: &'a Vec<Token<String>>,
            chart: PChart<'a>,
        ) -> Tree {
            fn recur<'a>(
                start: Node,
                edge: &Edge,
                grammar: &'a Grammar,
                scan: &'a Vec<Token<String>>,
                chart: &PChart,
            ) -> Tree {
                match edge.rule {
                    None => Tree { //Non-empty rhs
                        lhs: scan[start].clone(),
                        children: Vec::new(),
                    },
                    Some(rule) => Tree {
                        lhs: Token {
                            kind: rule.lhs.clone(),
                            lexeme: String::new(),
                        },
                        children: {
                            let mut children: Vec<Tree> =
                                top_list(start, edge, grammar, scan, chart).iter().rev()
                                    .map(|&(node, ref edge)| recur(node, &edge, grammar, scan, chart))
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

            let first_edge = chart.row(0).iter()
                .find(|edge| edge.finish == finish && edge.rule.unwrap().lhs == *grammar.start());
            match first_edge {
                None => panic!("Failed to find start item to begin parse"),
                Some(edge) => recur(0, edge, grammar, scan, &chart)
            }
        }

        fn top_list<'a>(
            start: Node,
            edge: &Edge,
            grammar: &'a Grammar,
            scan: &'a Vec<Token<String>>,
            chart: &'a PChart<'a>,
        ) -> Vec<(Node, Edge<'a>)> {
            let symbols: &Vec<String> = &edge.rule.unwrap().rhs;
            let bottom: usize = symbols.len();
            let leaf = |depth: usize, node: Node| depth == bottom && node == edge.finish;
            let edges = |depth: usize, node: Node| -> Vec<Edge> {
                if depth < bottom {
                    let symbol = &symbols[depth];
                    if grammar.is_terminal(symbol) {
                        if scan[node].kind == *symbol {
                            return vec![Edge {
                                rule: None,
                                finish: node + 1,
                            }];
                        }
                    } else { //TODO return iterators instead to avoid collection and cloning
                        return chart.row(node).iter()
                            .filter(|edge| edge.rule.unwrap().lhs == *symbol)
                            .cloned()
                            .collect();
                    }
                }
                Vec::new()
            };

            fn df_search<'a>(
                edges: &Fn(usize, Node) -> Vec<Edge<'a>>,
                leaf: &Fn(usize, Node) -> bool,
                depth: usize,
                root: Node,
            ) -> Option<Vec<(Node, Edge<'a>)>> {
                if leaf(depth, root) {
                    Some(Vec::new())
                } else {
                    for edge in edges(depth, root) {
                        match df_search(edges, leaf, depth + 1, edge.finish) {
                            None => {}
                            Some(mut path) => {
                                path.push((root, edge));
                                return Some(path);
                            }
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

struct RChart<'item> {
    rows: Vec<RChartRow<'item>>
}

impl<'item> RChart<'item> {
    fn new() -> Self {
        RChart {
            rows: vec![RChartRow::new(Vec::new())]
        }
    }

    fn len(&self) -> usize {
        self.rows.len()
    }

    fn add_row(&mut self, items: Vec<Item<'item>>) {
        self.rows.push(RChartRow::new(items));
    }

    fn row(&self, i: usize) -> &RChartRow<'item> {
        self.rows.get(i).unwrap()
    }

    fn row_mut(&mut self, i: usize) -> &mut RChartRow<'item> {
        self.rows.get_mut(i).unwrap()
    }
}

struct RChartRow<'item> {
    incomplete: Items<'item>,
    complete: Items<'item>,
}

impl<'item> RChartRow<'item> {
    fn new(scanned_items: Vec<Item>) -> RChartRow {
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

    fn unsafe_append(&mut self, item: Item<'item>) {
        if item.is_complete() {
            self.complete.unsafe_append(item);
        } else {
            self.incomplete.unsafe_append(item);
        }
    }

    fn contains(&self, item: &Item<'item>) -> bool {
        if item.is_complete() {
            self.complete.contains(item)
        } else {
            self.incomplete.contains(item)
        }
    }

    fn incomplete(&self) -> &Items<'item> {
        &self.incomplete
    }

    fn incomplete_mut(&mut self) -> &mut Items<'item> {
        &mut self.incomplete
    }

    fn complete(&self) -> &Items<'item> {
        &self.complete
    }

    fn complete_mut(&mut self) -> &mut Items<'item> {
        &mut self.complete
    }
}

struct Items<'item> {
    items: Vec<Item<'item>>
}

impl<'item> Items<'item> {
    fn new() -> Items<'item> {
        Items {
            items: Vec::new()
        }
    }

    fn len(&self) -> usize {
        self.items.len()
    }

    fn unsafe_append(&mut self, item: Item<'item>) {
        self.items.push(item);
    }

    fn item(&self, i: usize) -> &Item<'item> {
        self.items.get(i).unwrap()
    }

    fn contains(&self, item: &Item<'item>) -> bool {
        self.items.contains(item)
    }

    fn items<'scope>(&'scope self) -> ItemsIterator<'scope, 'item> {
        ItemsIterator {
            items: &self.items,
            index: 0,
        }
    }
}

struct ItemsIterator<'scope, 'item: 'scope> {
    items: &'scope Vec<Item<'item>>,
    index: usize,
}

impl<'scope, 'item: 'scope> Iterator for ItemsIterator<'scope, 'item> {
    type Item = &'scope Item<'item>;
    fn next(&mut self) -> Option<&'scope Item<'item>> {
        self.index += 1;
        self.items.get(self.index - 1)
    }
}

#[derive(PartialEq, Clone, Debug)]
struct Item<'a> {
    rule: &'a Production,
    start: usize,
    next: usize,
}

impl<'a> Item<'a> {
    fn next_symbol<'b>(&'b self) -> Option<&'a str> {
        if self.next < self.rule.rhs.len() {
            Some(&self.rule.rhs[self.next][..])
        } else {
            None
        }
    }

    fn is_complete(&self) -> bool {
        self.next >= self.rule.rhs.len()
    }
}

impl<'a> Data for Item<'a> {
    fn to_string(&self) -> String {
        let mut rule_string = format!("{} -> ", self.rule.lhs);
        for i in 0..self.rule.rhs.len() {
            if i == self.next {
                rule_string.push_str(". ");
            }
            rule_string.push_str(self.rule.rhs.get(i).unwrap());
            rule_string.push(' ');
        }
        if self.next == self.rule.rhs.len() {
            rule_string.push_str(". ");
        }
        format!("{} ({})", rule_string, self.start)
    }
}

struct PChart<'edge> {
    rows: Vec<PChartRow<'edge>>
}

impl<'edge> PChart<'edge> {
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

    fn row(&self, i: usize) -> &PChartRow {
        self.rows.get(i).unwrap()
    }

    fn row_mut(&mut self, i: usize) -> &mut PChartRow<'edge> {
        self.rows.get_mut(i).unwrap()
    }
}

struct PChartRow<'edge> {
    edges: Vec<Edge<'edge>>
}

impl<'edge> PChartRow<'edge> {
    fn new() -> Self {
        PChartRow {
            edges: Vec::new()
        }
    }

    fn add_edge(&mut self, edge: Edge<'edge>) {
        self.edges.push(edge);
    }

    fn edge(&self, i: usize) -> Option<&Edge> {
        self.edges.get(i)
    }

    fn iter(&self) -> PChartRowIterator {
        PChartRowIterator {
            row: self,
            index: 0,
        }
    }
}

struct PChartRowIterator<'row, 'edge: 'row> {
    row: &'row PChartRow<'edge>,
    index: usize,
}

impl<'row, 'edge: 'row> Iterator for PChartRowIterator<'row, 'edge> {
    type Item = &'row Edge<'row>;
    fn next(&mut self) -> Option<&'edge Edge<'row>> {
        self.index += 1;
        self.row.edge(self.index - 1)
    }
}

#[derive(Clone, PartialEq, Debug)]
struct Edge<'a> {
    rule: Option<&'a Production>,
    finish: usize,
}

impl<'a> Data for Edge<'a> {
    fn to_string(&self) -> String {
        match self.rule {
            None => format!("NONE ({})", self.finish),
            Some(rule) => {
                let mut rule_string = format!("{} -> ", rule.lhs);
                for i in 0..rule.rhs.len() {
                    rule_string.push_str(rule.rhs.get(i).unwrap());
                    rule_string.push(' ');
                }
                format!("{} ({})", rule_string, self.finish)
            }
        }
    }
}

type Node = usize;
