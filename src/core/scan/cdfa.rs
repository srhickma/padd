use std::collections::HashSet;
use std::collections::HashMap;
use std::error;
use std::fmt;
use std::usize;
use core::data::stream::StreamConsumer;
use core::data::map::CEHashMap;

pub trait CDFABuilder<State, Token> {
    fn new() -> Self;

    fn set_alphabet(&mut self, chars: impl Iterator<Item=char>);
    fn mark_start(&mut self, state: &State);
    fn mark_trans(&mut self, from: &State, to: &State, on: char);
    fn mark_chain(&mut self, from: &State, to: &State, on: impl Iterator<Item=char>);
    fn mark_def(&mut self, from: &State, to: &State);
    fn mark_token(&mut self, state: &State, token: &Token);
}

pub struct EncodedCDFABuilder {
    encoder: HashMap<String, usize>,
    decoder: Vec<String>,

    alphabet: HashedAlphabet,
    accepting: HashSet<usize>,
    t_delta: CEHashMap<TransitionTrie>,
    def_delta: CEHashMap<usize>,
    tokenizer: CEHashMap<usize>,
    start: usize,
}

impl EncodedCDFABuilder {
    fn encode(&mut self, val: &String) -> usize {
        if self.encoder.contains_key(val) {
            *self.encoder.get(val).unwrap()
        } else {
            let key = self.decoder.len();
            self.decoder.push(val.clone());
            self.encoder.insert(val.clone(), key);
            key
        }
    }

    fn decode(&self, key: usize) -> String {
        self.decoder[key].clone()
    }

    fn get_transition_trie(&mut self, from: usize) -> &mut TransitionTrie {
        if !self.t_delta.contains(from) {
            self.t_delta.insert(from, TransitionTrie::new());
        }
        self.t_delta.get_mut(from).unwrap()
    }
}

impl CDFABuilder<String, usize> for EncodedCDFABuilder {
    fn new() -> Self {
        EncodedCDFABuilder {
            encoder: HashMap::new(),
            decoder: Vec::new(),

            alphabet: HashedAlphabet::new(),
            accepting: HashSet::new(),
            t_delta: CEHashMap::new(),
            def_delta: CEHashMap::new(),
            tokenizer: CEHashMap::new(),
            start: usize::max_value()
        }
    }

    fn set_alphabet(&mut self, chars: impl Iterator<Item=char>) {
        chars.for_each(|c| self.alphabet.insert(c));
    }

    fn mark_start(&mut self, state: &String) {
        self.start = self.encode(state);
    }

    fn mark_trans(&mut self, from: &String, to: &String, on: char) {
        let from_encoded = self.encode(from);
        let to_encoded = self.encode(to);

        let t_trie = self.get_transition_trie(from_encoded);

        t_trie.insert(on, to_encoded);
    }

    fn mark_chain(&mut self, from: &String, to: &String, on: impl Iterator<Item=char>) {
        let from_encoded = self.encode(from);
        let to_encoded = self.encode(to);

        let t_trie = self.get_transition_trie(from_encoded);

        let mut chars: Vec<char> = Vec::new();
        on.for_each(|c| chars.push(c));
        t_trie.insert_chain(&chars, to_encoded);
    }

    fn mark_def(&mut self, from: &String, to: &String) {
        let from_encoded = self.encode(from);
        let to_encoded = self.encode(to);
        self.def_delta.insert(from_encoded, to_encoded)
    }

    fn mark_token(&mut self, state: &String, token: &usize) {
        let state_encoded = self.encode(state);
        self.tokenizer.insert(state_encoded, *token)
    }
}

pub trait CDFA<State, Token> {
    fn transition(&self, state: &State, stream: &mut StreamConsumer<char>) -> Option<State>;
    fn has_transition(&self, state: &State, stream: &mut StreamConsumer<char>) -> bool;
    fn accepts(&self, state: &State) -> bool;
    fn tokenize(&self, state: &State) -> Option<Token>;
    fn start(&self) -> State;
}

pub struct EncodedCDFA {
    alphabet: HashedAlphabet,
    accepting: HashSet<usize>,
    t_delta: CEHashMap<TransitionTrie>,
    def_delta: CEHashMap<usize>,
    tokenizer: CEHashMap<usize>,
    start: usize,
}

impl EncodedCDFA {
    fn build_from(builder: EncodedCDFABuilder) -> Result<Self, String> {
        if builder.start == usize::max_value() {
            Err("No start state was set".to_string())
        } else if builder.start > builder.t_delta.size() {
            Err("Invalid start state".to_string())
        } else {
            Ok(EncodedCDFA {
                alphabet: builder.alphabet,
                accepting: builder.accepting,
                t_delta: builder.t_delta,
                def_delta: builder.def_delta,
                tokenizer: builder.tokenizer,
                start: builder.start
            })
        }
    }
}

impl CDFA<usize, usize> for EncodedCDFA {
    fn transition(&self, state: &usize, stream: &mut StreamConsumer<char>) -> Option<usize> {
        match self.t_delta.get(*state) {
            None => None,
            Some(t_trie) => t_trie.transition(stream)
        }
    }

    fn has_transition(&self, state: &usize, stream: &mut StreamConsumer<char>) -> bool {
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


struct TransitionTrie {
    root: TransitionNode,
    default: Option<usize>,
}

impl TransitionTrie {
    fn new() -> TransitionTrie {
        TransitionTrie {
            root: TransitionNode {
                children: HashMap::new(),
                dest: 0,
            },
            default: None,
        }
    }

    fn transition(&self, stream: &mut StreamConsumer<char>) -> Option<usize> {
        let mut curr: &TransitionNode = &self.root;
        while !curr.leaf() {
            curr = match stream.pull() {
                None => return None,
                Some(c) => match curr.get_child(c) {
                    None => match self.default {
                        None => return None,
                        Some(state) => {
                            stream.replay().advance().consume();
                            return Some(state);
                        }
                    }
                    Some(child) => child
                }
            }
        }

        stream.consume();
        Some(curr.dest)
    }

    fn insert(&mut self, c: char, dest: usize) {
        TransitionTrie::insert_internal(c, &mut self.root, true, dest);
    }

    fn insert_chain(&mut self, chars: &Vec<char>, dest: usize) {
        TransitionTrie::insert_chain_internal(0, &mut self.root, chars, dest);
    }

    fn insert_chain_internal(i: usize, node: &mut TransitionNode, chars: &Vec<char>, dest: usize) {
        if i == chars.len() {
            return;
        }

        let c = chars[i];
        TransitionTrie::insert_internal(c, node, i == chars.len() - 1, dest);
        TransitionTrie::insert_chain_internal(i + 1, node.get_child_mut(c).unwrap(), chars, dest);
    }

    fn insert_internal(c: char, node: &mut TransitionNode, last: bool, dest: usize) {
        if !node.has_child(c) {
            let child = TransitionNode {
                children: HashMap::new(),
                dest: if last { dest } else { 0 },
            };
            node.add_child(c, child);
        } else if last {
            //TODO throw error here (trie is not prefix free)
            panic!("Trie is not prefix free");
        }
    }

    fn set_default(&mut self, default: usize) {
        if self.default.is_some() {
            //TODO throw error if default is not None (already set)
            panic!("default transition set twice");
        }
        self.default = Some(default);
    }
}

impl Default for TransitionTrie {
    fn default() -> TransitionTrie { TransitionTrie::new() }
}

struct TransitionNode {
    children: HashMap<char, TransitionNode>,
    dest: usize,
}

impl TransitionNode {
    fn leaf(&self) -> bool {
        self.children.is_empty()
    }

    fn get_child(&self, c: char) -> Option<&TransitionNode> {
        self.children.get(&c)
    }

    fn get_child_mut(&mut self, c: char) -> Option<&mut TransitionNode> {
        self.children.get_mut(&c)
    }

    fn has_child(&self, c: char) -> bool {
        self.children.contains_key(&c)
    }

    fn add_child(&mut self, c: char, child: TransitionNode) {
        self.children.insert(c, child);
    }
}

trait Alphabet {
    fn contains(&self, c: char) -> bool;
}

struct HashedAlphabet {
    alphabet: HashSet<char>
}

impl HashedAlphabet {
    fn new() -> HashedAlphabet {
        HashedAlphabet {
            alphabet: HashSet::new()
        }
    }

    fn insert(&mut self, c: char) {
        self.alphabet.insert(c);
    }
}

impl Alphabet for HashedAlphabet {
    fn contains(&self, c: char) -> bool {
        self.alphabet.contains(&c)
    }
}
