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
        let mut parse_chart: Vec<Vec<Edge>> = vec![Vec::new()];
        let mut chart: Vec<Vec<Item>> = vec![Vec::new()];

        grammar.productions_for_lhs(grammar.start()).unwrap().iter()
            .for_each(|prod| {
                let item = Item {
                    rule: prod,
                    start: 0,
                    next: 0,
                };
                chart[0].push(item);
            });

        let mut i = 0;
        while i <= chart.len() {
            let mut j = 0;
            while j < chart[i].len() {
                if chart[i][j].is_complete() {
                    let item = chart[i][j].clone();
                    let accumulator = cross(&chart[item.start], &item.rule.lhs[..], grammar);

                    let mut items_to_add = Vec::new();
                    for completed_item in accumulator {
                        if !chart[i].contains(&completed_item) {
                            items_to_add.push(completed_item);
                        }
                    }

                    for new_item in items_to_add {
                        unsafe_append(new_item, &mut chart[i]);
                    }
                }
                j += 1;
            }

            predict_full(grammar, &mut chart);

            for item in &chart[i] {
                if item.is_complete() {
                    mark_completed_item(&item, i, &mut parse_chart);
                }
            }

            if i >= scan.len() {
                break;
            }

            let a_i = &scan[i].kind[..];
            let next_row = cross(&chart[i], a_i, grammar);
            if next_row.is_empty() {
                break;
            }
            chart.push(next_row);
            parse_chart.push(Vec::new());

            i += 1;
        }

        fn mark_completed_item<'a, 'b: 'a>(item: &'a Item<'b>, finish: usize, parse_chart: &mut Vec<Vec<Edge<'b>>>) {
//            println!("{} in set {}", item.to_string(), finish);
//            println!("ADDING EDGE: {} to {}", Edge {
//                rule: Some(item.rule),
//                finish,
//            }.to_string(), item.start);

            parse_chart[item.start].push(Edge {
                rule: Some(item.rule),
                finish,
            });
        }

        fn predict_full<'a, 'b>(
            grammar: &'a Grammar,
            chart: &'b mut Vec<Vec<Item<'a>>>,
        ) {
            let row = chart.len() - 1;

            let mut i = 0;
            while i < chart[row].len() {
                let item = chart[row][i].clone(); //TODO see if we can reduce cloning here
                let next = (&item).next_symbol();
                match next {
                    None => {}
                    Some(symbol) => {
                        if !grammar.is_terminal(symbol) {
                            predict_op(&item, row, symbol, grammar, chart);
                        }
                    }
                }
                i += 1;
            }
        }

        fn predict_op<'a, 'b>(
            item: &Item<'a>,
            i: usize,
            symbol: &'a str,
            grammar: &'a Grammar,
            chart: &'b mut Vec<Vec<Item<'a>>>,
        ) {
            let mut nullable_found = false;
            let mut items_to_add = Vec::new();

            for prod in grammar.productions_for_lhs(symbol).unwrap() {
                let new_item = Item {
                    rule: prod,
                    start: i,
                    next: 0,
                };

                if !chart[i].contains(&new_item) {
                    items_to_add.push(new_item);
                }

                if !nullable_found && grammar.is_nullable(&prod) {
                    nullable_found = true;
                }
            }

            for new_item in items_to_add {
                unsafe_append(new_item, &mut chart[i]);
            }

            if nullable_found {
                append(Item {
                    rule: item.rule,
                    start: item.start,
                    next: item.next + 1,
                }, &mut chart[i], );
            }
        }

        fn cross<'a>(
            src: &Vec<Item<'a>>,
            symbol: &'a str,
            grammar: &'a Grammar,
        ) -> Vec<Item<'a>> {
            let mut dest: Vec<Item> = Vec::new();

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

                            unsafe_append(last_item.clone(), &mut dest);

                            loop {
                                match last_item.next_symbol() {
                                    None => break,
                                    Some(sym) => {
                                        let sym_string = sym.to_string();
                                        if !grammar.is_nullable_nt(&sym_string) {
                                            break;
                                        }
                                        last_item = Item {
                                            rule: last_item.rule,
                                            start: last_item.start,
                                            next: last_item.next + 1,
                                        };

                                        unsafe_append(last_item.clone(), &mut dest);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            dest
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

        fn recognized<'a, 'b>(grammar: &'a Grammar, chart: &'b Vec<Vec<Item<'a>>>) -> bool {
            chart.last().unwrap().iter()
                .any(|item| item.rule.lhs == *grammar.start()
                    && item.next >= item.rule.rhs.len()
                    && item.start == 0)
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
            if i == scan.len() {
                Ok(parse_tree(grammar, &scan, parse_chart))
            } else {
                Err(parse::Error {
                    message: format!("Largest parse did not consume all tokens: {} of {}", i, scan.len()),
                })
            }
        } else {
            if scan.len() == 0 {
                Err(parse::Error {
                    message: "No tokens scanned".to_string(),
                })
            } else if i == scan.len() {
                Err(parse::Error {
                    message: format!("Recognition failed after consuming all tokens"),
                })
            } else {
                Err(parse::Error {
                    message: format!("Recognition failed at token {}: {}", i + 1, scan[i].to_string()),
                })
            }
        };

        //TODO refactor to reduce long and duplicated parameter lists
        fn parse_tree<'a>(grammar: &'a Grammar, scan: &'a Vec<Token<String>>, chart: Vec<Vec<Edge<'a>>>) -> Tree {
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
                            if children.is_empty() { //Empty rhs
                                children.push(Tree::null());
                            }
                            children
                        },
                    }
                }
            }

            let start: Node = 0;
            let finish: Node = chart.len() - 1;

            let first_edge = chart[start].iter()
                .find(|edge| edge.finish == finish && edge.rule.unwrap().lhs == *grammar.start());
            match first_edge {
                None => panic!("Failed to find start item to begin parse"),
                Some(edge) => recur(start, edge, grammar, scan, &chart)
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
                    if grammar.is_terminal(symbol) {
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
