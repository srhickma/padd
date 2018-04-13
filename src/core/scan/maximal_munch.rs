use core::scan::DFA;
use core::scan::Token;
use core::scan::State;
use core::scan::Scanner;

pub struct MaximalMunchScanner;

impl Scanner for MaximalMunchScanner {
    fn scan<'a, 'b, 'c>(&self, input: &'a str, dfa: &'b DFA<'c>) -> Vec<Token> {

        fn scan_one<'a, 'b, 'c>(input: &'a [char], state: State<'c>, backtrack: (&'a [char], State<'c>), dfa: &'b DFA<'c>) -> (&'a [char], State<'c>)
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

        fn recur<'a, 'b, 'c>(input: &'a [char], accumulator: &'a mut Vec<Token>, dfa: &'b DFA<'c>) {
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