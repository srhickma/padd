use std::collections::HashSet;
use std::collections::HashMap;
use core::data::stream::ReadDrivenStream;
use core::data::map::CEHashMap;
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
    t_delta: CEHashMap<TransitionTrie>,
    def_delta: CEHashMap<usize>,
    tokenizer: CEHashMap<usize>,
    start: usize,
}

impl CDFA<usize, usize> for EncodedCDFA {
    fn transition(&self, state: &usize, stream: &mut ReadDrivenStream<char>) -> Option<usize> {
        match self.t_delta.get(*state) {
            None => None,
            Some(t_trie) => t_trie.transition(stream)
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

    fn transition(&self, stream: &mut ReadDrivenStream<char>) -> Option<usize> {
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
            return
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
        }
    }

    fn set_default(&mut self, default: usize) {
        //TODO throw error if default is not None (already set)
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


//trait Transition<State> {
//    fn follow(&self, stream: &mut ReadDrivenStream<char>) -> State;
//}
//
//struct SimplexTransition {
//    dest: usize
//}
//
//impl Transition<usize> for SimplexTransition {
//    fn follow(&self, stream: &mut ReadDrivenStream<char>) -> usize {
//        stream.consume();
//        self.dest
//    }
//}
//
//struct ChainTreeTransition {
//
//}
//
//impl Transition<usize> for ChainTreeTransition {
//    fn follow(&self, stream: &mut ReadDrivenStream<char>) -> usize {
//
//    }
//}


//struct PartialStateChain {
//    chain_nfa: ChainNFA
//}
//
//impl PartialStateChain {
//    fn build() {}
//
//    fn begin(&mut self) -> &ChainNFA {
//        self.chain_nfa.cursor = 1;
//        &self.chain_nfa
//    }
//}
//
//struct ChainNFA {
//    chain: Vec<char>,
//    cursor: usize,
//    dest: usize,
//    default: usize,
//}
//
//impl ChainNFA {
//    fn transistion(&mut self, c: char) -> (usize, bool) { //TODO returning a bool is a bit hacky
//        (0, false)
//    }
//}
//
//
//struct StateDelta {
//    c_delta: HashMap<char, usize>,
//    default: Option<usize>,
//}
//
//impl Default for StateDelta {
//    fn default() -> StateDelta { StateDelta::new() }
//}
//
//
//impl StateDelta {
//    fn new() -> StateDelta {
//        return StateDelta {
//            c_delta: HashMap::new(),
//            default: None,
//        };
//    }
//
//    fn transition(&self, c: char) -> Option<usize> {
//        match self.c_delta.get(&c) {
//            None => self.default,
//            Some(dest) => Some(*dest)
//        }
//    }
//
//    fn mark_trans(&mut self, c: char, dest: usize) {
//        self.c_delta.insert(c, dest);
//    }
//
//    fn mark_def(&mut self, dest: usize) {
//        self.default = Some(dest);
//    }
//
//    fn has_trans(&self, c: char) -> bool {
//        self.c_delta.contains_key(&c)
//    }
//
//    fn has_def(&self) -> bool {
//        self.default.is_some()
//    }
//}


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