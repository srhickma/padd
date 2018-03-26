use std::io;

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

//trait Parser {
//
//}

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
