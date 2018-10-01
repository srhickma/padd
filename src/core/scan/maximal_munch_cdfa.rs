use std::collections::LinkedList;
use std::cmp;
use core::spec::DEF_MATCHER;
use core::scan;
use core::scan::Token;
use core::scan::FAIL_SEQUENCE_LENGTH;
use core::scan::cdfa::CDFA;
use core::data::stream::Stream;
use core::data::stream::StreamSource;
use core::data::stream::StreamConsumer;

//TODO use generic token type?

pub trait Scanner<State: PartialEq + Clone> {
    fn scan<'a, 'b>(&self, stream: &'a mut StreamSource<char>, cdfa: &'b CDFA<State, String>) -> Result<Vec<Token>, scan::Error>;
}

pub struct MaximalMunchScanner;

impl<State: PartialEq + Clone> Scanner<State> for MaximalMunchScanner {
    fn scan<'a, 'b>(&self, stream_source: &'a mut StreamSource<char>, cdfa: &'b CDFA<State, String>) -> Result<Vec<Token>, scan::Error> {
        fn scan_one<State: PartialEq + Clone>(stream: &mut Stream<char>, line: usize, character: usize, cdfa: &CDFA<State, String>) -> (String, Option<State>, usize, usize) {

            let mut state: State = cdfa.start();
            let mut line: usize = line;
            let mut character: usize = character;

            let mut last_accepting: (String, Option<State>, usize, usize) = (String::new(), None, line, character);

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
                                consumed.push_back(*c)
                            }
                        }));

                    cdfa.transition(&state, &mut consumer)
                };

                match res {
                    None => {
                        break;
                    }
                    Some(next) => {
                        if cdfa.accepts(&next) {
                            last_accepting = (consumed.iter().collect(), Some(next.clone()), line, character);
                            stream.detach_front();
                            stream.split();
                        }
                        state = next;
                    }
                }
            }

            last_accepting
        }

        let mut stream: Stream<char> = stream_source.split();

        let mut tokens: Vec<Token> = vec![];
        let mut line: usize = 1;
        let mut character: usize = 1;

        loop {
            let (scanned, end_state, end_line, end_character) = scan_one(&mut stream, line, character, cdfa);

            stream = match stream.detach_back() {
                None => stream.split(),
                Some(wrapped_stream) => wrapped_stream
            };


            line = end_line;
            character = end_character;

            match end_state {
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
                    let accept_as = cdfa.tokenize(&state).unwrap();
                    if accept_as != DEF_MATCHER.to_string() { //TODO get rid of this hacky crap
                        let token = Token {
                            kind: accept_as,
                            lexeme: scanned,
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
