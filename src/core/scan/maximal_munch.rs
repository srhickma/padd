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

        fn scan_one<'a, 'b>(input: &'a [char], line: usize, character: usize, dfa: &'b DFA) -> (usize, &'b State, usize, usize)
        {
            let mut input: &[char] = input;

            let mut scanned: usize = 0;
            let mut state: &State = &dfa.start;
            let mut line: usize = line;
            let mut character: usize = character;

            let mut last_accepting: (usize, &State, usize, usize) = (scanned, state, line, character);

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
                    scanned += 1;
                    &input[1..]
                };

                state = dfa.transition(state, input[0]);
                input = tail;
                line = new_line;
                character = new_character;

                if dfa.accepts(state) {
                    last_accepting = (scanned, state, line, character);
                }
            }

            return last_accepting;
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

                return Err(ScanningError{
                    sequence: input.iter().take(seq_len).collect(),
                    line,
                    character,
                });
            }

            let accept_as = dfa.tokenize(end_state);
            if accept_as != "_" {
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