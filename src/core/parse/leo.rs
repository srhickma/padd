//extern crate stopwatch;
//
//use core::data::Data;
//use core::parse;
//use core::parse::grammar::Grammar;
//use core::parse::Parser;
//use core::parse::Production;
//use core::parse::Tree;
//use core::scan::Token;
//
//use self::stopwatch::Stopwatch;
//
//pub struct LeoParser;
//
//impl Parser for LeoParser {
//    fn parse(&self, scan: Vec<Token<String>>, grammar: &Grammar) -> Result<Tree, parse::Error> {
//        let sw = Stopwatch::start_new();
//
//        let mut chart: Vec<Row> = vec![Row::new()];
//
//        //TODO ensure topmost item by creating S-prime start production
//
//        grammar.productions.iter()
//            .filter(|prod| prod.lhs == grammar.start)
//            .for_each(|prod| {
//                let item = Item {
//                    rule: prod,
//                    start: 0,
//                    next: 0,
//                };
//                chart[0].items.push(item);
//            });
//
//        println!("Creating recognition chart took {}", sw.elapsed_ms());
//
//        predict_full(0, grammar, &mut chart);
//
//        let mut i = 1;
//        while i <= chart.len() && i <= scan.len() {
//            println!("-----------------------------------------------------");
//            println!("SET {}", i);
//
//            // Scanning
//            let mut next_row = Row::new();
//            let a_i = &scan[i - 1].kind[..];
//            cross(&chart[i - 1].items, a_i, &mut next_row.items, grammar);
//            if next_row.items.is_empty() {
//                break;
//            }
//            chart.push(next_row);
//
//            // Completion
//            let mut j = 0;
//            while j < chart[i].items.len() {
//                let item = chart[i].items[j].clone();
//                let next = (&item).next_symbol();
//                match next {
//                    None => {
//                        t_update(&item, grammar, &mut chart);
//
//                        let mut trans_res: Option<Item> = None;
//                        for t_item in &chart[item.start].transitive_items {
//                            if t_item.captor == item.rule.lhs {
//                                trans_res = Some(Item { //TODO abstract into factory method
//                                    rule: t_item.rule,
//                                    start: t_item.start,
//                                    next: t_item.next,
//                                });
//                                break;
//                            }
//                        }
//
//                        match trans_res {
//                            Some(item) => append(item, &mut chart[i].items),
//                            None => {
//                                let mut accumulator: Vec<Item> = Vec::new();
//                                cross(&chart[item.start].items, &item.rule.lhs[..], &mut accumulator, grammar);
//                                for completed_item in accumulator {
//                                    append(completed_item, &mut chart[i].items)
//                                }
//                            }
//                        }
//                    }
//                    Some(symbol) => {}
//                }
//                j += 1;
//            }
//
//            // Prediction
//            predict_full(i, grammar, &mut chart);
//
//            i += 1;
//        }
//
//        println!("Filling recognition chart took {} for {} tokens", sw.elapsed_ms(), scan.len());
//
//        fn t_update<'a, 'b>(
//            item: &Item<'a>,
//            grammar: &'a Grammar,
//            chart: &'b mut Vec<Row<'a>>,
//        ) -> Item<'a> {
//            println!("t_update for {}", item.to_string());
//
//            let mut trans_res: Option<Item> = None;
//            for t_item in &chart[item.start].transitive_items {
//                println!("{} vs {}", t_item.captor, item.rule.lhs);
//                if t_item.captor == item.rule.lhs {
//                    trans_res = Some(Item { //TODO abstract into factory method
//                        rule: t_item.rule,
//                        start: t_item.start,
//                        next: t_item.next,
//                    });
//                    break;
//                }
//            }
//
//            match trans_res {
//                Some(trans_item) => return trans_item,
//                None => {
//                    println!("NO TRANS");
//
//                    let mut target_res: Option<Item> = None;
//                    for step_back_item in &chart[item.start].items {
//                        match step_back_item.next_symbol() {
//                            None => {}
//                            Some(sym) => {
//                                println!("lhs={} sym={}", &item.rule.lhs[..], sym);
//                                if &item.rule.lhs[..] == sym {
//                                    // Allow at most one target_res
//                                    if target_res.is_some() {
//                                        target_res = None;
//                                        break;
//                                    }
//
//                                    target_res = Some(step_back_item.clone());
//                                }
//                            }
//                        }
//                    }
//
//                    match target_res {
//                        Some(target_item) => {
//                            println!("TARGET CANDIDATE");
//                            //if target_item.is_quasi_complete_in(grammar) {
//                            if target_item.is_quasi_complete_in(grammar) {
//                                println!("QUASI COMPLETE");
//
//                                let new_item = Item {
//                                    rule: target_item.rule,
//                                    start: target_item.start,
//                                    next: target_item.rule.rhs.len(),
//                                };
//
//                                let rec_item = t_update(&new_item, grammar, chart);
//                                let transtive_item = TransitiveItem {
//                                    rule: rec_item.rule,
//                                    start: rec_item.start,
//                                    next: rec_item.next,
//                                    captor: item.rule.lhs.clone(),
//                                };
//
//                                println!("LEVEL ITEM {}", transtive_item.to_string());
//
//                                // Add transition item
//                                let mut has_transition_item = false;
//                                for t_item in &chart[item.start].transitive_items {
//                                    if *t_item == transtive_item {
//                                        has_transition_item = true;
//                                    }
//                                }
//
//                                if !has_transition_item {
//                                    println!("TRANSITIVE {}", transtive_item.to_string());
//                                    chart[item.start].transitive_items.push(transtive_item);
//                                }
//
//                                return rec_item;
//                            }
//                        }
//                        None => {}
//                    }
//                    println!("MISS TARGET");
//                    return item.clone();
//                }
//            }
//        }
//
//        fn cross<'a>(
//            src: &Vec<Item<'a>>,
//            symbol: &'a str,
//            dest: &mut Vec<Item<'a>>,
//            grammar: &'a Grammar,
//        ) {
//            for item in src {
//                let next = item.next_symbol();
//                match next {
//                    None => {}
//                    Some(sym) => {
//                        if sym == symbol {
//                            let mut last_item = Item {
//                                rule: item.rule,
//                                start: item.start,
//                                next: item.next + 1,
//                            };
//
//                            unsafe_append(last_item.clone(), dest);
//
//                            loop {
//                                match last_item.next_symbol() {
//                                    None => break,
//                                    Some(sym) => {
//                                        let sym_string = sym.to_string();
//                                        if !grammar.nullable_nt(&sym_string) {
//                                            break;
//                                        }
//                                        last_item = Item {
//                                            rule: last_item.rule,
//                                            start: last_item.start,
//                                            next: last_item.next + 1,
//                                        };
//
//                                        unsafe_append(last_item.clone(), dest);
//                                    }
//                                }
//                            }
//                        }
//                    }
//                }
//            }
//        }
//
//        fn predict_full<'a, 'b>(i: usize, grammar: &'a Grammar, chart: &'b mut Vec<Row<'a>>) {
//            let mut k = 0;
//            while k < chart[i].items.len() {
//                let item = chart[i].items[k].clone();
//                match (&item).next_symbol() {
//                    None => {}
//                    Some(symbol) => {
//                        if !grammar.terminals.contains(symbol) {
//                            predict_op(&item, i, symbol, grammar, chart);
//                        }
//                    }
//                }
//
//                k += 1;
//            }
//        }
//
//        fn predict_op<'a, 'b>(item: &Item<'a>, i: usize, symbol: &'a str, grammar: &'a Grammar, chart: &'b mut Vec<Row<'a>>) {
//            grammar.productions.iter()
//                .filter(|prod| prod.lhs == symbol)
//                .for_each(|prod| {
//                    append(
//                        Item {
//                            rule: prod,
//                            start: i,
//                            next: 0,
//                        },
//                        &mut chart[i].items,
//                    );
//
//                    if grammar.nullable(&prod) {
//                        append(
//                            Item {
//                                rule: item.rule,
//                                start: item.start,
//                                next: item.next + 1,
//                            },
//                            &mut chart[i].items,
//                        );
//                    }
//                });
//        }
//
//        fn append<'a, 'b>(item: Item<'a>, item_set: &'b mut Vec<Item<'a>>) {
//            for j in 0..item_set.len() {
//                if item_set[j] == item {
//                    return;
//                }
//            }
//            unsafe_append(item, item_set);
//        }
//
//        fn unsafe_append<'a, 'b>(item: Item<'a>, item_set: &'b mut Vec<Item<'a>>) {
//            item_set.push(item);
//        }
//
//        fn recognized<'a, 'b>(grammar: &'a Grammar, chart: &'b Vec<Row<'a>>) -> bool {
//            chart.last().unwrap().items.iter()
//                .any(|item| item.rule.lhs == grammar.start
//                    && item.next >= item.rule.rhs.len()
//                    && item.start == 0)
//        }
//
//        println!("-----------------------------------------------------");
//        for i in 0..chart.len() {
//            println!("SET {}", i);
//            for j in 0..chart[i].items.len() {
//                println!("{}", chart[i].items[j].to_string());
//            }
//            for j in 0..chart[i].transitive_items.len() {
//                println!("{}", chart[i].transitive_items[j].to_string());
//            }
//            println!();
//        }
//        println!("-----------------------------------------------------");
//
//        return if recognized(grammar, &chart) {
//            if i - 1 == scan.len() {
//                Ok(parse_tree(grammar, &scan, chart))
//            } else {
//                Err(parse::Error {
//                    message: format!("Largest parse did not consume all tokens: {} of {}", i - 1, scan.len()),
//                })
//            }
//        } else {
//            if scan.len() == 0 {
//                Err(parse::Error {
//                    message: "No tokens scanned".to_string(),
//                })
//            } else if i - 1 == scan.len() {
//                Err(parse::Error {
//                    message: format!("Recognition failed after consuming all tokens"),
//                })
//            } else {
//                Err(parse::Error {
//                    message: format!("Recognition failed at token {}: {}", i, scan[i - 1].to_string()),
//                })
//            }
//        };
//
//        //TODO refactor to reduce long and duplicated parameter lists
//        fn parse_tree<'a>(grammar: &'a Grammar, scan: &'a Vec<Token<String>>, chart: Vec<Row<'a>>) -> Tree {
//            fn recur<'a>(start: Node, edge: &Edge, grammar: &'a Grammar, scan: &'a Vec<Token<String>>, chart: &Vec<Vec<Edge>>) -> Tree {
//                match edge.rule {
//                    None => Tree { //Non-empty rhs
//                        lhs: scan[start].clone(),
//                        children: vec![],
//                    },
//                    Some(rule) => Tree {
//                        lhs: Token {
//                            kind: rule.lhs.clone(),
//                            lexeme: String::new(),
//                        },
//                        children: {
//                            let mut children: Vec<Tree> =
//                                top_list(start, edge, grammar, scan, chart).iter().rev()
//                                    .map(|&(node, ref edge)| recur(node, &edge, grammar, scan, chart))
//                                    .collect();
//                            if children.is_empty() { //empty rhs
//                                children.push(Tree::null());
//                            }
//                            children
//                        },
//                    }
//                }
//            }
//
//            let start: Node = 0;
//            let finish: Node = chart.len() - 1;
//
//            //TODO build the parse chart during the main loop
//            let mut parse_chart: Vec<Vec<Edge>> = Vec::with_capacity(chart.len());
//            for _ in 0..chart.len() {
//                parse_chart.push(vec![]);
//            }
//            for i in 0..chart.len() {
//                for item in &chart[i].items {
//                    if item.is_complete() {
//                        parse_chart[item.start].push(Edge {
//                            rule: Some(item.rule),
//                            finish: i,
//                        })
//                    }
//                }
//            }
//
////            println!("-----------------------------------------------------");
////            for i in 0..parse_chart.len() {
////                println!("SET {}", i);
////                for j in 0..parse_chart[i].len() {
////                    println!("{}", parse_chart[i][j].to_string());
////                }
////                println!();
////            }
////            println!("-----------------------------------------------------");
//
//            let first_edge = parse_chart[start].iter()
//                .find(|edge| edge.finish == finish && edge.rule.unwrap().lhs == grammar.start);
//            match first_edge {
//                None => panic!("Failed to find start item to begin parse"),
//                Some(edge) => recur(start, edge, grammar, scan, &parse_chart)
//            }
//        }
//
//
//        fn top_list<'a>(start: Node,
//                        edge: &Edge,
//                        grammar: &'a Grammar,
//                        scan: &'a Vec<Token<String>>,
//                        chart: &Vec<Vec<Edge<'a>>>) -> Vec<(Node, Edge<'a>)> {
//            let symbols: &Vec<String> = &edge.rule.unwrap().rhs;
//            let bottom: usize = symbols.len();
//            let leaf = |depth: usize, node: Node| depth == bottom && node == edge.finish;
//            let edges = |depth: usize, node: Node| -> Vec<Edge> {
//                if depth < bottom {
//                    let symbol = &symbols[depth];
//                    if grammar.terminals.contains(symbol) {
//                        if scan[node].kind == *symbol {
//                            return vec![Edge {
//                                rule: None,
//                                finish: node + 1,
//                            }];
//                        }
//                    } else { //TODO return iterators instead to avoid collection and cloning
//                        return chart[node].iter()
//                            .filter(|edge| edge.rule.unwrap().lhs == *symbol)
//                            .cloned()
//                            .collect();
//                    }
//                }
//                vec![]
//            };
//
//            fn df_search<'a>(edges: &Fn(usize, Node) -> Vec<Edge<'a>>,
//                             leaf: &Fn(usize, Node) -> bool,
//                             depth: usize,
//                             root: Node) -> Option<Vec<(Node, Edge<'a>)>> {
//                if leaf(depth, root) {
//                    Some(vec![])
//                } else {
//                    for edge in edges(depth, root) {
//                        match df_search(edges, leaf, depth + 1, edge.finish) {
//                            None => {}
//                            Some(mut path) => {
//                                path.push((root, edge));
//                                return Some(path);
//                            }
//                        }
//                    }
//                    None
//                }
//            }
//
//            match df_search(&edges, &leaf, 0, start) {
//                None => panic!("Failed to decompose parse edge of recognized scan"),
//                Some(path) => path
//            }
//        }
//    }
//}
//
////TODO should use sets, so that we do not have to use the slow append
//struct Row<'a> {
//    items: Vec<Item<'a>>,
//    transitive_items: Vec<TransitiveItem<'a>>,
//}
//
//impl<'a> Row<'a> {
//    fn new() -> Row<'a> {
//        Row {
//            items: Vec::new(),
//            transitive_items: Vec::new(),
//        }
//    }
//}
//
//#[derive(PartialEq, Clone, Debug)]
//struct TransitiveItem<'a> {
//    rule: &'a Production,
//    start: usize,
//    next: usize,
//    captor: String, // Non-Terminal
//}
//
//impl<'a> Data for TransitiveItem<'a> {
//    #[cfg_attr(tarpaulin, skip)]
//    fn to_string(&self) -> String {
//        let mut rule_string = format!("{} -> ", self.rule.lhs);
//        for i in 0..self.rule.rhs.len() {
//            if i == self.next {
//                rule_string.push_str(". ");
//            }
//            rule_string.push_str(self.rule.rhs.get(i).unwrap());
//            rule_string.push(' ');
//        }
//        if self.next == self.rule.rhs.len() {
//            rule_string.push_str(". ");
//        }
//        format!("{} ({}, {})", rule_string, self.captor, self.start)
//    }
//}
//
//#[derive(PartialEq, Clone, Debug)]
//struct Item<'a> {
//    rule: &'a Production,
//    start: usize,
//    next: usize,
//}
//
//impl<'a> Item<'a> {
//    fn next_symbol<'b>(&'b self) -> Option<&'a str> {
//        if self.next < self.rule.rhs.len() {
//            Some(&self.rule.rhs[self.next][..])
//        } else {
//            None
//        }
//    }
//
//    fn is_complete(&self) -> bool {
//        self.next >= self.rule.rhs.len()
//    }
//
//    fn is_quasi_complete_in(&self, grammar: &Grammar) -> bool {
////        let mut next = self.next;
////        let mut item = self.next_symbol();
////
////        for i in next..self.rule.rhs.len() {
////            //TODO find a better way to do this so that we don't have to convert to strings
////            //TODO this could be solved when the parser is encrypted
////            if !grammar.nullable_nt(&self.rule.rhs[next]) {
////                return false;
////            }
////
////            next += 1;
////        }
////
////        true
//        self.next == self.rule.rhs.len() - 1
//    }
//}
//
//impl<'a> Data for Item<'a> {
//    #[cfg_attr(tarpaulin, skip)]
//    fn to_string(&self) -> String {
//        let mut rule_string = format!("{} -> ", self.rule.lhs);
//        for i in 0..self.rule.rhs.len() {
//            if i == self.next {
//                rule_string.push_str(". ");
//            }
//            rule_string.push_str(self.rule.rhs.get(i).unwrap());
//            rule_string.push(' ');
//        }
//        if self.next == self.rule.rhs.len() {
//            rule_string.push_str(". ");
//        }
//        format!("{} ({})", rule_string, self.start)
//    }
//}
//
//#[derive(Clone, PartialEq, Debug)]
//struct Edge<'a> {
//    rule: Option<&'a Production>,
//    finish: usize,
//}
//
//impl<'a> Data for Edge<'a> {
//    #[cfg_attr(tarpaulin, skip)]
//    fn to_string(&self) -> String {
//        match self.rule {
//            None => format!("NONE ({})", self.finish),
//            Some(rule) => {
//                let mut rule_string = format!("{} -> ", rule.lhs);
//                for i in 0..rule.rhs.len() {
//                    rule_string.push_str(rule.rhs.get(i).unwrap());
//                    rule_string.push(' ');
//                }
//                format!("{} ({})", rule_string, self.finish)
//            }
//        }
//    }
//}
//
//type Node = usize;
