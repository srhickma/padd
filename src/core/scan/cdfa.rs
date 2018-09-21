use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::LinkedList;
use std::collections::linked_list::Iter;
use std::iter::Peekable;
use std::error;
use std::fmt;

pub trait CDFA<State, Token> {
    fn transition(&self, state: &State, stream: &mut ReadDrivenStream<char>) -> Option<State>;
    fn has_transition(&self, state: &State, stream: &mut ReadDrivenStream<char>) -> bool;
    fn accepts(&self, state: &State) -> bool;
    fn tokenize(&self, state: &State) -> Option<Token>;
    fn start(&self) -> State;
}

pub struct EncodedCDFA {
    alphabet: HashedAlphabet,
    accepting: HashSet<usize>,
    t_delta: CEHashMap<StateDelta>,
    def_delta: CEHashMap<usize>,
    tokenizer: CEHashMap<usize>,
    start: usize,
}

impl CDFA<usize, usize> for EncodedCDFA {
    fn transition(&self, state: &usize, stream: &mut ReadDrivenStream<char>) -> Option<usize> {
        //TODO need to account for partial states here
        match self.t_delta.get(*state) {
            None => None,
            Some(state_delta) => {
                let res: Option<usize> = state_delta.transition(stream.pull().unwrap());
                stream.consume();
                res
            }
        }
    }

    fn has_transition(&self, state: &usize, stream: &mut ReadDrivenStream<char>) -> bool {
        stream.block();
        let res = self.transition(state, stream).is_some();
        stream.unblock();
        res
    }

    fn accepts(&self, state: &usize) -> bool {
        self.accepting.contains(state)
    }

    fn tokenize(&self, state: &usize) -> Option<usize> {
        match self.tokenizer.get(*state) {
            None => None,
            Some(dest) => Some(*dest)
        }
    }

    fn start(&self) -> usize {
        self.start
    }
}


struct PartialStateChain {
    chain_nfa: ChainNFA
}

impl PartialStateChain {
    fn build() {}

    fn begin(&mut self) -> &ChainNFA {
        self.chain_nfa.cursor = 1;
        &self.chain_nfa
    }
}

struct ChainNFA {
    chain: Vec<char>,
    cursor: usize,
    dest: usize,
    default: usize,
}

impl ChainNFA {
    fn transistion(&mut self, c: char) -> (usize, bool) { //TODO returning a bool is a bit hacky
        //if self.chain
        //if self.chain.get(cursor)
        (0, false)
    }
}


struct StateDelta {
    c_delta: HashMap<char, usize>,
    default: Option<usize>,
}

impl Default for StateDelta {
    fn default() -> StateDelta { StateDelta::new() }
}


impl StateDelta {
    fn new() -> StateDelta {
        return StateDelta {
            c_delta: HashMap::new(),
            default: None,
        };
    }

    fn transition(&self, c: char) -> Option<usize> {
        match self.c_delta.get(&c) {
            None => self.default,
            Some(dest) => Some(*dest)
        }
    }

    fn mark_trans(&mut self, c: char, dest: usize) {
        self.c_delta.insert(c, dest);
    }

    fn mark_def(&mut self, dest: usize) {
        self.default = Some(dest);
    }

    fn has_trans(&self, c: char) -> bool {
        self.c_delta.contains_key(&c)
    }

    fn has_def(&self) -> bool {
        self.default.is_some()
    }
}


pub trait CDFABuilder<State, Token, Impl: CDFA<State, Token>> {
    fn new() -> Self;
    fn build(&self) -> Result<Impl, String>; //TODO create custom error type

    fn mark_start(&self, state: &State);
    fn mark_trans(&self, from: &State, to: &State, on: char);
    fn mark_chain(&self, from: &State, to: &State, on: impl Iterator<Item=char>);
    fn mark_def(&self, from: &State, to: &State);
    fn mark_token(&self, state: &State, token: &Token);
}


trait Alphabet {
    fn contains(&self, c: char) -> bool;
}

struct HashedAlphabet {
    alphabet: HashSet<char>
}

impl Alphabet for HashedAlphabet {
    fn contains(&self, c: char) -> bool {
        self.alphabet.contains(&c)
    }
}


//Continuously encoded hash map
struct CEHashMap<V: Default> {
    vector: Vec<V>
}

impl<V: Default> CEHashMap<V> {
    fn insert(&mut self, key: usize, value: V) {
        while self.vector.len() <= key {
            self.vector.push(V::default());
        }
        self.vector.insert(key, value)
    }

    fn get(&self, key: usize) -> Option<&V> {
        self.vector.get(key)
    }

    fn contains(&self, key: usize) -> bool {
        self.get(key).is_some()
    }
}


//TODO separate into its own data type file with tests
pub struct ReadDrivenStream<T: Clone> {
    incoming_buffer: LinkedList<T>,
    outgoing_buffer: LinkedList<T>,
    getter: fn() -> Option<T>,
    block: bool,
}

impl<T: Clone> ReadDrivenStream<T> {
    fn observe(getter: fn() -> Option<T>) -> ReadDrivenStream<T> {
        ReadDrivenStream {
            incoming_buffer: LinkedList::new(),
            outgoing_buffer: LinkedList::new(),
            getter,
            block: false,
        }
    }

    fn pull(&mut self) -> Option<T> {
        let val: T = match self.incoming_buffer.pop_back() {
            None => match (self.getter)() {
                None => { return None; }
                Some(val) => val
            },
            Some(val) => val
        };

        self.outgoing_buffer.push_back(val.clone());

        Some(val)
    }

    fn replay(&mut self) {
        let mut val: Option<T> = self.outgoing_buffer.pop_front();
        while val.is_some() {
            self.incoming_buffer.push_back(val.unwrap());
            val = self.outgoing_buffer.pop_front();
        }
    }

    fn consume(&mut self) {
        if self.block {
            self.replay();
            self.block = false;
        } else {
            self.outgoing_buffer.clear();
        }
    }

    fn block(&mut self) {
        self.block = true;
    }

    fn unblock(&mut self) {
        self.block = false;
    }
}