use core::{
    data::Data,
    lex::{self, Lexer, Token, TransitionResult, CDFA, FAIL_SEQUENCE_LENGTH},
    parse::grammar::GrammarSymbol,
};

/// Longest Match Lexer: Lexer which greedily consumes input, producing the longest possible tokens.
pub struct LongestMatchLexer;

impl<State: Data, Symbol: GrammarSymbol> Lexer<State, Symbol> for LongestMatchLexer {
    fn lex<'cdfa>(
        &self,
        input: &[char],
        cdfa: &'cdfa dyn CDFA<State, Symbol>,
    ) -> Result<Vec<Token<Symbol>>, lex::Error> {
        /// Scan-One Result: The result of scanning a single token.
        ///
        /// # Type Parameters
        ///
        /// * `State` - the state type of the CDFA being used.
        ///
        /// # Fields
        ///
        /// * `consumed` - the number of input characters consumed by the lex.
        /// * `end_state` - the accepted CDFA state after completing the lex.
        /// * `next_start` - the CDFA state to start the next lex from.
        /// * `line` - the current line number after the lex.
        /// * `characters` - the current character number after the lex.
        struct ScanOneResult<State> {
            consumed: usize,
            end_state: Option<State>,
            next_start: Option<State>,
            line: usize,
            character: usize,
        }

        /// Scan a single token from the head of `input`. Lexing is performed by iteratively reading
        /// input and traversing the passed CDFA. Once the input is exhausted or no transition
        /// exists, the input up to the most recently accepting state in the CDFA is consumed.
        ///
        /// Returns an error if the scan fails, or a `ScanOneResult` containing the details of the
        /// scanned token.
        ///
        /// # Type Parameters:
        ///
        /// * `State` - the state type of the CDFA being used.
        /// * `Symbol` - the type of grammar symbol being tokenized into.
        ///
        /// # Parameters
        ///
        /// * `input` - a slice of the input array being scanned, where the start of the slice is
        /// the current lex cursor.
        /// * `start` - the starting CDFA state in which to begin the lex.
        /// * `line` - the current line number.
        /// * `character` - the current character number (on the current line).
        /// * `cdfa` - the CDFA to use when lexing the input.
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

            // If the start state has an acceptor destination, remember it.
            let end_state = if let Some(ref accd) = next_start {
                if cdfa.accepts(&state) && state != *accd {
                    Some(state.clone())
                } else {
                    None
                }
            } else {
                None
            };

            let mut consumed: usize = 0;

            let mut last_accepting = ScanOneResult {
                consumed,
                end_state,
                next_start,
                line,
                character,
            };

            loop {
                // Take a transition on the remaining input.
                let res = cdfa.transition(&state, remaining);

                match res {
                    TransitionResult::Fail => break,
                    TransitionResult::Ok(dest) => {
                        consumed += dest.consumed;

                        for c in remaining.iter().take(dest.consumed) {
                            // Update calculation of current character and line.
                            character += 1;
                            if *c == '\n' {
                                line += 1;
                                character = 1;
                            }

                            // Error out if we see an unexpected character.
                            if !cdfa.alphabet_contains(*c) {
                                return Err(lex::Error::AlphabetErr(*c));
                            }
                        }

                        // If the current state is accepting, remember it.
                        // This avoids backtracking when we reach the end of the lex.
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
            // Scan a single token.
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
                    // If more input remains after a failed token scan, return a lexing error.
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
                    // Scanning succeeded, tokenize the consumed input and continue.
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
