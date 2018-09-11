use core::scan;
use core::scan::DFA;
use core::scan::Token;
use core::scan::State;
use core::scan::Scanner;
use core::scan::FAIL_SEQUENCE_LENGTH;
use std::cmp;

pub struct MaximalMunchScanner;

impl Scanner for MaximalMunchScanner {
    fn scan<'a, 'b>(&self, input: &'a str, dfa: &'b DFA) -> Result<Vec<Token>, scan::Error> {

        fn scan_one<'a, 'b>(input: &'a [char], line: usize, character: usize, dfa: &'b DFA) -> (usize, &'b State, usize, usize)
        {
            let mut input: &[char] = input;

            let mut scanned: usize = 0;
            let mut state: &State = &dfa.start;
            let mut line: usize = line;
            let mut character: usize = character;

            let mut last_accepting: (usize, &State, usize, usize) = (scanned, state, line, character);

            while !input.is_empty() && dfa.has_transition(input[0], state) {
                let head: char = input[0];

                character += 1;
                if head == '\n' {
                    line += 1;
                    character = 1;
                }

                //TODO remove with CDFA
                if state.chars().next().unwrap() != '#' || dfa.td.has_non_def_transition(head, state) {
                    scanned += 1;
                    input = &input[1..];
                }

                state = dfa.transition(state, head);

                if dfa.accepts(state) {
                    last_accepting = (scanned, state, line, character);
                }
            }

            last_accepting
        }

        let chars : Vec<char> = input.chars().map(|c| {
            c
        }).collect();

        let mut tokens: Vec<Token> = vec![];
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

                return Err(scan::Error{
                    sequence: input.iter().take(seq_len).collect(),
                    line,
                    character,
                });
            }

            let accept_as = dfa.tokenize(end_state).unwrap();
            if accept_as != "_" { //TODO replace with def matcher
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