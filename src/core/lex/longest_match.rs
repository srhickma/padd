use core::{
    data::Data,
    lex::{self, Lexer, Token, CDFA, FAIL_SEQUENCE_LENGTH},
    parse::grammar::GrammarSymbol,
};

pub struct LongestMatchLexer;

impl<State: Data, Symbol: GrammarSymbol> Lexer<State, Symbol> for LongestMatchLexer {
    fn lex<'cdfa>(
        &self,
        input: &[char],
        cdfa: &'cdfa CDFA<State, Symbol>,
    ) -> Result<Vec<Token<Symbol>>, lex::Error> {
        struct ScanOneResult<State> {
            consumed: usize,
            end_state: Option<State>,
            next_start: Option<State>,
            line: usize,
            character: usize,
        }

        fn scan_one<State: Data, Symbol: GrammarSymbol>(
            input: &[char],
            start: State,
            line: usize,
            character: usize,
            cdfa: &CDFA<State, Symbol>,
        ) -> ScanOneResult<State> {
            let mut remaining = input;
            let mut state: State = start;
            let mut line: usize = line;
            let mut character: usize = character;

            let next_start = cdfa.acceptor_destination(&state, &state);

            let end_state = if let Some(ref accd) = next_start {
                if cdfa.accepts(&state) && state != *accd {
                    Some(state.clone())
                } else {
                    None
                }
            } else {
                None
            };

            let mut last_accepting = ScanOneResult {
                consumed: 0,
                end_state,
                next_start,
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
                                next_start: match res.acceptor_destination {
                                    Some(destination) => Some(destination), // TODO(shane) fix this api!
                                    None => cdfa.acceptor_destination(&next, &state),
                                },
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
            let res: ScanOneResult<State> =
                scan_one(remaining, next_start.clone(), line, character, cdfa);

            next_start = match res.next_start {
                None => next_start,
                Some(state) => state,
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

                        return Err(lex::Error {
                            sequence,
                            line,
                            character,
                        });
                    }

                    break;
                }
                Some(state) => {
                    if let Some(kind) = cdfa.tokenize(&state) {
                        tokens.push(Token::leaf(
                            kind,
                            (&remaining[0..res.consumed]).iter().collect(),
                        ));
                    }
                }
            }

            remaining = &remaining[res.consumed..];
        }

        Ok(tokens)
    }
}
