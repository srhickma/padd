use core::{
    data::Data,
    lex::{self, Lexer, Token, TransitionResult, CDFA, FAIL_SEQUENCE_LENGTH},
    parse::grammar::GrammarSymbol,
};

pub struct LongestMatchLexer;

impl<State: Data, Symbol: GrammarSymbol> Lexer<State, Symbol> for LongestMatchLexer {
    fn lex<'cdfa>(
        &self,
        input: &[char],
        cdfa: &'cdfa dyn CDFA<State, Symbol>,
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
            cdfa: &dyn CDFA<State, Symbol>,
        ) -> Result<ScanOneResult<State>, lex::Error> {
            let mut remaining = input;
            let mut state: State = start;
            let mut line: usize = line;
            let mut character: usize = character;

            let next_start = cdfa.default_acceptor_destination(&state);

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

                match res {
                    TransitionResult::Fail => break,
                    TransitionResult::Ok(dest) => {
                        consumed += dest.consumed;

                        for c in remaining.iter().take(dest.consumed) {
                            character += 1;
                            if *c == '\n' {
                                line += 1;
                                character = 1;
                            }

                            if !cdfa.alphabet_contains(*c) {
                                return Err(lex::Error::AlphabetErr(*c));
                            }
                        }

                        if cdfa.accepts(&dest.state) {
                            last_accepting = ScanOneResult {
                                consumed,
                                end_state: Some(dest.state.clone()),
                                next_start: match dest.acceptor_destination {
                                    Some(destination) => Some(destination),
                                    None => cdfa.default_acceptor_destination(&dest.state),
                                },
                                line,
                                character,
                            };
                        }

                        state = dest.state;
                        remaining = &remaining[dest.consumed..];
                    }
                }
            }

            Ok(last_accepting)
        }

        let mut remaining = input;
        let mut tokens: Vec<Token<Symbol>> = vec![];
        let mut next_start = cdfa.start();
        let mut line: usize = 1;
        let mut character: usize = 1;

        loop {
            let res: ScanOneResult<State> =
                scan_one(remaining, next_start.clone(), line, character, cdfa)?;

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

                        return Err(lex::Error::from(lex::UnacceptedError {
                            sequence,
                            line,
                            character,
                        }));
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
