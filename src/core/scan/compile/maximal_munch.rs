use std::cmp;

use core::scan;
use core::scan::compile::DFA;
use core::scan::compile::Scanner;
use core::scan::FAIL_SEQUENCE_LENGTH;
use core::scan::Token;
use core::spec::DEF_MATCHER;

pub struct MaximalMunchScanner;

impl<State: PartialEq + Clone> Scanner<State> for MaximalMunchScanner {
    fn scan<'a, 'b>(&self, input: &'a str, dfa: &'b DFA<State>) -> Result<Vec<Token<String>>, scan::Error> {
        fn scan_one<'a, 'b, State: PartialEq + Clone>(input: &'a [char], line: usize, character: usize, dfa: &'b DFA<State>) -> (usize, State, usize, usize) {
            let mut input: &[char] = input;

            let mut scanned: usize = 0;
            let mut state: State = dfa.start.clone();
            let mut line: usize = line;
            let mut character: usize = character;

            let mut last_accepting: (usize, State, usize, usize) = (scanned, state.clone(), line, character);

            while !input.is_empty() && dfa.has_transition(input[0], &state) {
                let head: char = input[0];

                character += 1;
                if head == '\n' {
                    line += 1;
                    character = 1;
                }

                scanned += 1;
                input = &input[1..];

                state = dfa.transition(&state, head);

                if dfa.accepts(&state) {
                    last_accepting = (scanned, state.clone(), line, character);
                }
            }

            last_accepting
        }

        let chars: Vec<char> = input.chars().map(|c| {
            c
        }).collect();

        let mut tokens: Vec<Token<String>> = vec![];
        let mut input: &[char] = &chars;
        let mut line: usize = 1;
        let mut character: usize = 1;

        while !input.is_empty() {
            let (scanned, end_state, end_line, end_character) = scan_one(input, line, character, dfa);

            line = end_line;
            character = end_character;

            let scanned_chars: &[char] = &input[0..scanned];
            input = &input[scanned..];

            if scanned == 0 {
                let seq_len = cmp::min(input.len(), FAIL_SEQUENCE_LENGTH);

                return Err(scan::Error {
                    sequence: input.iter().take(seq_len).collect(),
                    line,
                    character,
                });
            }

            let accept_as = dfa.tokenize(&end_state).unwrap();
            if accept_as != DEF_MATCHER.to_string() {
                let token = Token {
                    kind: accept_as,
                    lexeme: scanned_chars.iter().collect(),
                };
                tokens.push(token);
            }
        }

        Ok(tokens)
    }
}
