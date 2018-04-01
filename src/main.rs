use std::io;
use std::collections::HashSet;
use std::collections::HashMap;

fn main() {
    let mut grammar = Grammar::from(&[
        Production{
            lhs: "S",
            rhs: &["expr"],
        },
        Production{
            lhs: "expr",
            rhs: &["(", "expr", ")"],
        },
        Production{
            lhs: "expr",
            rhs: &["expr", "OP", "expr"],
        },
        Production{
            lhs: "expr",
            rhs: &["ID"],
        }
    ]);

    let scan = vec![
        Token{
            kind: "(".to_string(),
            lexeme: "".to_string(),
        },
        Token{
            kind: "ID".to_string(),
            lexeme: "".to_string(),
        },
        Token{
            kind: "OP".to_string(),
            lexeme: "".to_string(),
        },
        Token{
            kind: "ID".to_string(),
            lexeme: "".to_string(),
        },
        Token{
            kind: ")".to_string(),
            lexeme: "".to_string(),
        },
        Token{
            kind: "OP".to_string(),
            lexeme: "".to_string(),
        },
        Token{
            kind: "ID".to_string(),
            lexeme: "".to_string(),
        },
        Token{
            kind: "OP".to_string(),
            lexeme: "".to_string(),
        },
        Token{
            kind: "(".to_string(),
            lexeme: "".to_string(),
        },
        Token{
            kind: "ID".to_string(),
            lexeme: "".to_string(),
        },
        Token{
            kind: ")".to_string(),
            lexeme: "".to_string(),
        }
    ];

//    let mut grammar = Grammar::from(&[
//        Production{
//            lhs: "S",
//            rhs: &["BOF", "A", "EOF"],
//        },
//        Production{
//            lhs: "A",
//            rhs: &["x"],
//        },
//        Production{
//            lhs: "A",
//            rhs: &["A", "x"],
//        }
//    ]);
//
//    let scan = vec![
//        Token{
//            kind: "BOF".to_string(),
//            lexeme: "".to_string(),
//        },
//        Token{
//            kind: "x".to_string(),
//            lexeme: "".to_string(),
//        },
//        Token{
//            kind: "EOF".to_string(),
//            lexeme: "".to_string(),
//        }
//    ];

//    let grammar = Grammar::from(&[
//        Production{
//            lhs: "Sentence",
//            rhs: &["Noun", "Verb"],
//        },
//        Production{
//            lhs: "Noun",
//            rhs: &["mary"],
//        },
//        Production{
//            lhs: "Verb",
//            rhs: &["runs"],
//        }
//    ]);
//
//    let scan = vec![
//        Token{
//            kind: "mary".to_string(),
//            lexeme: "".to_string(),
//        },
//        Token{
//            kind: "runs".to_string(),
//            lexeme: "".to_string(),
//        }
//    ];

    let res = EarleyParser::parse(scan, &grammar);

    let alphabet = "01";
    let states: [State; 3] = ["start", "0", "not0"];
    let start: State = "start";
    let accepting: [State; 2] = ["0", "not0"];
    let delta: fn(State, char) -> State = |state, c| match (state, c) {
        ("start", '0') => "0",
        ("start", '1') => "not0",
        ("not0", _) => "not0",
        (&_, _) => "",
    };
    let tokenizer: fn(State) -> &str = |state| match state {
        "0" => "ZERO",
        "not0" => "NZ",
        _ => "",
    };

    let dfa = DFA{
        alphabet: &alphabet,
        states: &states,
        start,
        accepting: &accepting,
        delta,
        tokenizer
    };

    loop {
        println!("Input some string");

        let mut input = String::new();

        io::stdin().read_line(&mut input)
            .expect("Failed to read line");

        input.pop(); //Remove trailing newline

        let tokens = MaximalMunchScanner::scan(&input, &dfa);

        println!("Scanned Tokens: {}", tokens.len());

        for token in tokens {
            println!("kind={} lexeme={}", token.kind, token.lexeme)
        }
    }
}

fn unbox<T>(value: Box<T>) -> T {
    *value
}

#[derive(Clone)]
struct Tree {
    lhs: Token,
    children: Vec<Tree>,
}

impl Tree {
    fn print(self){
        self.print_internal("".to_string(), true)
    }
    fn print_internal(self, prefix: String, is_tail: bool) {
        println!("{}{}{}", prefix, if is_tail {"└── "} else {"├── "}, self.lhs.kind);
        let mut i = 0;
        let mut len = self.children.len();
        for child in self.children {
            if i == len - 1{
                child.print_internal(format!("{}{}", prefix, if is_tail {"    "} else {"│   "}), true);
            } else {
                child.print_internal(format!("{}{}", prefix, if is_tail {"    "} else {"│   "}), false);
            }
            i += 1;
        }
    }
}

#[derive(Clone)]
struct Token {
    kind: Kind,
    lexeme: String,
}

#[derive(PartialEq, Clone)]
struct Item<'a> {
    rule: &'a Production<'a>,
    start: usize,
    next: usize,
    completing: Option<Box<Item<'a>>>,
    previous: Option<Box<Item<'a>>>,
}

impl<'a> Item<'a> {
    fn next_symbol<'b>(&'b self) -> Option<&'a str> {
        if self.next >= 0 && self.next < self.rule.rhs.len() {
            return Some(self.rule.rhs[self.next]);
        }
        return None;
    }
    fn prev_symbol<'b>(&'b self) -> Option<&'a str> {
        if self.next > 0 {
            return Some(self.rule.rhs[self.next - 1]);
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

trait Parser {
    fn parse<'a>(scan: Vec<Token>, grammar: &Grammar<'a>) -> Option<Tree>;
}

impl Parser for EarleyParser {
    fn parse<'a>(scan: Vec<Token>, grammar: &Grammar<'a>) -> Option<Tree> {

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
                        completing: None,
                        previous: None,
                    };
                    println!("PREDICTED: {}", item.to_string());
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
                    completing: None,
                    previous: Some(Box::new(item.clone())),
                };
                println!("SCANNED: {} TO {}", item.to_string(), new_item.to_string());
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
                        completing: Some(Box::new(item.clone())),
                        previous: Some(Box::new(old_item.clone())),
                    };
                    println!("COMPLETED: {} TO {}", old_item.to_string(), item.to_string());
                    if dest.contains(&item) {
                        return;
                    }
                    dest.push(item);
                });
        }

        println!("-----------------------------------------------------");
        for i in 0..chart.len() {
            println!("SET {}", i);
            for j in 0..chart[i].len() {
                println!("{}", chart[i][j].to_string());
            }
            println!();
        }
        println!("-----------------------------------------------------");

        fn has_partial_parse<'a, 'b>(i: usize, grammar: &'a Grammar<'a>, chart: &'b mut Vec<Vec<Item<'a>>>) -> (bool, Vec<Item<'a>>) {
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

        let (valid, complete_parses) = has_partial_parse(chart.len() - 1, grammar, &mut chart);

        if !valid {
            return None;
        }


        let mut parse_trees : Vec<Tree> = vec![];
        for item in &complete_parses {
            parse_trees.push(build_nodes(&item).clone());
            //parse_trees.extend(build_nodes(&item).iter().cloned());
        }

        parse_trees.first().unwrap().clone().print();

        fn build_nodes(root: &Item) -> Tree {
            if root.previous.is_some() {
                println!("{} prev={}", root.to_string(), unbox(root.previous.clone().unwrap()).to_string());
            } else {
                println!("{} prev=NONE", root.to_string());
            }
            let down = match root.completing.clone() {
                Some(node) => {
                    println!("SOME({})", &unbox(node.clone()).to_string());
                    build_nodes(&unbox(node))
                },
                None => {
                    println!("NONE");
                    Tree{
                        lhs: Token{
                            kind: root.prev_symbol().unwrap().to_string(),
                            lexeme: "INSERT LEXEME HERE".to_string(),
                        },
                        children: vec![],
                    }
                }
            };

            let mut prev = root.previous.clone();
            let mut left = vec![];
            if prev.is_some() {
                let p: Item = unbox(prev.unwrap());
                if p.next > 0 {
                    println!("PUSHING build_nodes({}) TO LEFT of {}", &p.to_string(), root.to_string());
                    left.extend(build_nodes(&p).children);
                }
            }

            left.push(down);

            return Tree{
                lhs: Token{
                    kind: root.rule.lhs.to_string(),
                    lexeme: "INSERT LEXEME HERE".to_string(),
                },
                children: left,
            };
        }

//        fn build_nodes(root: &Item) -> Vec<Tree> {
//
//            if root.previous.is_some() {
//                println!("{} prev={}", root.to_string(), unbox(root.previous.clone().unwrap()).to_string());
//            } else {
//                println!("{} prev=NONE", root.to_string());
//            }
//            let down = match root.completing.clone() {
//                Some(node) => {
//                    println!("SOME({})", &unbox(node.clone()).to_string());
//                    build_nodes(&unbox(node))
//                },
//                None => {
//                    println!("NONE");
//                    vec![Tree{
//                        lhs: Token{
//                            kind: root.prev_symbol().unwrap().to_string(),
//                            lexeme: "INSERT LEXEME HERE".to_string(),
//                        },
//                        children: vec![],
//                    }]
//                }
//            };
//
//            let mut prev = root.previous.clone();
//            let mut left = vec![];
//            while prev.is_some() {
//                let p: Item = unbox(prev.unwrap());
//                if p.next <= 0 {
//                    break;
//                }
//                println!("PUSHING build_nodes({}) TO LEFT of {}", &p.to_string(), root.to_string());
//                left.push(build_nodes(&p));
//                prev = p.previous;
//            }
//
//            //left.reverse();
//            left.push(down);
//
//            let mut res = vec![];
//
//            for children in left {
//                res.push(Tree{
//                    lhs: Token{
//                        kind: root.rule.lhs.to_string(),
//                        lexeme: "INSERT LEXEME HERE".to_string(),
//                    },
//                    children,
//                });
//            }
//
//            return res;
//        }


        println!("HAS PARTIAL PARSE: {} : {} COMPLETE PARSE(S)", valid, complete_parses.len());

        return None;
    }
}

struct EarleyParser;

struct Grammar<'a> {
    productions: &'a [Production<'a>],
    non_terminals: HashSet<&'a str>,
    terminals: HashSet<&'a str>,
    symbols: HashSet<&'a str>,
    start: &'a str,
    prods_exp: HashMap<&'a str, Vec<&'a Production<'a>>>
}

impl<'a> Grammar<'a> {
    fn from(productions: &'a [Production<'a>]) -> Grammar<'a> {
        let non_terminals: HashSet<&'a str> = productions.iter()
            .map(|prod| prod.lhs)
            .collect();
        let mut symbols: HashSet<&'a str> = productions.iter()
            .flat_map(|prod| prod.rhs.iter())
            .map(|&x| x)
            .collect();
        for non_terminal in &non_terminals {
            symbols.insert(non_terminal);
        }
        let terminals = symbols.difference(&non_terminals)
            .map(|&x| x)
            .collect();

        let mut prods_exp = HashMap::new();

        for prod in productions {
            if !prods_exp.contains_key(prod.lhs) {
                prods_exp.insert(prod.lhs, vec![]);
            }
            prods_exp.get_mut(prod.lhs).unwrap().push(prod);
        }

        return Grammar {
            productions,
            non_terminals,
            terminals,
            symbols,
            start: productions[0].lhs,
            prods_exp,
        };
    }
}

#[derive(PartialEq, Clone)]
struct Production<'a> {
    lhs: &'a str,
    rhs: &'a [&'a str],
}

impl<'a> Production<'a> {
    fn fmt(&self) -> String {
        let mut rhs: String = "".to_string();
        for s in self.rhs {
            rhs.push_str(s);
            rhs.push(' ');
        }
        return format!("{} -> {}", self.lhs, rhs);
    }
}

trait Scanner {
    fn scan<'a>(input: &'a str, dfa: &'a DFA) -> Vec<Token>;
}

struct MaximalMunchScanner;

impl Scanner for MaximalMunchScanner {
    fn scan<'a>(input: &'a str, dfa: &'a DFA) -> Vec<Token> {

        fn scan_one<'a>(input: &'a [char], state: State<'a>, backtrack: (&'a [char], State<'a>), dfa: &'a DFA) -> (&'a [char], State<'a>)
        {
            if input.is_empty() || !dfa.has_transition(input[0], state) {
                if dfa.accepts(state) {
                    return (input, state);
                }
                return backtrack;
            }

            let next_state = dfa.transition(state, input[0]);
            let tail: &[char] = &input[1..];
            let (r_input, end_state) = scan_one(tail, next_state, (input, state), dfa);

            return if dfa.accepts(end_state) {
                (r_input, end_state)
            } else {
                backtrack
            }
        }

        fn recur<'a>(input: &'a [char], accumulator: &'a mut Vec<Token>, dfa: &'a DFA) {
            if input.is_empty() {
                return
            }

            let (r_input, end_state) = scan_one(input, dfa.start, (input, dfa.start), dfa);
            let scanned_chars: &[char] = &input[0..(input.len() - r_input.len())];
            if scanned_chars.is_empty() {
                panic!("Error scanning input");
            }

            let token = Token {
                kind: dfa.tokenize(end_state),
                lexeme: scanned_chars.iter().cloned().collect::<String>(),
            };
            accumulator.push(token);
            recur(r_input, accumulator, dfa);
        }

        let chars : Vec<char> = input.chars().map(|c| {
            c
        }).collect();

        let mut tokens: Vec<Token> = vec![];
        recur(&chars, &mut tokens, dfa);
        return tokens;
    }
}

type State<'a> = &'a str;
type Kind = String;

struct DFA<'a> {
    alphabet: &'a str,
    states: &'a [State<'a>],
    start: State<'a>,
    accepting: &'a [State<'a>],
    delta: fn(State, char) -> State,
    tokenizer: fn(State) -> &str,
}

impl<'a> DFA<'a> {
    fn has_transition(&self, c: char, state: State) -> bool {
        return self.alphabet.chars().any(|x| c == x) && self.transition(state, c) != "";
    }
    fn accepts(&self, state: State) -> bool {
        return self.accepting.contains(&state);
    }
    fn transition(&self, state: State<'a>, c: char) -> State<'a> {
        return (self.delta)(state, c);
    }
    fn tokenize(&self, state: State) -> Kind {
        return (self.tokenizer)(state).to_string();
    }
}
