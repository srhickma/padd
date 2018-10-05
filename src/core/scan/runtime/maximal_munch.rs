use std::collections::LinkedList;
use core::scan;
use core::scan::FAIL_SEQUENCE_LENGTH;
use core::scan::runtime::CDFA;
use core::data::Data;
use core::data::stream::Stream;
use core::data::stream::StreamSource;
use core::scan::runtime::Scanner;
use core::scan::runtime::Token;

pub struct MaximalMunchScanner;

impl<State: Data, Kind: Data> Scanner<State, Kind> for MaximalMunchScanner {
    fn scan<'a, 'b>(&self, stream_source: &'a mut StreamSource<char>, cdfa: &'b CDFA<State, Kind>) -> Result<Vec<Token<Kind>>, scan::Error> {

        struct ScanOneResult<State> {
            scanned: String,
            end_state: Option<State>,
            line: usize,
            character: usize
        }

        fn scan_one<State: Data, Kind: Data>(stream: &mut Stream<char>, line: usize, character: usize, cdfa: &CDFA<State, Kind>) -> ScanOneResult<State> {
            let mut state: State = cdfa.start();
            let mut line: usize = line;
            let mut character: usize = character;

            let mut last_accepting = ScanOneResult {
                scanned: String::new(),
                end_state: None,
                line,
                character
            };

            let mut consumed: LinkedList<char> = LinkedList::new();

            loop {
                let res: Option<State> = {
                    let mut consumer = stream
                        .consumer(Box::new(| list: &LinkedList<char> | {
                            for c in list {
                                character += 1;
                                if *c == '\n' {
                                    line += 1;
                                    character = 1;
                                }

                                println!("CONSUMED {}", c);

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
                                character
                            };
                            stream.detach_tail();
                            stream.split();
                        }
                        state = next;
                    }
                }
            }

            last_accepting
        }

        let mut stream: Stream<char> = stream_source.split();

        let mut tokens: Vec<Token<Kind>> = vec![];
        let mut line: usize = 1;
        let mut character: usize = 1;

        loop {
            let result: ScanOneResult<State> = scan_one(&mut stream, line, character, cdfa);

            stream = match stream.detach_head() {
                None => stream.split(),
                Some(wrapped_stream) => wrapped_stream
            };

            line = result.line;
            character = result.character;

            match result.end_state {
                None => if stream.has_next() {
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
                },
                Some(state) => {
                    let kind = cdfa.tokenize(&state).unwrap();
                    let token = Token {
                        kind,
                        lexeme: result.scanned,
                    };

                    println!("SCANNED {}", &token.to_string());

                    //TODO write tokens to another ReadDrivenStream
                    tokens.push(token);
                }
            }
        }

        Ok(tokens)
    }
}
