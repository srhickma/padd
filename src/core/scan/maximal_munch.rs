use {
    core::{
        data::Data,
        scan::{
            self,
            CDFA,
            FAIL_SEQUENCE_LENGTH,
            Scanner,
            Token,
        },
    },
};

pub struct MaximalMunchScanner;

impl<State: Data, Symbol: Data> Scanner<State, Symbol> for MaximalMunchScanner {
    fn scan<'cdfa>(
        &self,
        input: &[char],
        cdfa: &'cdfa CDFA<State, Symbol>,
    ) -> Result<Vec<Token<Symbol>>, scan::Error> {
        struct ScanOneResult<State> {
            consumed: usize,
            end_state: Option<State>,
            next_start: Option<State>,
            line: usize,
            character: usize,
        }

        fn scan_one<State: Data, Kind: Data>(
            input: &[char],
            start: State,
            line: usize,
            character: usize,
            cdfa: &CDFA<State, Kind>,
        ) -> ScanOneResult<State> {
            let mut remaining = input;
            let mut state: State = start;
            let mut line: usize = line;
            let mut character: usize = character;

            let mut last_accepting = ScanOneResult {
                consumed: 0,
                end_state: None,
                next_start: None,
                line,
                character,
            };

            let mut consumed: usize = 0;

            loop {
                let res = cdfa.transition(&state, remaining);

                consumed += res.consumed;

                for c in remaining.iter().take(res.consumed) {
                    character += 1;
                    if *c == '\n' {
                        line += 1;
                        character = 1;
                    }
                }

                match res.state {
                    None => break,
                    Some(next) => {
                        if cdfa.accepts(&next) {
                            last_accepting = ScanOneResult {
                                consumed,
                                end_state: Some(next.clone()),
                                next_start: cdfa.acceptor_destination(&next, &state),
                                line,
                                character,
                            };
                        }

                        state = next;
                    }
                }

                remaining = &remaining[res.consumed..];
            }

            last_accepting
        }

        let mut remaining = input;
        let mut tokens: Vec<Token<Symbol>> = vec![];
        let mut next_start = cdfa.start();
        let mut line: usize = 1;
        let mut character: usize = 1;

        loop {
            let res: ScanOneResult<State> = scan_one(
                remaining,
                next_start.clone(),
                line,
                character,
                cdfa,
            );

            next_start = match res.next_start {
                None => next_start,
                Some(state) => state
            };

            line = res.line;
            character = res.character;

            match res.end_state {
                None => {
                    if !remaining.is_empty() {
                        let sequence: String = (0..FAIL_SEQUENCE_LENGTH)
                            .map(|i| remaining.get(i))
                            .filter(Option::is_some)
                            .map(Option::unwrap)
                            .collect();

                        return Err(scan::Error {
                            sequence,
                            line,
                            character,
                        });
                    }

                    break;
                }
                Some(state) => if let Some(kind) = cdfa.tokenize(&state) {
                    tokens.push(Token::leaf(kind, (&remaining[0..res.consumed]).iter().collect()));
                }
            }

            remaining = &remaining[res.consumed..];
        }

        Ok(tokens)
    }
}
