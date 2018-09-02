use std::collections::HashSet;
use core::parse::Parser;
use core::parse::Grammar;
use core::parse::Production;
use core::parse::Tree;
use core::scan::Token;

pub struct EarleyParser;

impl Parser for EarleyParser {
    fn parse(&self, scan: Vec<Token>, grammar: &Grammar) -> Option<Tree> {

        //TODO improve using quadratic time algorithm https://github.com/jeffreykegler/kollos/blob/master/notes/misc/loup2.md
        fn build_nss(grammar: &Grammar) ->  HashSet<String> {
            fn update_nss(nss: &mut HashSet<String>, grammar: &Grammar){
                for rule in &grammar.productions {
                    if rule.rhs.iter().all(|symbol| nss.contains(symbol)) && !nss.contains(&rule.lhs) {
                        nss.insert(rule.lhs.clone());
                    }
                }
            }

            let mut nss: HashSet<String> = HashSet::new();
            loop {
                let old_size = nss.len();
                update_nss(&mut nss, grammar);
                if old_size == nss.len() {
                    break;
                }
            }
            nss
        }

        let nss: HashSet<String> = build_nss(grammar);

        let mut chart: Vec<Vec<Item>> = vec![vec![]];
        grammar.productions.iter()
            .filter(|prod| prod.lhs == grammar.start)
            .for_each(|prod| {
                let item = Item{
                    rule: prod,
                    start: 0,
                    next: 0,
                    token: None,
                };
                chart[0].push(item);
            });

        let mut i = 0;
        while i < chart.len() {
            let mut j = 0;
            while j < chart[i].len() {
                let item = chart[i][j].clone();
                let next = (&item).next_symbol();
                match next {
                    None => {
                        let index = item.start;
                        //TODO eliminate this cloning!
                        complete_op(item, &chart[index].clone(), &mut chart[i]);
                    },
                    Some(symbol) => {
                        if grammar.terminals.contains(symbol) {
                            scan_op(item, i, symbol, &scan, &mut chart);
                        } else {
                            predict_op(item, i, symbol, &nss, grammar, &mut chart);
                        }
                    },
                }
                j += 1;
            }
            i += 1;
        }

        fn predict_op<'a, 'b>(item: Item<'a>, i: usize, symbol: &'a str, nss: &HashSet<String>, grammar: &'a Grammar, chart: &'b mut Vec<Vec<Item<'a>>>) {
            grammar.productions.iter()
                .filter(|prod| prod.lhs == symbol)
                .for_each(|prod| {
                    append(
                        Item{
                            rule: prod,
                            start: i,
                            next: 0,
                            token: None,
                        },
                        &mut chart[i]
                    );

                    if nss.contains(&prod.lhs) {
                        append(
                            Item{
                                rule: item.rule,
                                start: item.start,
                                next: item.next + 1,
                                token: None,
                            },
                            &mut chart[i]
                        );
                    }
                });
        }

        fn scan_op<'a, 'b>(item: Item<'a>, i: usize, symbol: &'a str, scan: &'a Vec<Token>, chart: &'b mut Vec<Vec<Item<'a>>>) {
            if i < scan.len() && scan[i].kind == symbol.to_string() {
                if chart.len() <= i + 1 {
                    chart.push(vec![])
                }

                unsafe_append(
                    Item{
                        rule: item.rule,
                        start: item.start,
                        next: item.next + 1,
                        token: Some(&scan[i]),
                    },
                    &mut chart[i+1]
                );
            }
        }

        fn complete_op<'a, 'b>(item: Item<'a>, src: &'b Vec<Item<'a>>, dest: &'b mut Vec<Item<'a>>) {
            src.iter()
                .filter(|old_item| match old_item.clone().next_symbol() {
                        None => false,
                        Some(sym) => sym == item.rule.lhs,
                })
                .for_each(|old_item| append(
                        Item{
                            rule: old_item.rule,
                            start: old_item.start,
                            next: old_item.next + 1,
                            token: None,
                        },
                        dest
                ));
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

//        println!("-----------------------------------------------------");
//        for i in 0..chart.len() {
//            println!("SET {}", i);
//            for j in 0..chart[i].len() {
//                println!("{}", chart[i][j].to_string());
//            }
//            println!();
//        }
//        println!("-----------------------------------------------------");

        fn partial_parse<'a, 'b>(i: usize, grammar: &'a Grammar, chart: &'b mut Vec<Vec<Item<'a>>>) -> (bool, Vec<Item<'a>>) {
            //TODO switch to a usize and remove valid
            let mut complete_parses = vec![];
            let mut res = false;
            for j in 0..chart[i].len() {
                let item = &chart[i][j];
                if item.rule.lhs == grammar.start && item.next >= item.rule.rhs.len() && item.start == 0 {
                    complete_parses.push(item.clone());
                    res = true;
                }
            }
            return (res, complete_parses);
        }

        let (valid, complete_parses) = partial_parse(chart.len() - 1, grammar, &mut chart);

        //TODO this message is not always correct, depricate or fix
        if complete_parses.len() != 1 {
            println!("WARN: Found {} complete parse(s)", complete_parses.len());
        }

        return if valid {
            Some(parse_tree(grammar, &scan, chart))
        } else {
            None
        };

        //TODO refactor to reduce long and duplicated parameter lists
        fn parse_tree<'a>(grammar: &'a Grammar, scan: &'a Vec<Token>, chart: Vec<Vec<Item<'a>>>) -> Tree {

            fn aux<'a>(start: Node, edge: &Edge, grammar: &'a Grammar, scan: &'a Vec<Token>, chart: &Vec<Vec<Edge>>) -> Tree {
                match edge.rule{ //TODO need to figure out what happens for NULL
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
                            let children: Vec<Tree> = top_list(start, edge, grammar, scan, chart).iter().rev()
                                .map(|&(node, ref edge)| aux(node, &edge, grammar, scan, chart))
                                .collect();
                            if children.is_empty() {
                                vec![Tree::null()]
                            } else {
                                children
                            }
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
            for i in 0..chart.len()  {
                for item in &chart[i] {
                    if item.is_complete() {
                        parse_chart[item.start].push(Edge{
                            rule: Some(item.rule),
                            finish: i
                        })
                    }
                }
            }

            let first_edge = parse_chart[start].iter()
                .find(|edge| edge.finish == finish && edge.rule.unwrap().lhs == grammar.start);
            match first_edge {
                None => panic!("Failed to find start item to begin parse"),
                Some(edge) => aux(start, edge, grammar, scan, &parse_chart)
            }
        }

        fn top_list<'a>(start: Node, edge: &Edge, grammar: &'a Grammar, scan: &'a Vec<Token>, chart: &Vec<Vec<Edge<'a>>>) -> Vec<(Node, Edge<'a>)> {
            let symbols: &Vec<String> = &edge.rule.unwrap().rhs;
            let bottom: usize = symbols.len();
            let leaf = |depth: usize, node: Node| depth == bottom && node == edge.finish;
            let child = |edge: &Edge| edge.finish;
            let edges = |depth: usize, node: Node| -> Vec<Edge> {
                if depth >= bottom {
                    vec![]
                } else {
                    let symbol = &symbols[depth];
                    if grammar.terminals.contains(symbol) {
                        if scan[node].kind == *symbol {
                            vec![Edge{
                                rule: None,
                                finish: node + 1
                            }]
                        } else {
                            vec![]
                        }
                    } else { //TODO return iterators instead to avoid collection and cloning
                        chart[node].iter()
                            .filter(|edge| edge.rule.unwrap().lhs == *symbol)
                            .cloned()
                            .collect()
                    }
                }
            };
            match df_search(&edges, &child, &leaf, start) {
                None => panic!("Failed to decompose parse edge of recognized scan"),
                Some(path) => path
            }
        }

        fn df_search<'a>(edges: &Fn(usize, Node) -> Vec<Edge<'a>>,
                     child: &Fn(&Edge) -> Node,
                     leaf: &Fn(usize, Node) -> bool,
                     root: Node)
            -> Option<Vec<(Node, Edge<'a>)>> {

            fn aux<'a>(edges: &Fn(usize, Node) -> Vec<Edge<'a>>,
                       child: &Fn(&Edge) -> Node,
                       leaf: &Fn(usize, Node) -> bool,
                       depth: usize,
                       root: Node)
                -> Option<Vec<(Node, Edge<'a>)>> {

                if leaf(depth, root) {
                    Some(vec![])
                } else {
                    for edge in edges(depth, root) {
                        let mut res = aux(edges, child, leaf, depth + 1, child(&edge));

                        match res {
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

            aux(edges, child, leaf, 0, root)
        }
    }
}

#[derive(PartialEq, Clone)]
struct Item<'a> {
    rule: &'a Production,
    start: usize,
    next: usize,
    token: Option<&'a Token>,
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