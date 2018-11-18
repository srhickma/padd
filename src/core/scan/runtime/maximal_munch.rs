use std::collections::LinkedList;

use core::data::Data;
use core::data::stream::Stream;
use core::data::stream::StreamSource;
use core::scan;
use core::scan::FAIL_SEQUENCE_LENGTH;
use core::scan::runtime::CDFA;
use core::scan::runtime::Scanner;
use core::scan::Token;

pub struct MaximalMunchScanner;

impl<State: Data, Kind: Data> Scanner<State, Kind> for MaximalMunchScanner {
    fn scan<'a, 'b>(&self, stream_source: &'a mut StreamSource<char>, cdfa: &'b CDFA<State, Kind>) -> Result<Vec<Token<Kind>>, scan::Error> {
        struct ScanOneResult<State> {
            scanned: String,
            end_state: Option<State>,
            line: usize,
            character: usize,
        }

        fn scan_one<State: Data, Kind: Data>(stream_source: &mut StreamSource<char>, line: usize, character: usize, cdfa: &CDFA<State, Kind>) -> ScanOneResult<State> {
            let mut state: State = cdfa.start();
            let mut line: usize = line;
            let mut character: usize = character;

            let mut last_accepting = ScanOneResult {
                scanned: String::new(),
                end_state: None,
                line,
                character,
            };

            let mut consumed: LinkedList<char> = LinkedList::new();

            let mut stream: Stream<char> = stream_source.head();

            loop {
                let res: Option<State> = {
                    let mut consumer = stream
                        .consumer(Box::new(|list: &LinkedList<char>| {
                            for c in list {
                                character += 1;
                                if *c == '\n' {
                                    line += 1;
                                    character = 1;
                                }

                                consumed.push_back(*c)
                            }
                        }));

                    cdfa.transition(&state, &mut consumer)
                };

                match res {
                    None => break,
                    Some(next) => {
                        if cdfa.accepts(&next) {
                            last_accepting = ScanOneResult {
                                scanned: consumed.iter().collect(),
                                end_state: Some(next.clone()),
                                line,
                                character,
                            };
                            stream.detach_tail();
                            stream = stream.split();
                        }
                        state = next;
                    }
                }
            }

            if stream.has_tail() {
                stream.detach_head();
            }
            stream.replay();

            last_accepting
        }

        let mut tokens: Vec<Token<Kind>> = vec![];
        let mut line: usize = 1;
        let mut character: usize = 1;

        loop {
            let result: ScanOneResult<State> = scan_one(stream_source, line, character, cdfa);

            line = result.line;
            character = result.character;

            match result.end_state {
                None => {
                    let mut stream = stream_source.head();

                    if stream.has_next() {
                        let sequence: String = (0..FAIL_SEQUENCE_LENGTH)
                            .map(|_| stream.pull())
                            .filter(|opt| opt.is_some())
                            .map(|opt| opt.unwrap())
                            .collect();

                        return Err(scan::Error {
                            sequence,
                            line,
                            character,
                        });
                    } else {
                        break;
                    }
                }
                Some(state) => match cdfa.tokenize(&state) {
                    None => {}
                    Some(kind) => {
                        let token = Token {
                            kind,
                            lexeme: result.scanned,
                        };

                        //TODO write tokens to another ReadDrivenStream
                        tokens.push(token);
                    }
                }
            }
        }

        Ok(tokens)
    }
}
