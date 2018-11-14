extern crate stopwatch;

use core::data::Data;
use core::parse;
use core::parse::grammar::Grammar;
use core::parse::Parser;
use core::parse::Production;
use core::parse::Tree;
use core::scan::Token;

use self::stopwatch::Stopwatch;

pub struct LeoParser;

impl Parser for LeoParser {
    fn parse(&self, scan: Vec<Token<String>>, grammar: &Grammar) -> Result<Tree, parse::Error> {
        let sw = Stopwatch::start_new();

        let mut chart: Vec<Row> = vec![Row::new()];

        //TODO ensure topmost item by creating S-prime start production

        grammar.productions.iter()
            .filter(|prod| prod.lhs == grammar.start)
            .for_each(|prod| {
                let item = Item {
                    rule: prod,
                    start: 0,
                    next: 0,
                };
                chart[0].items.push(item);
            });

        println!("Creating recognition chart took {}", sw.elapsed_ms());

        let mut k = 0;
        while k < chart[0].items.len() {
            let item = chart[0].items[k].clone();
            match (&item).next_symbol() {
                None => {}
                Some(symbol) => {
                    if !grammar.terminals.contains(symbol) {
                        predict_op(&item, 0, symbol, grammar, &mut chart);
                    }
                }
            }

            k += 1;
        }

        let mut i = 1;
        while i <= chart.len() && i <= scan.len() {

            // Scanning
            let mut next_row = Row::new();
            let a_i = &scan[i - 1].kind[..];
            cross(&chart[i - 1].items, a_i, &mut next_row.items, grammar);
            if next_row.items.is_empty() {
                break;
            }
            chart.push(next_row);

            // Completion
            let mut j = 0;
            while j < chart[i].items.len() {
                let item = chart[i].items[j].clone();
                let next = (&item).next_symbol();
                match next {
                    None => {
                        let mut accumulator: Vec<Item> = Vec::new();
                        cross(&chart[item.start].items, &item.rule.lhs[..], &mut accumulator, grammar);

                        chart[i].items.append(&mut accumulator);
                    }
                    Some(symbol) => {}
                }
                j += 1;
            }

            // Prediction
            let mut k = 0;
            while k < chart[i].items.len() {
                let item = chart[i].items[k].clone();
                match (&item).next_symbol() {
                    None => {}
                    Some(symbol) => {
                        if !grammar.terminals.contains(symbol) {
                            predict_op(&item, i, symbol, grammar, &mut chart);
                        }
                    }
                }

                k += 1;
            }

            i += 1;
        }

        println!("Filling recognition chart took {} for {} tokens", sw.elapsed_ms(), scan.len());

        fn t_update<'a, 'b>(
            item: &Item<'a>,
            i: usize,
            grammar: &'a Grammar,
            chart: &'b mut Vec<Row<'a>>,
        ) {}

        fn cross<'a>(
            src: &Vec<Item<'a>>,
            symbol: &'a str,
            dest: &mut Vec<Item<'a>>,
            grammar: &'a Grammar,
        ) {
            for item in src {
                let next = item.next_symbol();
                match next {
                    None => {}
                    Some(sym) => {
                        if sym == symbol {
                            let mut last_item = Item {
                                rule: item.rule,
                                start: item.start,
                                next: item.next + 1,
                            };

                            unsafe_append(last_item.clone(), dest);

                            loop {
                                match last_item.next_symbol() {
                                    None => break,
                                    Some(sym) => {
                                        let sym_string = sym.to_string();
                                        if !grammar.nullable_nt(&sym_string) {
                                            break;
                                        }
                                        last_item = Item {
                                            rule: last_item.rule,
                                            start: last_item.start,
                                            next: last_item.next + 1,
                                        };

                                        unsafe_append(last_item.clone(), dest);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        fn predict_op<'a, 'b>(item: &Item<'a>, i: usize, symbol: &'a str, grammar: &'a Grammar, chart: &'b mut Vec<Row<'a>>) {
            grammar.productions.iter()
                .filter(|prod| prod.lhs == symbol)
                .for_each(|prod| {
                    append(
                        Item {
                            rule: prod,
                            start: i,
                            next: 0,
                        },
                        &mut chart[i].items,
                    );

                    if grammar.nullable(&prod) {
                        append(
                            Item {
                                rule: item.rule,
                                start: item.start,
                                next: item.next + 1,
                            },
                            &mut chart[i].items,
                        );
                    }
                });
        }

        fn complete_op<'a, 'b>(item: &Item<'a>, i: usize, chart: &'b mut Vec<Row<'a>>) {
            let mut advanced: Vec<Item> = vec![];

            chart[item.start].items.iter()
                .filter(|old_item| match old_item.next_symbol() {
                    None => false,
                    Some(sym) => sym == item.rule.lhs,
                })
                .for_each(|old_item| advanced.push(Item {
                    rule: old_item.rule,
                    start: old_item.start,
                    next: old_item.next + 1,
                }));

            for item in advanced {
                append(item, &mut chart[i].items);
            }
        }

        fn append<'a, 'b>(item: Item<'a>, item_set: &'b mut Vec<Item<'a>>) {
            for j in 0..item_set.len() {
                if item_set[j] == item {
                    return;
                }
            }
            unsafe_append(item, item_set);
        }

        fn unsafe_append<'a, 'b>(item: Item<'a>, item_set: &'b mut Vec<Item<'a>>) {
            item_set.push(item);
        }

        fn recognized<'a, 'b>(grammar: &'a Grammar, chart: &'b Vec<Row<'a>>) -> bool {
            chart.last().unwrap().items.iter()
                .any(|item| item.rule.lhs == grammar.start
                    && item.next >= item.rule.rhs.len()
                    && item.start == 0)
        }

        println!("-----------------------------------------------------");
        for i in 0..chart.len() {
            println!("SET {}", i);
            for j in 0..chart[i].items.len() {
                println!("{}", chart[i].items[j].to_string());
            }
            println!();
        }
        println!("-----------------------------------------------------");

        return if recognized(grammar, &chart) {
            if i - 1 == scan.len() {
                Ok(parse_tree(grammar, &scan, chart))
            } else {
                Err(parse::Error {
                    message: format!("Largest parse did not consume all tokens: {} of {}", i - 1, scan.len()),
                })
            }
        } else {
            if scan.len() == 0 {
                Err(parse::Error {
                    message: "No tokens scanned".to_string(),
                })
            } else if i - 1 == scan.len() {
                Err(parse::Error {
                    message: format!("Recognition failed after consuming all tokens"),
                })
            } else {
                Err(parse::Error {
                    message: format!("Recognition failed at token {}: {}", i, scan[i - 1].to_string()),
                })
            }
        };

        //TODO refactor to reduce long and duplicated parameter lists
        fn parse_tree<'a>(grammar: &'a Grammar, scan: &'a Vec<Token<String>>, chart: Vec<Row<'a>>) -> Tree {
            fn recur<'a>(start: Node, edge: &Edge, grammar: &'a Grammar, scan: &'a Vec<Token<String>>, chart: &Vec<Vec<Edge>>) -> Tree {
                match edge.rule {
                    None => Tree { //Non-empty rhs
                        lhs: scan[start].clone(),
                        children: vec![],
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
                            if children.is_empty() { //empty rhs
                                children.push(Tree::null());
                            }
                            children
                        },
                    }
                }
            }

            let start: Node = 0;
            let finish: Node = chart.len() - 1;

            //TODO build the parse chart during the main loop
            let mut parse_chart: Vec<Vec<Edge>> = Vec::with_capacity(chart.len());
            for _ in 0..chart.len() {
                parse_chart.push(vec![]);
            }
            for i in 0..chart.len() {
                for item in &chart[i].items {
                    if item.next_symbol().is_none() {
                        parse_chart[item.start].push(Edge {
                            rule: Some(item.rule),
                            finish: i,
                        })
                    }
                }
            }

            let first_edge = parse_chart[start].iter()
                .find(|edge| edge.finish == finish && edge.rule.unwrap().lhs == grammar.start);
            match first_edge {
                None => panic!("Failed to find start item to begin parse"),
                Some(edge) => recur(start, edge, grammar, scan, &parse_chart)
            }
        }


        fn top_list<'a>(start: Node,
                        edge: &Edge,
                        grammar: &'a Grammar,
                        scan: &'a Vec<Token<String>>,
                        chart: &Vec<Vec<Edge<'a>>>) -> Vec<(Node, Edge<'a>)> {
            let symbols: &Vec<String> = &edge.rule.unwrap().rhs;
            let bottom: usize = symbols.len();
            let leaf = |depth: usize, node: Node| depth == bottom && node == edge.finish;
            let edges = |depth: usize, node: Node| -> Vec<Edge> {
                if depth < bottom {
                    let symbol = &symbols[depth];
                    if grammar.terminals.contains(symbol) {
                        if scan[node].kind == *symbol {
                            return vec![Edge {
                                rule: None,
                                finish: node + 1,
                            }];
                        }
                    } else { //TODO return iterators instead to avoid collection and cloning
                        return chart[node].iter()
                            .filter(|edge| edge.rule.unwrap().lhs == *symbol)
                            .cloned()
                            .collect();
                    }
                }
                vec![]
            };

            fn df_search<'a>(edges: &Fn(usize, Node) -> Vec<Edge<'a>>,
                             leaf: &Fn(usize, Node) -> bool,
                             depth: usize,
                             root: Node) -> Option<Vec<(Node, Edge<'a>)>> {
                if leaf(depth, root) {
                    Some(vec![])
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

struct Row<'a> {
    items: Vec<Item<'a>>,
    transitive_items: Vec<TransitiveItem<'a>>,
}

impl<'a> Row<'a> {
    fn new() -> Row<'a> {
        Row {
            items: Vec::new(),
            transitive_items: Vec::new(),
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
struct TransitiveItem<'a> {
    rule: &'a Production,
    start: usize,
    next: usize,
    captor: String, // Non-Terminal
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
}

impl<'a> Data for Item<'a> {
    #[cfg_attr(tarpaulin, skip)]
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

#[derive(Clone, PartialEq, Debug)]
struct Edge<'a> {
    rule: Option<&'a Production>,
    finish: usize,
}

impl<'a> Data for Edge<'a> {
    #[cfg_attr(tarpaulin, skip)]
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
