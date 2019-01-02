use {
    core::{
        data::{
            Data,
            stream::{Stream, StreamSource},
        },
        scan::{
            self,
            CDFA,
            FAIL_SEQUENCE_LENGTH,
            Scanner,
            Token,
        },
    },
    std::collections::LinkedList,
};

pub struct MaximalMunchScanner;

impl<State: Data, Kind: Data> Scanner<State, Kind> for MaximalMunchScanner {
    fn scan<'a, 'b>(
        &self,
        stream_source: &'a mut StreamSource<char>,
        cdfa: &'b CDFA<State, Kind>,
    ) -> Result<Vec<Token<Kind>>, scan::Error> {
        struct ScanOneResult<State> {
            scanned: String,
            end_state: Option<State>,
            next_start: Option<State>,
            line: usize,
            character: usize,
        }

        fn scan_one<State: Data, Kind: Data>(
            stream_source: &mut StreamSource<char>,
            start: State,
            line: usize,
            character: usize,
            cdfa: &CDFA<State, Kind>,
        ) -> ScanOneResult<State> {
            let mut state: State = start;
            let mut line: usize = line;
            let mut character: usize = character;

            let mut last_accepting = ScanOneResult {
                scanned: String::new(),
                end_state: None,
                next_start: None,
                line,
                character,
            };

            let mut consumed: LinkedList<char> = LinkedList::new();

            let mut stream: Stream<char> = stream_source.head();
            stream = stream.split();

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
                                next_start: cdfa.acceptor_destination(&next, &state),
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
                stream = stream.detach_head();
            }
            stream.replay();

            last_accepting
        }

        let mut tokens: Vec<Token<Kind>> = vec![];
        let mut next_start = cdfa.start();
        let mut line: usize = 1;
        let mut character: usize = 1;

        loop {
            let result: ScanOneResult<State> = scan_one(
                stream_source,
                next_start.clone(),
                line,
                character,
                cdfa,
            );

            next_start = match result.next_start {
                None => next_start,
                Some(state) => state
            };

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
                Some(state) => if let Some(kind) = cdfa.tokenize(&state) {
                    let token = Token {
                        kind,
                        lexeme: result.scanned,
                    };


                    println!("SCANNED kind={} lexeme={}", token.kind.to_string(), token.lexeme);
                    //TODO write tokens to another ReadDrivenStream
                    tokens.push(token);
                }
            }
        }

        Ok(tokens)
    }
}
