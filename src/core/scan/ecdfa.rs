use {
    core::{
        data::{
            map::{CEHashMap, CEHashMapIterator},
            stream::StreamConsumer,
        },
        scan::{
            alphabet::HashedAlphabet,
            CDFA,
            CDFABuilder,
            CDFAError,
        },
    },
    std::{
        collections::{HashMap, HashSet},
        hash::Hash,
        usize,
    },
};

pub struct EncodedCDFABuilder<State: Eq + Hash + Clone, Kind: Default + Clone> {
    encoder: HashMap<State, usize>,
    decoder: Vec<State>,
    alphabet_str: String,

    alphabet: HashedAlphabet,
    accepting: HashSet<usize>,
    t_delta: CEHashMap<TransitionTrie>,
    tokenizer: CEHashMap<Kind>,
    start: usize,
}

impl<State: Eq + Hash + Clone, Kind: Default + Clone> EncodedCDFABuilder<State, Kind> {
    fn encode(&mut self, val: &State) -> usize {
        if self.encoder.contains_key(val) {
            *self.encoder.get(val).unwrap()
        } else {
            let key = self.decoder.len();
            self.decoder.push(val.clone());
            self.encoder.insert(val.clone(), key);
            key
        }
    }

    fn get_transition_trie(&mut self, from: usize) -> &mut TransitionTrie {
        if !self.t_delta.contains(from) {
            self.t_delta.insert(from, TransitionTrie::new());
        }
        self.t_delta.get_mut(from).unwrap()
    }

    fn get_alphabet_range(&self, start: char, end: char) -> Vec<char> {
        let mut in_range = false;

        self.alphabet_str.chars()
            .filter(|c| {
                if *c == start {
                    in_range = true;
                }
                if *c == end {
                    in_range = false;
                    return true;
                }
                in_range
            })
            .collect()
    }
}

impl<State: Eq + Hash + Clone, Kind: Default + Clone> CDFABuilder<State, Kind, EncodedCDFA<Kind>>
for EncodedCDFABuilder<State, Kind> {
    fn new() -> Self {
        EncodedCDFABuilder {
            encoder: HashMap::new(),
            decoder: Vec::new(),
            alphabet_str: String::new(),

            alphabet: HashedAlphabet::new(),
            accepting: HashSet::new(),
            t_delta: CEHashMap::new(),
            tokenizer: CEHashMap::new(),
            start: usize::max_value(),
        }
    }

    fn build(self) -> Result<EncodedCDFA<Kind>, CDFAError> {
        if self.start == usize::max_value() {
            Err(CDFAError::BuildErr("No start state was set".to_string()))
        } else if self.start > self.t_delta.size() {
            Err(CDFAError::BuildErr("Invalid start state".to_string()))
        } else {
            Ok(EncodedCDFA {
                alphabet: self.alphabet,
                accepting: self.accepting,
                t_delta: self.t_delta,
                tokenizer: self.tokenizer,
                start: self.start,
            })
        }
    }

    fn set_alphabet(&mut self, chars: impl Iterator<Item=char>) -> &mut Self {
        chars.for_each(|c| {
            self.alphabet_str.push(c);
            self.alphabet.insert(c);
        });
        self
    }

    fn mark_accepting(&mut self, state: &State) -> &mut Self {
        let state_encoded = self.encode(state);
        self.accepting.insert(state_encoded);
        self
    }

    fn mark_start(&mut self, state: &State) -> &mut Self {
        self.start = self.encode(state);
        self
    }

    fn mark_trans(
        &mut self,
        from: &State,
        to: &State,
        on: char,
    ) -> Result<&mut Self, CDFAError> {
        let from_encoded = self.encode(from);
        let to_encoded = self.encode(to);

        {
            let t_trie = self.get_transition_trie(from_encoded);
            t_trie.insert(on, to_encoded)?;
        }

        Ok(self)
    }

    fn mark_chain(
        &mut self,
        from: &State,
        to: &State,
        on: impl Iterator<Item=char>,
    ) -> Result<&mut Self, CDFAError> {
        let from_encoded = self.encode(from);
        let to_encoded = self.encode(to);

        {
            let t_trie = self.get_transition_trie(from_encoded);

            let mut chars: Vec<char> = Vec::new();
            on.for_each(|c| chars.push(c));
            t_trie.insert_chain(&chars, to_encoded)?;
        }

        Ok(self)
    }

    fn mark_range(
        &mut self,
        from: &State,
        to: &State,
        start: char,
        end: char,
    ) -> Result<&mut Self, CDFAError> {
        let to_mark = self.get_alphabet_range(start, end);

        for c in &to_mark {
            self.mark_trans(from, to, *c)?;
        }

        Ok(self)
    }

    fn mark_range_for_all<'state_o: 'state_i, 'state_i>(
        &mut self,
        sources: impl Iterator<Item=&'state_i &'state_o State>,
        to: &'state_o State,
        start: char,
        end: char,
    ) -> Result<&mut Self, CDFAError> {
        let to_mark = self.get_alphabet_range(start, end);

        for source in sources {
            for c in &to_mark {
                self.mark_trans(&source, to, *c)?;
            }
        }

        Ok(self)
    }

    fn mark_def(&mut self, from: &State, to: &State) -> Result<&mut Self, CDFAError> {
        let from_encoded = self.encode(from);
        let to_encoded = self.encode(to);

        match {
            let t_trie = self.get_transition_trie(from_encoded);

            t_trie.set_default(to_encoded)
        } {
            Err(err) => Err(err),
            Ok(()) => Ok(self)
        }
    }

    fn mark_token(&mut self, state: &State, token: &Kind) -> &mut Self {
        self.mark_accepting(state);
        let state_encoded = self.encode(state);
        self.tokenizer.insert(state_encoded, token.clone());
        self
    }
}

pub struct EncodedCDFA<Kind: Default + Clone> {
    //TODO add separate error message if character not in alphabet
    #[allow(dead_code)]
    alphabet: HashedAlphabet,
    accepting: HashSet<usize>,
    t_delta: CEHashMap<TransitionTrie>,
    tokenizer: CEHashMap<Kind>,
    start: usize,
}

impl<Kind: Default + Clone> EncodedCDFA<Kind> {
    pub fn produces(&self) -> CEHashMapIterator<Kind> {
        self.tokenizer.iter()
    }
}

impl<Kind: Default + Clone> CDFA<usize, Kind> for EncodedCDFA<Kind> {
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

    fn tokenize(&self, state: &usize) -> Option<Kind> {
        match self.tokenizer.get(*state) {
            None => None,
            Some(dest) => Some(dest.clone())
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
                dest: usize::max_value(),
            },
            default: None,
        }
    }

    fn transition(&self, stream: &mut StreamConsumer<char>) -> Option<usize> {
        let mut curr: &TransitionNode = &self.root;

        if curr.children.is_empty() {
            match self.default {
                None => None,
                Some(state) => {
                    match stream.pull() {
                        None => None,
                        Some(_) => {
                            stream.consume();
                            Some(state)
                        }
                    }
                }
            }
        } else {
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
    }

    fn insert(&mut self, c: char, dest: usize) -> Result<(), CDFAError> {
        TransitionTrie::insert_internal(c, &mut self.root, true, dest)
    }

    fn insert_chain(&mut self, chars: &Vec<char>, dest: usize) -> Result<(), CDFAError> {
        TransitionTrie::insert_chain_internal(0, &mut self.root, chars, dest)
    }

    fn insert_chain_internal(
        i: usize,
        node: &mut TransitionNode,
        chars: &Vec<char>,
        dest: usize,
    ) -> Result<(), CDFAError> {
        if i == chars.len() {
            return Ok(());
        }

        let c = chars[i];
        TransitionTrie::insert_internal(c, node, i == chars.len() - 1, dest)?;
        TransitionTrie::insert_chain_internal(i + 1, node.get_child_mut(c).unwrap(), chars, dest)
    }

    fn insert_internal(
        c: char,
        node: &mut TransitionNode,
        last: bool,
        dest: usize,
    ) -> Result<(), CDFAError> {
        if !node.has_child(c) {
            let child = TransitionNode {
                children: HashMap::new(),
                dest: if last { dest } else { usize::max_value() },
            };
            node.add_child(c, child);
        } else if last {
            return Err(CDFAError::BuildErr(format!(
                "Transition trie is not prefix free on character '{}'", c
            )));
        }
        Ok(())
    }

    fn set_default(&mut self, default: usize) -> Result<(), CDFAError> {
        if self.default.is_some() {
            Err(CDFAError::BuildErr("Default matcher used twice".to_string()))
        } else {
            self.default = Some(default);
            Ok(())
        }
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

#[cfg(test)]
mod tests {
    use core::{
        data::{
            Data,
            stream::StreamSource,
        },
        scan::{self, Token},
    };

    use super::*;

    #[test]
    fn scan_binary() {
        //setup
        let mut builder: EncodedCDFABuilder<String, String> = EncodedCDFABuilder::new();
        builder
            .set_alphabet("01".chars());
        builder
            .mark_trans(&"start".to_string(), &"zero".to_string(), '0').unwrap()
            .mark_trans(&"start".to_string(), &"notzero".to_string(), '1').unwrap();
        builder
            .mark_def(&"notzero".to_string(), &"notzero".to_string()).unwrap();
        builder
            .mark_token(&"zero".to_string(), &"ZERO".to_string())
            .mark_token(&"notzero".to_string(), &"NZ".to_string());
        builder
            .mark_start(&"start".to_string());
        builder
            .mark_accepting(&"zero".to_string())
            .mark_accepting(&"notzero".to_string());

        let cdfa: EncodedCDFA<String> = builder.build().unwrap();

        let input = "000011010101".to_string();
        let mut iter = input.chars();

        let mut getter = || iter.next();

        let mut stream: StreamSource<char> = StreamSource::observe(&mut getter);

        let scanner = scan::def_scanner();

        //exercise
        let tokens = scanner.scan(&mut stream, &cdfa).unwrap();

        //verify
        assert_eq!(tokens_string(&tokens), "\
ZERO <- '0'
ZERO <- '0'
ZERO <- '0'
ZERO <- '0'
NZ <- '11010101'
");
    }

    #[test]
    fn scan_brackets() {
        //setup
        let mut builder: EncodedCDFABuilder<String, String> = EncodedCDFABuilder::new();
        builder
            .set_alphabet("{} \t\n".chars());
        builder
            .mark_trans(&"start".to_string(), &"ws".to_string(), ' ').unwrap()
            .mark_trans(&"start".to_string(), &"ws".to_string(), '\t').unwrap()
            .mark_trans(&"start".to_string(), &"ws".to_string(), '\n').unwrap()
            .mark_trans(&"start".to_string(), &"lbr".to_string(), '{').unwrap()
            .mark_trans(&"start".to_string(), &"rbr".to_string(), '}').unwrap()
            .mark_trans(&"ws".to_string(), &"ws".to_string(), ' ').unwrap()
            .mark_trans(&"ws".to_string(), &"ws".to_string(), '\t').unwrap()
            .mark_trans(&"ws".to_string(), &"ws".to_string(), '\n').unwrap();
        builder
            .mark_token(&"lbr".to_string(), &"LBR".to_string())
            .mark_token(&"rbr".to_string(), &"RBR".to_string())
            .mark_token(&"ws".to_string(), &"WS".to_string());
        builder
            .mark_start(&"start".to_string());
        builder
            .mark_accepting(&"lbr".to_string())
            .mark_accepting(&"rbr".to_string())
            .mark_accepting(&"ws".to_string());

        let cdfa: EncodedCDFA<String> = builder.build().unwrap();

        let input = "  {{\n}{}{} \t{} \t{}}".to_string();
        let mut iter = input.chars();

        let mut getter = || iter.next();

        let mut stream: StreamSource<char> = StreamSource::observe(&mut getter);

        let scanner = scan::def_scanner();

        //exercise
        let tokens = scanner.scan(&mut stream, &cdfa).unwrap();

        //verify
        assert_eq!(tokens_string(&tokens), "\
WS <- '  '
LBR <- '{'
LBR <- '{'
WS <- '\\n'
RBR <- '}'
LBR <- '{'
RBR <- '}'
LBR <- '{'
RBR <- '}'
WS <- ' \\t'
LBR <- '{'
RBR <- '}'
WS <- ' \\t'
LBR <- '{'
RBR <- '}'
RBR <- '}'
");
    }

    #[test]
    fn scan_ignore() {
        //setup
        let mut builder: EncodedCDFABuilder<String, String> = EncodedCDFABuilder::new();
        builder
            .set_alphabet("{} \t\n".chars());
        builder
            .mark_trans(&"start".to_string(), &"ws".to_string(), ' ').unwrap()
            .mark_trans(&"start".to_string(), &"ws".to_string(), '\t').unwrap()
            .mark_trans(&"start".to_string(), &"ws".to_string(), '\n').unwrap()
            .mark_trans(&"start".to_string(), &"lbr".to_string(), '{').unwrap()
            .mark_trans(&"start".to_string(), &"rbr".to_string(), '}').unwrap()
            .mark_trans(&"ws".to_string(), &"ws".to_string(), ' ').unwrap()
            .mark_trans(&"ws".to_string(), &"ws".to_string(), '\t').unwrap()
            .mark_trans(&"ws".to_string(), &"ws".to_string(), '\n').unwrap();
        builder
            .mark_token(&"lbr".to_string(), &"LBR".to_string())
            .mark_token(&"rbr".to_string(), &"RBR".to_string());
        builder
            .mark_start(&"start".to_string());
        builder
            .mark_accepting(&"lbr".to_string())
            .mark_accepting(&"rbr".to_string())
            .mark_accepting(&"ws".to_string());

        let cdfa: EncodedCDFA<String> = builder.build().unwrap();

        let input = "  {{\n}{}{} \t{} \t{}}".to_string();
        let mut iter = input.chars();

        let mut getter = || iter.next();

        let mut stream: StreamSource<char> = StreamSource::observe(&mut getter);

        let scanner = scan::def_scanner();

        //exercise
        let tokens = scanner.scan(&mut stream, &cdfa).unwrap();

        //verify
        assert_eq!(tokens_string(&tokens), "\
LBR <- '{'
LBR <- '{'
RBR <- '}'
LBR <- '{'
RBR <- '}'
LBR <- '{'
RBR <- '}'
LBR <- '{'
RBR <- '}'
LBR <- '{'
RBR <- '}'
RBR <- '}'
");
    }

    #[test]
    fn scan_fail_simple() {
        //setup
        let mut builder: EncodedCDFABuilder<String, String> = EncodedCDFABuilder::new();
        builder
            .set_alphabet("{} \t\n".chars());
        builder
            .mark_trans(&"start".to_string(), &"ws".to_string(), ' ').unwrap()
            .mark_trans(&"start".to_string(), &"ws".to_string(), '\t').unwrap()
            .mark_trans(&"start".to_string(), &"ws".to_string(), '\n').unwrap()
            .mark_trans(&"start".to_string(), &"lbr".to_string(), '{').unwrap()
            .mark_trans(&"start".to_string(), &"rbr".to_string(), '}').unwrap()
            .mark_trans(&"ws".to_string(), &"ws".to_string(), ' ').unwrap()
            .mark_trans(&"ws".to_string(), &"ws".to_string(), '\t').unwrap()
            .mark_trans(&"ws".to_string(), &"ws".to_string(), '\n').unwrap();
        builder
            .mark_token(&"lbr".to_string(), &"LBR".to_string())
            .mark_token(&"rbr".to_string(), &"RBR".to_string());
        builder
            .mark_start(&"start".to_string());
        builder
            .mark_accepting(&"lbr".to_string())
            .mark_accepting(&"rbr".to_string())
            .mark_accepting(&"ws".to_string());

        let cdfa: EncodedCDFA<String> = builder.build().unwrap();

        let input = "  {{\n}{}{} \tx{} \t{}}".to_string();
        let mut iter = input.chars();

        let mut getter = || iter.next();

        let mut stream: StreamSource<char> = StreamSource::observe(&mut getter);

        let scanner = scan::def_scanner();

        //exercise
        let result = scanner.scan(&mut stream, &cdfa);

        //verify
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(err.sequence, "x{} \t{}}");
        assert_eq!(err.line, 2);
        assert_eq!(err.character, 8);
    }

    #[test]
    fn scan_fail_complex() {
        //setup
        let mut builder: EncodedCDFABuilder<String, String> = EncodedCDFABuilder::new();
        builder
            .set_alphabet("{} \t\n".chars());
        builder
            .mark_trans(&"start".to_string(), &"ws".to_string(), ' ').unwrap()
            .mark_trans(&"start".to_string(), &"ws".to_string(), '\t').unwrap()
            .mark_trans(&"start".to_string(), &"ws".to_string(), '\n').unwrap()
            .mark_trans(&"start".to_string(), &"lbr".to_string(), '{').unwrap()
            .mark_trans(&"start".to_string(), &"rbr".to_string(), '}').unwrap()
            .mark_trans(&"ws".to_string(), &"ws".to_string(), ' ').unwrap()
            .mark_trans(&"ws".to_string(), &"ws".to_string(), '\t').unwrap()
            .mark_trans(&"ws".to_string(), &"ws".to_string(), '\n').unwrap();
        builder
            .mark_token(&"lbr".to_string(), &"LBR".to_string())
            .mark_token(&"rbr".to_string(), &"RBR".to_string());
        builder
            .mark_start(&"start".to_string());
        builder
            .mark_accepting(&"lbr".to_string())
            .mark_accepting(&"rbr".to_string())
            .mark_accepting(&"ws".to_string());

        let cdfa: EncodedCDFA<String> = builder.build().unwrap();

        let input = "   {  {  {{{\t}}}\n {} }  }   { {}\n }   {  {  {{{\t}}}\n {} }  } xyz  { {}\n }   {  {  {{{\t}}}\n {} }  }   { {}\n } ".to_string();
        let mut iter = input.chars();

        let mut getter = || iter.next();

        let mut stream: StreamSource<char> = StreamSource::observe(&mut getter);

        let scanner = scan::def_scanner();

        //exercise
        let result = scanner.scan(&mut stream, &cdfa);

        //verify
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(err.sequence, "xyz  { {}\n");
        assert_eq!(err.line, 4);
        assert_eq!(err.character, 10);
    }

    #[test]
    fn scan_chain_simple() {
        //setup
        let mut builder: EncodedCDFABuilder<String, String> = EncodedCDFABuilder::new();
        builder
            .set_alphabet("fourive".chars());
        builder
            .mark_chain(&"start".to_string(), &"four".to_string(), "four".chars()).unwrap()
            .mark_chain(&"start".to_string(), &"five".to_string(), "five".chars()).unwrap();
        builder
            .mark_token(&"four".to_string(), &"FOUR".to_string())
            .mark_token(&"five".to_string(), &"FIVE".to_string());
        builder
            .mark_start(&"start".to_string());
        builder
            .mark_accepting(&"four".to_string())
            .mark_accepting(&"five".to_string());

        let cdfa: EncodedCDFA<String> = builder.build().unwrap();

        let input = "fivefourfourfourfivefivefourfive".to_string();
        let mut iter = input.chars();

        let mut getter = || iter.next();

        let mut stream: StreamSource<char> = StreamSource::observe(&mut getter);

        let scanner = scan::def_scanner();

        //exercise
        let tokens = scanner.scan(&mut stream, &cdfa).unwrap();

        //verify
        assert_eq!(tokens_string(&tokens), "\
FIVE <- 'five'
FOUR <- 'four'
FOUR <- 'four'
FOUR <- 'four'
FIVE <- 'five'
FIVE <- 'five'
FOUR <- 'four'
FIVE <- 'five'
");
    }

    #[test]
    fn scan_chain_def() {
        //setup
        let mut builder: EncodedCDFABuilder<String, String> = EncodedCDFABuilder::new();
        builder
            .set_alphabet("fordk".chars());
        builder
            .mark_chain(&"start".to_string(), &"FOR".to_string(), "for".chars()).unwrap();
        builder
            .mark_token(&"FOR".to_string(), &"FOR".to_string())
            .mark_token(&"id".to_string(), &"ID".to_string());
        builder
            .mark_start(&"start".to_string());
        builder
            .mark_def(&"start".to_string(), &"id".to_string()).unwrap()
            .mark_def(&"id".to_string(), &"id".to_string()).unwrap();
        builder
            .mark_accepting(&"FOR".to_string())
            .mark_accepting(&"id".to_string());

        let cdfa: EncodedCDFA<String> = builder.build().unwrap();

        let input = "fdk".to_string();
        let mut iter = input.chars();

        let mut getter = || iter.next();

        let mut stream: StreamSource<char> = StreamSource::observe(&mut getter);

        let scanner = scan::def_scanner();

        //exercise
        let tokens = scanner.scan(&mut stream, &cdfa).unwrap();

        //verify
        assert_eq!(tokens_string(&tokens), "\
ID <- 'fdk'
");
    }

    fn tokens_string<Kind: Data>(tokens: &Vec<Token<Kind>>) -> String {
        let mut result = String::new();
        for token in tokens {
            result.push_str(&token.to_string());
            result.push('\n');
        }
        result
    }
}
