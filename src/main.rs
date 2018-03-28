use std::io;
use std::collections::HashSet;
use std::collections::HashMap;

fn main() {
    println!("Hello, world!");

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

struct Token {
    kind: Kind,
    lexeme: String,
}

trait Parser {
    fn parse<'a>(scan: Vec<Token>, grammar: &Grammar<'a>) -> Option<Vec<Tree>>;
}

impl Parser for CYKParser {
    fn parse<'a>(scan: Vec<Token>, grammar: &Grammar<'a>) -> Option<Vec<Tree>> {
        fn recur<'a>(lhs: &'a[&'a str], from: usize, length: usize,
                 scan: Vec<Token>,
                 grammar: &Grammar<'a>,
                 memo: &'a mut HashMap<(&'a [&'a str], usize, usize), Option<Vec<Tree>>>)
            -> Option<Vec<Tree>> {
            let key = (lhs, from, length);
            if memo.contains_key(&key) {
                return (*(memo.get(&key).unwrap())).unwrap().clone();
            }
            memo.insert(key, None);

            fn return_captor<'a>(result: Option<Vec<Tree>>,
                key: (&'a [&'a str], usize, usize),
                memo: &'a mut HashMap<(&'a [&'a str], usize, usize), Option<Vec<Tree>>>)
                -> Option<Vec<Tree>> {
                memo.insert(key, result);
                return result;
            }

            if lhs.is_empty() {
                if length == 0 {
                    return return_captor(Some(vec![]), key, memo);
                }
            } else if grammar.terminals.contains(lhs[0]) {
                let a = lhs[0];
                let beta = &lhs[1..];
                if length == 0 || scan[from].kind != a {
                    return return_captor(None, key, memo);
                }
                let res = recur(beta, from + 1, length - 1, scan, grammar, memo);
                if res.is_some() {
                    let tree = Tree{
                        lhs: Token {
                            kind: lhs[0].to_string(),
                            lexeme: "".to_string(),
                        },
                        children: res.unwrap(),
                    };
                    return return_captor(Some(vec![tree]), key, memo);
                }
            } else if lhs.len() == 1 && grammar.non_terminals.contains(lhs[0]) {
                for gamma in grammar.prods_exp.get(lhs[0]).unwrap() {
                    let res = recur(gamma.rhs, from, length, scan, grammar, memo);
                    if res.is_some() {
                        let tree = Tree{
                            lhs: Token {
                                kind: lhs[0].to_string(),
                                lexeme: "".to_string(),
                            },
                            children: res.unwrap(),
                        };
                        return return_captor(Some(vec![tree]), key, memo);
                    }
                }
            } else {
                let a_nt = lhs[0];
                let beta = &lhs[1..];
                let new_lhs = &[a_nt];
                for i in 0..(length + 1) {
                    let res1 = recur(new_lhs, from, i, scan, grammar, memo);
                    let res2 = recur(beta, from + i, length - i, scan, grammar, memo);
                    if res1.is_some() && res2.is_some() {
                        let mut res = res1.unwrap();
                        res.append(&mut res2.unwrap());
                        return return_captor(Some(res), key, memo);
                    }
                }
            }

            return return_captor(None, key, memo);
        }

        let lhs = &[grammar.start];
        let mut memo = HashMap::new();

        return recur(lhs, 0, scan.len(), scan, grammar, &mut memo);
    }
}

struct CYKParser;

struct Tree {
    lhs: Token,
    children: Vec<Tree>,
}

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

struct Production<'a> {
    lhs: &'a str,
    rhs: &'a [&'a str],
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
