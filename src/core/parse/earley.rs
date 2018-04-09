use core::parse::Parser;
use core::parse::Grammar;
use core::parse::Production;
use core::parse::Tree;
use core::scan::Token;

pub struct EarleyParser;

impl Parser for EarleyParser {
    fn parse<'a>(&self, scan: Vec<Token>, grammar: &Grammar<'a>) -> Option<Tree> {

        fn append<'a, 'b>(i: usize, item: Item<'a>, chart: &'b mut Vec<Vec<Item<'a>>>) {
            for j in 0..chart[i].len() {
                if chart[i][j] == item {
                    return;
                }
            }
            chart[i].push(item);
        }

        let mut chart: Vec<Vec<Item>> = vec![vec![]];
        grammar.productions.iter()
            .filter(|prod| prod.lhs == grammar.start)
            .for_each(|prod| {
                let item = Item{
                    rule: prod,
                    start: 0,
                    next: 0,
                    token: None,
                    completing: None,
                    previous: None,
                };
                chart[0].push(item);
            });

        let mut i = 0;
        while i < chart.len() {
            let mut j = 0;
            while j < chart[i].len() {
                let item = chart[i][j].clone();
                let symbol = (&item).next_symbol();
                match symbol {
                    None => {
                        let index = item.start;
                        complete_op(item, &chart[index].clone(), &mut chart[i]);
                    },
                    Some(sym) => {
                        if grammar.terminals.contains(&sym) {
                            scan_op(i, j, sym, &scan, &mut chart);
                        } else {
                            predict_op(i, sym, grammar, &mut chart);
                        }
                    },
                }
                j += 1;
            }
            i += 1;
        }

        fn predict_op<'a, 'b>(i: usize, symbol: &'a str, grammar: &'a Grammar<'a>, chart: &'b mut Vec<Vec<Item<'a>>>) {
            grammar.productions.iter()
                .filter(|prod| prod.lhs == symbol)
                .for_each(|prod| {
                    let item = Item{
                        rule: prod,
                        start: i,
                        next: 0,
                        token: None,
                        completing: None,
                        previous: None,
                    };
                    append(i, item, chart);
                });
        }

        fn scan_op<'a, 'b>(i: usize, j: usize, symbol: &'a str, scan: &'a Vec<Token>, chart: &'b mut Vec<Vec<Item<'a>>>) {
            if i < scan.len() && scan[i].kind == symbol.to_string() {
                if chart.len() <= i + 1 {
                    chart.push(vec![])
                }
                let item = chart[i][j].clone();
                let new_item = Item{
                    rule: item.rule,
                    start: item.start,
                    next: item.next + 1,
                    token: Some(&scan[i]),
                    completing: None,
                    previous: Some(Box::new(item.clone())),
                };
                chart[i + 1].push(new_item);
            }
        }

        fn complete_op<'a, 'b>(item: Item<'a>, src: &'b Vec<Item<'a>>, dest: &'b mut Vec<Item<'a>>){
            src.iter()
                .filter(|old_item| {
                    match old_item.clone().next_symbol() {
                        None => false,
                        Some(sym) => sym == item.rule.lhs,
                    }
                })
                .for_each(|old_item| {
                    let item = Item{
                        rule: old_item.rule,
                        start: old_item.start,
                        next: old_item.next + 1,
                        token: None,
                        completing: Some(Box::new(item.clone())),
                        previous: Some(Box::new(old_item.clone())),
                    };
                    if dest.contains(&item) {
                        return;
                    }
                    dest.push(item);
                });
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

        fn partial_parse<'a, 'b>(i: usize, grammar: &'a Grammar<'a>, chart: &'b mut Vec<Vec<Item<'a>>>) -> (bool, Vec<Item<'a>>) {
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

        println!("FOUND {} COMPLETE PARSE(S)", complete_parses.len());

        if !valid {
            return None;
        }

        let first_parse : Tree = build_nodes(&complete_parses.first().unwrap());
        return Some(first_parse);

        fn build_nodes(root: &Item) -> Tree {
            let down = match root.completing.clone() {
                Some(node) => {
                    build_nodes(&unbox(node))
                },
                None => {
                    match root.token { //Leaf Node Creation
                        Some(t) => Tree{ //Non-empty rhs
                            lhs: root.token.unwrap().clone(),
                            children: vec![],
                        },
                        None => Tree::null(), //Empty rhs
                    }

                }
            };

            let prev = root.previous.clone();
            let mut left = vec![];
            if prev.is_some() {
                let p: Item = unbox(prev.unwrap());
                if p.next > 0 {
                    left.extend(build_nodes(&p).children);
                }
            }

            left.push(down);

            return Tree{ //Inner Node Creation
                lhs: Token{
                    kind: root.rule.lhs.to_string(),
                    lexeme: "".to_string(),
                },
                children: left,
            };
        }
    }
}

#[derive(PartialEq, Clone)]
struct Item<'a> {
    rule: &'a Production<'a>,
    start: usize,
    next: usize,
    token: Option<&'a Token>,
    completing: Option<Box<Item<'a>>>,
    previous: Option<Box<Item<'a>>>,
}

impl<'a> Item<'a> {
    fn next_symbol<'b>(&'b self) -> Option<&'a str> {
        if self.next < self.rule.rhs.len() {
            return Some(self.rule.rhs[self.next]);
        }
        return None;
    }
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
        return format!("{}: {} p:{} c:{}", rule_string, self.start, if self.previous.is_some() {"SOME"} else {"NONE"}, if self.completing.is_some() {"SOME"} else {"NONE"});
    }
}

fn unbox<T>(value: Box<T>) -> T {
    *value
}