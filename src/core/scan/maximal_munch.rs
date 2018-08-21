use core::scan::DFA;
use core::scan::Token;
use core::scan::State;
use core::scan::Scanner;
use core::scan::ScanningError;
use core::scan::FAIL_SEQUENCE_LENGTH;
use std::cmp;

pub struct MaximalMunchScanner;

impl Scanner for MaximalMunchScanner {
    fn scan<'a, 'b>(&self, input: &'a str, dfa: &'b DFA) -> Result<Vec<Token>, ScanningError> {

        fn scan_one<'a, 'b>(input: &'a [char], state: &'b State, line: usize, character: usize, backtrack: (&'a [char], &'b State, usize, usize), dfa: &'b DFA) -> (&'a [char], &'b State, usize, usize)
        {
            let mut input: &[char] = input;
            let mut state: &State = state;
            let mut line: usize = line;
            let mut character: usize = character;
            let mut last_accepting: (&[char], &State, usize, usize) = backtrack;

            while !input.is_empty() && dfa.has_transition(input[0], state) {
                let (new_line, new_character) = if input[0] == '\n' {
                    (line + 1, 1)
                } else {
                    (line, character + 1)
                };

                //TODO remove with CDFA
                let tail: &[char] = if state.chars().next().unwrap() == '#' && !dfa.td.has_non_def_transition(input[0], state) {
                    input
                } else {
                    &input[1..]
                };

                state = dfa.transition(state, input[0]);
                input = tail;
                line = new_line;
                character = new_character;

                if dfa.accepts(state) {
                    last_accepting = (input, state, line, character);
                }
            }

            return last_accepting;
        }

        fn recur<'a, 'b>(input: &'a [char], accumulator: &'a mut Vec<Token>, dfa: &'b DFA, line: usize, character: usize) -> Option<ScanningError> {
            if input.is_empty() {
                return None;
            }

            let (r_input, end_state, end_line, end_character) = scan_one(input, &dfa.start, line, character, (input, &dfa.start, line, character), dfa);
            let scanned_chars: &[char] = &input[0..(input.len() - r_input.len())];
            if scanned_chars.is_empty() {
                let seq_len = cmp::min(r_input.len(), FAIL_SEQUENCE_LENGTH);
                let mut sequence: String = String::with_capacity(seq_len);
                for c in &r_input[..seq_len] {
                    sequence.push(*c);
                }
                return Some(ScanningError{
                    sequence,
                    line: end_line,
                    character: end_character,
                });
            }

            let accept_as = dfa.tokenize(end_state);
            if accept_as != "_" {
                let token = Token {
                    kind: accept_as,
                    lexeme: scanned_chars.iter().cloned().collect::<String>(),
                };
                accumulator.push(token);
            }

            recur(r_input, accumulator, dfa, end_line, end_character)
        }

        let chars : Vec<char> = input.chars().map(|c| {
            c
        }).collect();

        let mut tokens: Vec<Token> = vec![];
        let err = recur(&chars, &mut tokens, dfa, 1, 1);
        match err {
            Some(se) => Err(se),
            None => Ok(tokens),
        }
    }
}