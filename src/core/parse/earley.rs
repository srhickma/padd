use core::parse::Parser;
use core::parse::Grammar;
use core::parse::Production;
use core::parse::Tree;
use core::scan::Token;

pub struct EarleyParser;

impl Parser for EarleyParser {
    fn parse(&self, scan: Vec<Token>, grammar: &Grammar) -> Option<Tree> {
        let mut parse_chart: Vec<Vec<Edge>> = vec![];
        let mut chart: Vec<Vec<Item>> = vec![vec![]];

        grammar.productions.iter()
            .filter(|prod| prod.lhs == grammar.start)
            .for_each(|prod| {
                let item = Item{
                    rule: prod,
                    start: 0,
                    next: 0,
                };
                chart[0].push(item);
            });

        let mut i = 0;
        while i < chart.len() {
            let mut j = 0;
            parse_chart.push(vec![]);

            while j < chart[i].len() {
                let item = chart[i][j].clone();
                let next = (&item).next_symbol();
                match next {
                    None => {
                        complete_op(&item, i, &mut chart);

                        parse_chart[item.start].push(Edge{
                            rule: Some(item.rule),
                            finish: i
                        });
                    },
                    Some(symbol) => {
                        if grammar.terminals.contains(symbol) {
                            scan_op(&item, i, symbol, &scan, &mut chart);
                        } else {
                            predict_op(&item, i, symbol, grammar, &mut chart);
                        }
                    },
                }
                j += 1;
            }
            i += 1;
        }

        fn predict_op<'a, 'b>(item: &Item<'a>, i: usize, symbol: &'a str, grammar: &'a Grammar, chart: &'b mut Vec<Vec<Item<'a>>>) {
            grammar.productions.iter()
                .filter(|prod| prod.lhs == symbol)
                .for_each(|prod| {
                    append(
                        Item{
                            rule: prod,
                            start: i,
                            next: 0,
                        },
                        &mut chart[i]
                    );

                    if grammar.nullable(&prod) {
                        append(
                            Item{
                                rule: item.rule,
                                start: item.start,
                                next: item.next + 1,
                            },
                            &mut chart[i]
                        );
                    }
                });
        }

        fn scan_op<'a, 'b>(item: &Item<'a>, i: usize, symbol: &'a str, scan: &'a Vec<Token>, chart: &'b mut Vec<Vec<Item<'a>>>) {
            if i < scan.len() && scan[i].kind == symbol.to_string() {
                if chart.len() <= i + 1 {
                    chart.push(vec![])
                }

                unsafe_append(
                    Item{
                        rule: item.rule,
                        start: item.start,
                        next: item.next + 1,
                    },
                    &mut chart[i+1]
                );
            }
        }

        fn complete_op<'a, 'b>(item: &Item<'a>, i: usize, chart: &'b mut Vec<Vec<Item<'a>>>) {
            let mut advanced: Vec<Item> = vec![];

            chart[item.start].iter()
                .filter(|old_item| match old_item.next_symbol() {
                    None => false,
                    Some(sym) => sym == item.rule.lhs,
                })
                .for_each(|old_item| advanced.push(Item{
                    rule: old_item.rule,
                    start: old_item.start,
                    next: old_item.next + 1,
                }));

            for item in advanced {
                append(item, &mut chart[i]);
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

        fn unsafe_append<'a, 'b>(item: Item<'a>, item_set: &'b mut Vec<Item<'a>>){
            item_set.push(item);
        }

        fn recognized<'a, 'b>(grammar: &'a Grammar, chart: &'b Vec<Vec<Item<'a>>>) -> bool {
            chart.last().unwrap().iter()
                .any(|item| item.rule.lhs == grammar.start && item.next >= item.rule.rhs.len() && item.start == 0)
        }

        return if recognized(grammar, &chart) {
            Some(parse_tree(grammar, &scan, parse_chart))
        } else {
            None
        };

        //TODO refactor to reduce long and duplicated parameter lists
        fn parse_tree<'a>(grammar: &'a Grammar, scan: &'a Vec<Token>, chart: Vec<Vec<Edge<'a>>>) -> Tree {

            fn recur<'a>(start: Node, edge: &Edge, grammar: &'a Grammar, scan: &'a Vec<Token>, chart: &Vec<Vec<Edge>>) -> Tree {
                match edge.rule{
                    None => Tree{ //Non-empty rhs
                        lhs: scan[start].clone(),
                        children: vec![],
                    },
                    Some(rule) => Tree{
                        lhs: Token{
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
                .find(|edge| edge.finish == finish && edge.rule.unwrap().lhs == grammar.start);
            match first_edge {
                None => panic!("Failed to find start item to begin parse"),
                Some(edge) => recur(start, edge, grammar, scan, &chart)
            }
        }

        fn top_list<'a>(start: Node,
                        edge: &Edge,
                        grammar: &'a Grammar,
                        scan: &'a Vec<Token>,
                        chart: &Vec<Vec<Edge<'a>>>) -> Vec<(Node, Edge<'a>)> {
            let symbols: &Vec<String> = &edge.rule.unwrap().rhs;
            let bottom: usize = symbols.len();
            let leaf = |depth: usize, node: Node| depth == bottom && node == edge.finish;
            let edges = |depth: usize, node: Node| -> Vec<Edge> {
                if depth < bottom {
                    let symbol = &symbols[depth];
                    if grammar.terminals.contains(symbol) {
                        if scan[node].kind == *symbol {
                            return vec![Edge{
                                rule: None,
                                finish: node + 1
                            }]
                        }
                    } else { //TODO return iterators instead to avoid collection and cloning
                        return chart[node].iter()
                            .filter(|edge| edge.rule.unwrap().lhs == *symbol)
                            .cloned()
                            .collect()
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
                            None => {},
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

#[derive(PartialEq, Clone)]
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

    #[allow(dead_code)]
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

#[derive(Clone)]
struct Edge<'a> {
    rule: Option<&'a Production>,
    finish: usize,
}

impl<'a> Edge<'a> {
    #[allow(dead_code)]
    fn to_string(&'a self) -> String {
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