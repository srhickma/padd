use {
    core::{
        data::{
            map::{CEHashMap, CEHashMapIterator},
            Data,
        },
        lex::{
            alphabet::HashedAlphabet, CDFABuilder, CDFAError, ConsumerStrategy,
            TransitionDestination, TransitionResult, CDFA, Transit,
        },
        parse::grammar::GrammarSymbol,
        util::encoder::Encoder,
    },
    std::{collections::HashMap, fmt::Debug, usize},
};

pub struct EncodedCDFABuilder<State: Data, Symbol: GrammarSymbol> {
    encoder: Encoder<State>,
    alphabet_str: String,

    alphabet: HashedAlphabet,
    accepting: HashMap<usize, AcceptorDestinationMux>,
    t_delta: CEHashMap<TransitionTrie>,
    tokenizer: CEHashMap<Symbol>,
    start: usize,
}

impl<State: Data, Symbol: GrammarSymbol> EncodedCDFABuilder<State, Symbol> {
    fn get_transition_trie(&mut self, from: usize) -> &mut TransitionTrie {
        if !self.t_delta.contains(from) {
            self.t_delta.insert(from, TransitionTrie::new());
        }
        self.t_delta.get_mut(from).unwrap()
    }

    fn get_alphabet_range(&self, start: char, end: char) -> Vec<char> {
        let mut in_range = false;

        self.alphabet_str
            .chars()
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

    pub fn state<'scope, 'state: 'scope>(
        &'scope mut self,
        state: &'state State,
    ) -> EncodedCDFAStateBuilder<'scope, 'state, State, Symbol> {
        EncodedCDFAStateBuilder {
            ecdfa_builder: self,
            state,
        }
    }
}

impl<State: Data, Symbol: GrammarSymbol> CDFABuilder<State, Symbol, EncodedCDFA<Symbol>>
    for EncodedCDFABuilder<State, Symbol>
{
    fn new() -> Self {
        EncodedCDFABuilder {
            encoder: Encoder::new(),
            alphabet_str: String::new(),

            alphabet: HashedAlphabet::new(),
            accepting: HashMap::new(),
            t_delta: CEHashMap::new(),
            tokenizer: CEHashMap::new(),
            start: usize::max_value(),
        }
    }

    fn build(self) -> Result<EncodedCDFA<Symbol>, CDFAError> {
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

    fn set_alphabet(&mut self, chars: impl Iterator<Item = char>) -> &mut Self {
        chars.for_each(|c| {
            self.alphabet_str.push(c);
            self.alphabet.insert(c);
        });
        self
    }

    fn accept(&mut self, state: &State) -> &mut Self {
        let state_encoded = self.encoder.encode(state);

        if self.accepting.contains_key(&state_encoded) {
            return self;
        }

        self.accepting
            .insert(state_encoded, AcceptorDestinationMux::new());
        self
    }

    fn accept_to(
        &mut self,
        state: &State,
        from: &State,
        to: &State,
    ) -> Result<&mut Self, CDFAError> {
        let state_encoded = self.encoder.encode(state);
        let from_encoded = self.encoder.encode(from);
        let to_encoded = self.encoder.encode(to);

        self.accepting
            .entry(state_encoded)
            .or_insert_with(AcceptorDestinationMux::new)
            .add_from(state, to_encoded, from_encoded)?;

        Ok(self)
    }

    fn accept_to_from_all(&mut self, state: &State, to: &State) -> Result<&mut Self, CDFAError> {
        let state_encoded = self.encoder.encode(state);
        let to_encoded = self.encoder.encode(to);

        self.accepting
            .entry(state_encoded)
            .or_insert_with(AcceptorDestinationMux::new)
            .add_from_all(state, to_encoded)?;

        Ok(self)
    }

    fn mark_start(&mut self, state: &State) -> &mut Self {
        if self.start == usize::max_value() {
            self.start = self.encoder.encode(state);
        }
        self
    }

    fn mark_trans(
        &mut self,
        from: &State,
        transit: Transit<State>,
        on: char,
    ) -> Result<&mut Self, CDFAError> {
        let from_encoded = self.encoder.encode(from);
        let to_encoded = self.encoder.encode(&transit.dest);
        // TODO(shane) conglomerate this stuff
        let acceptor_destination_encoded = match transit.acceptor_destination {
            None => None,
            Some(acceptor_destination) => Some(self.encoder.encode(&acceptor_destination)),
        };
        let transition_delta = TransitionDestination::new(
            to_encoded,
            transit.consumer,
            acceptor_destination_encoded
        );

        {
            let t_trie = self.get_transition_trie(from_encoded);
            t_trie.insert(on, transition_delta)?;
        }

        Ok(self)
    }

    fn mark_chain(
        &mut self,
        from: &State,
        transit: Transit<State>,
        on: impl Iterator<Item = char>,
    ) -> Result<&mut Self, CDFAError> {
        let from_encoded = self.encoder.encode(from);
        let to_encoded = self.encoder.encode(&transit.dest);
        let acceptor_destination_encoded = match transit.acceptor_destination {
            None => None,
            Some(acceptor_destination) => Some(self.encoder.encode(&acceptor_destination)),
        };
        let transition_delta = TransitionDestination::new(
            to_encoded,
            transit.consumer,
            acceptor_destination_encoded
        );

        {
            let t_trie = self.get_transition_trie(from_encoded);

            let mut chars: Vec<char> = Vec::new();
            on.for_each(|c| chars.push(c));
            t_trie.insert_chain(&chars, transition_delta)?;
        }

        Ok(self)
    }

    fn mark_range(
        &mut self,
        from: &State,
        transit: Transit<State>,
        start: char,
        end: char,
    ) -> Result<&mut Self, CDFAError> {
        let to_mark = self.get_alphabet_range(start, end);

        for c in &to_mark {
            self.mark_trans(from, transit.clone(), *c)?;
        }

        Ok(self)
    }

    fn mark_range_for_all<'state_o: 'state_i, 'state_i>(
        &mut self,
        sources: impl Iterator<Item = &'state_i &'state_o State>,
        transit: Transit<State>,
        start: char,
        end: char,
    ) -> Result<&mut Self, CDFAError> where State: 'state_o {
        let to_mark = self.get_alphabet_range(start, end);

        for source in sources {
            for c in &to_mark {
                self.mark_trans(&source, transit.clone(), *c)?;
            }
        }

        Ok(self)
    }

    fn default_to(
        &mut self,
        from: &State,
        transit: Transit<State>,
    ) -> Result<&mut Self, CDFAError> {
        let from_encoded = self.encoder.encode(from);
        let to_encoded = self.encoder.encode(&transit.dest);
        let acceptor_destination_encoded = match transit.acceptor_destination {
            None => None,
            Some(acceptor_destination) => Some(self.encoder.encode(&acceptor_destination)),
        };
        let transition_delta = TransitionDestination::new( // TODO(shane) abstract this transformation
            to_encoded,
            transit.consumer,
            acceptor_destination_encoded
        );

        match {
            let t_trie = self.get_transition_trie(from_encoded);

            t_trie.set_default(transition_delta)
        } {
            Err(err) => Err(err),
            Ok(()) => Ok(self),
        }
    }

    fn tokenize(&mut self, state: &State, token: &Symbol) -> &mut Self {
        let state_encoded = self.encoder.encode(state);
        self.tokenizer.insert(state_encoded, token.clone());
        self
    }
}

pub struct EncodedCDFAStateBuilder<
    'scope,
    'state: 'scope,
    State: 'state + Data,
    Symbol: 'scope + GrammarSymbol,
> {
    ecdfa_builder: &'scope mut EncodedCDFABuilder<State, Symbol>,
    state: &'state State,
}

impl<'scope, 'state: 'scope, State: 'state + Data, Symbol: 'scope + GrammarSymbol>
    EncodedCDFAStateBuilder<'scope, 'state, State, Symbol>
{
    pub fn accept(&mut self) -> &mut Self {
        self.ecdfa_builder.accept(self.state);
        self
    }

    pub fn accept_to_from_all(&mut self, to: &State) -> Result<&mut Self, CDFAError> {
        self.ecdfa_builder.accept_to_from_all(self.state, to)?;
        Ok(self)
    }

    pub fn mark_trans(&mut self, transit: Transit<State>, on: char) -> Result<&mut Self, CDFAError> {
        self.ecdfa_builder
            .mark_trans(self.state, transit, on)?;
        Ok(self)
    }

    pub fn mark_chain(
        &mut self,
        transit: Transit<State>,
        on: impl Iterator<Item = char>,
    ) -> Result<&mut Self, CDFAError> {
        self.ecdfa_builder
            .mark_chain(self.state, transit, on)?;
        Ok(self)
    }

    pub fn mark_range(
        &mut self,
        transit: Transit<State>,
        start: char,
        end: char,
    ) -> Result<&mut Self, CDFAError> {
        self.ecdfa_builder
            .mark_range(self.state, transit, start, end)?;
        Ok(self)
    }

    pub fn default_to(&mut self, transit: Transit<State>) -> Result<&mut Self, CDFAError> {
        self.ecdfa_builder
            .default_to(self.state, transit)?;
        Ok(self)
    }

    pub fn tokenize(&mut self, token: &Symbol) -> &mut Self {
        self.ecdfa_builder.tokenize(self.state, token);
        self
    }
}

pub struct EncodedCDFA<Symbol: GrammarSymbol> {
    //TODO add separate error message if character not in alphabet
    #[allow(dead_code)]
    alphabet: HashedAlphabet,
    accepting: HashMap<usize, AcceptorDestinationMux>,
    t_delta: CEHashMap<TransitionTrie>,
    tokenizer: CEHashMap<Symbol>,
    start: usize,
}

impl<Symbol: GrammarSymbol> EncodedCDFA<Symbol> {
    pub fn produces(&self) -> CEHashMapIterator<Symbol> {
        self.tokenizer.iter()
    }
}

impl<Symbol: GrammarSymbol> CDFA<usize, Symbol> for EncodedCDFA<Symbol> {
    fn transition(&self, state: &usize, input: &[char]) -> TransitionResult<usize> {
        match self.t_delta.get(*state) {
            None => TransitionResult::fail(),
            Some(t_trie) => t_trie.transition(input),
        }
    }

    fn has_transition(&self, state: &usize, input: &[char]) -> bool {
        self.transition(state, input).state.is_some()
    }

    fn accepts(&self, state: &usize) -> bool {
        self.accepting.contains_key(state)
    }

    fn acceptor_destination(&self, state: &usize, from: &usize) -> Option<usize> {
        match self.accepting.get(state) {
            None => None,
            Some(accd_mux) => accd_mux.get_destination(*from),
        }
    }

    fn tokenize(&self, state: &usize) -> Option<Symbol> {
        match self.tokenizer.get(*state) {
            None => None,
            Some(dest) => Some(dest.clone()),
        }
    }

    fn start(&self) -> usize {
        self.start
    }
}

struct TransitionTrie {
    root: TransitionNode,
    default: Option<TransitionDestination<usize>>,
}

impl TransitionTrie {
    fn new() -> TransitionTrie {
        TransitionTrie {
            root: TransitionNode {
                children: HashMap::new(),
                dest: TransitionDestination::new(usize::max_value(), ConsumerStrategy::None, None),
            },
            default: None,
        }
    }

    fn transition(&self, input: &[char]) -> TransitionResult<usize> {
        let mut curr: &TransitionNode = &self.root;

        if curr.children.is_empty() {
            match self.default {
                None => TransitionResult::fail(),
                Some(ref dest) => match input.first() {
                    None => TransitionResult::fail(),
                    Some(_) => TransitionResult::direct(dest),
                },
            }
        } else {
            let mut cursor: usize = 0;
            while !curr.leaf() {
                curr = match input.get(cursor) {
                    None => return TransitionResult::fail(),
                    Some(c) => match curr.get_child(*c) {
                        None => match self.default {
                            None => return TransitionResult::fail(),
                            Some(ref dest) => return TransitionResult::direct(dest),
                        },
                        Some(child) => child,
                    },
                };

                cursor += 1;
            }

            TransitionResult::new(&curr.dest, cursor)
        }
    }

    fn insert(&mut self, c: char, dest: TransitionDestination<usize>) -> Result<(), CDFAError> {
        TransitionTrie::insert_internal(c, &mut self.root, true, dest)
    }

    fn insert_chain(
        &mut self,
        chars: &[char],
        dest: TransitionDestination<usize>,
    ) -> Result<(), CDFAError> {
        TransitionTrie::insert_chain_internal(0, &mut self.root, chars, dest)
    }

    fn insert_chain_internal(
        i: usize,
        node: &mut TransitionNode,
        chars: &[char],
        dest: TransitionDestination<usize>,
    ) -> Result<(), CDFAError> {
        if i == chars.len() {
            return Ok(());
        }

        let c = chars[i];
        TransitionTrie::insert_internal(c, node, i == chars.len() - 1, dest.clone())?;
        TransitionTrie::insert_chain_internal(i + 1, node.get_child_mut(c).unwrap(), chars, dest)
    }

    fn insert_internal(
        c: char,
        node: &mut TransitionNode,
        last: bool,
        dest: TransitionDestination<usize>,
    ) -> Result<(), CDFAError> {
        if !node.has_child(c) {
            let child = TransitionNode {
                children: HashMap::new(),
                dest: if last {
                    dest
                } else {
                    TransitionDestination::new(usize::max_value(), ConsumerStrategy::None, None)
                },
            };
            node.add_child(c, child);
        } else if last {
            return Err(CDFAError::BuildErr(format!(
                "Transition trie is not prefix free on character '{}'",
                c
            )));
        }
        Ok(())
    }

    fn set_default(&mut self, default: TransitionDestination<usize>) -> Result<(), CDFAError> {
        if self.default.is_some() {
            Err(CDFAError::BuildErr(
                "Default matcher used twice".to_string(),
            ))
        } else {
            self.default = Some(default);
            Ok(())
        }
    }
}

impl Default for TransitionTrie {
    fn default() -> TransitionTrie {
        TransitionTrie::new()
    }
}

struct TransitionNode {
    children: HashMap<char, TransitionNode>,
    dest: TransitionDestination<usize>,
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

struct AcceptorDestinationMux {
    mux: Option<HashMap<usize, usize>>,
    all: Option<usize>,
}

impl AcceptorDestinationMux {
    fn new() -> Self {
        AcceptorDestinationMux {
            mux: None,
            all: None,
        }
    }

    fn add_from<State: Debug>(
        &mut self,
        state: &State,
        dest_encoded: usize,
        from_encoded: usize,
    ) -> Result<(), CDFAError> {
        if self.all.is_some() {
            return Err(CDFAError::BuildErr(format!(
                "State {:?} already has an acceptance destination from all incoming states",
                state
            )));
        }

        if self.mux.is_none() {
            let mut mux = HashMap::new();
            mux.insert(from_encoded, dest_encoded);
            self.mux = Some(mux);
        } else if let Some(ref mut mux) = self.mux {
            {
                let entry = mux.get(&from_encoded);
                if entry.is_some() && *entry.unwrap() != dest_encoded {
                    return Err(CDFAError::BuildErr(format!(
                        "State {:?} is accepted multiple times with different destinations",
                        state
                    )));
                }
            }

            mux.insert(from_encoded, dest_encoded);
        }

        Ok(())
    }

    fn add_from_all<State: Debug>(
        &mut self,
        state: &State,
        dest_encoded: usize,
    ) -> Result<(), CDFAError> {
        if self.mux.is_some() {
            return Err(CDFAError::BuildErr(format!(
                "State {:?} already has an acceptance destination from a specific state",
                state
            )));
        }

        self.all = Some(dest_encoded);

        Ok(())
    }

    fn get_destination(&self, from_encoded: usize) -> Option<usize> {
        if let Some(dest) = self.all {
            Some(dest)
        } else if let Some(ref mux) = self.mux {
            match mux.get(&from_encoded) {
                None => None,
                Some(dest) => Some(*dest),
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use core::{
        data::Data,
        lex::{self, Token, TransitBuilder},
    };

    use super::*;

    #[test]
    fn lex_binary() {
        //setup
        let mut builder: EncodedCDFABuilder<String, String> = EncodedCDFABuilder::new();
        builder
            .set_alphabet("01".chars())
            .mark_start(&"start".to_string());
        builder
            .state(&"start".to_string())
            .mark_trans(Transit::to("zero".to_string()), '0')
            .unwrap()
            .mark_trans(Transit::to("notzero".to_string()), '1')
            .unwrap();
        builder
            .state(&"notzero".to_string())
            .default_to(Transit::to("notzero".to_string()))
            .unwrap()
            .accept()
            .tokenize(&"NZ".to_string());
        builder
            .accept(&"zero".to_string())
            .tokenize(&"zero".to_string(), &"ZERO".to_string());

        let cdfa: EncodedCDFA<String> = builder.build().unwrap();

        let input = "000011010101".to_string();
        let chars: Vec<char> = input.chars().collect();

        let lexer = lex::def_lexer();

        //exercise
        let tokens = lexer.lex(&chars[..], &cdfa).unwrap();

        //verify
        assert_eq!(
            tokens_string(&tokens),
            "\
ZERO <- '0'
ZERO <- '0'
ZERO <- '0'
ZERO <- '0'
NZ <- '11010101'
"
        );
    }

    #[test]
    fn lex_brackets() {
        //setup
        let mut builder: EncodedCDFABuilder<String, String> = EncodedCDFABuilder::new();
        builder
            .set_alphabet("{} \t\n".chars())
            .mark_start(&"start".to_string());
        builder
            .state(&"start".to_string())
            .mark_trans(Transit::to("ws".to_string()), ' ')
            .unwrap()
            .mark_trans(Transit::to("ws".to_string()), '\t')
            .unwrap()
            .mark_trans(Transit::to("ws".to_string()), '\n')
            .unwrap()
            .mark_trans(Transit::to("lbr".to_string()), '{')
            .unwrap()
            .mark_trans(Transit::to("rbr".to_string()), '}')
            .unwrap();
        builder
            .state(&"ws".to_string())
            .mark_trans(Transit::to("ws".to_string()), ' ')
            .unwrap()
            .mark_trans(Transit::to("ws".to_string()), '\t')
            .unwrap()
            .mark_trans(Transit::to("ws".to_string()), '\n')
            .unwrap()
            .accept()
            .tokenize(&"WS".to_string());
        builder
            .accept(&"lbr".to_string())
            .tokenize(&"lbr".to_string(), &"LBR".to_string())
            .accept(&"rbr".to_string())
            .tokenize(&"rbr".to_string(), &"RBR".to_string());

        let cdfa: EncodedCDFA<String> = builder.build().unwrap();

        let input = "  {{\n}{}{} \t{} \t{}}".to_string();
        let chars: Vec<char> = input.chars().collect();

        let lexer = lex::def_lexer();

        //exercise
        let tokens = lexer.lex(&chars[..], &cdfa).unwrap();

        //verify
        assert_eq!(
            tokens_string(&tokens),
            "\
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
"
        );
    }

    #[test]
    fn lex_ignore() {
        //setup
        let mut builder: EncodedCDFABuilder<String, String> = EncodedCDFABuilder::new();
        builder
            .set_alphabet("{} \t\n".chars())
            .mark_start(&"start".to_string());
        builder
            .state(&"start".to_string())
            .mark_trans(Transit::to("ws".to_string()), ' ')
            .unwrap()
            .mark_trans(Transit::to("ws".to_string()), '\t')
            .unwrap()
            .mark_trans(Transit::to("ws".to_string()), '\n')
            .unwrap()
            .mark_trans(Transit::to("lbr".to_string()), '{')
            .unwrap()
            .mark_trans(Transit::to("rbr".to_string()), '}')
            .unwrap();
        builder
            .state(&"ws".to_string())
            .mark_trans(Transit::to("ws".to_string()), ' ')
            .unwrap()
            .mark_trans(Transit::to("ws".to_string()), '\t')
            .unwrap()
            .mark_trans(Transit::to("ws".to_string()), '\n')
            .unwrap()
            .accept();
        builder
            .accept(&"lbr".to_string())
            .tokenize(&"lbr".to_string(), &"LBR".to_string())
            .accept(&"rbr".to_string())
            .tokenize(&"rbr".to_string(), &"RBR".to_string());

        let cdfa: EncodedCDFA<String> = builder.build().unwrap();

        let input = "  {{\n}{}{} \t{} \t{}}".to_string();
        let chars: Vec<char> = input.chars().collect();

        let lexer = lex::def_lexer();

        //exercise
        let tokens = lexer.lex(&chars[..], &cdfa).unwrap();

        //verify
        assert_eq!(
            tokens_string(&tokens),
            "\
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
"
        );
    }

    #[test]
    fn lex_fail_simple() {
        //setup
        let mut builder: EncodedCDFABuilder<String, String> = EncodedCDFABuilder::new();
        builder
            .set_alphabet("{} \t\n".chars())
            .mark_start(&"start".to_string());
        builder
            .state(&"start".to_string())
            .mark_trans(Transit::to("ws".to_string()), ' ')
            .unwrap()
            .mark_trans(Transit::to("ws".to_string()), '\t')
            .unwrap()
            .mark_trans(Transit::to("ws".to_string()), '\n')
            .unwrap()
            .mark_trans(Transit::to("lbr".to_string()), '{')
            .unwrap()
            .mark_trans(Transit::to("rbr".to_string()), '}')
            .unwrap();
        builder
            .state(&"ws".to_string())
            .mark_trans(Transit::to("ws".to_string()), ' ')
            .unwrap()
            .mark_trans(Transit::to("ws".to_string()), '\t')
            .unwrap()
            .mark_trans(Transit::to("ws".to_string()), '\n')
            .unwrap()
            .accept();
        builder
            .accept(&"lbr".to_string())
            .tokenize(&"lbr".to_string(), &"LBR".to_string())
            .accept(&"rbr".to_string())
            .tokenize(&"rbr".to_string(), &"RBR".to_string());

        let cdfa: EncodedCDFA<String> = builder.build().unwrap();

        let input = "  {{\n}{}{} \tx{} \t{}}".to_string();
        let chars: Vec<char> = input.chars().collect();

        let lexer = lex::def_lexer();

        //exercise
        let result = lexer.lex(&chars[..], &cdfa);

        //verify
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(err.sequence, "x{} \t{}}");
        assert_eq!(err.line, 2);
        assert_eq!(err.character, 8);
    }

    #[test]
    fn lex_fail_complex() {
        //setup
        let mut builder: EncodedCDFABuilder<String, String> = EncodedCDFABuilder::new();
        builder
            .set_alphabet("{} \t\n".chars())
            .mark_start(&"start".to_string());
        builder
            .state(&"start".to_string())
            .mark_trans(Transit::to("ws".to_string()), ' ')
            .unwrap()
            .mark_trans(Transit::to("ws".to_string()), '\t')
            .unwrap()
            .mark_trans(Transit::to("ws".to_string()), '\n')
            .unwrap()
            .mark_trans(Transit::to("lbr".to_string()), '{')
            .unwrap()
            .mark_trans(Transit::to("rbr".to_string()), '}')
            .unwrap();
        builder
            .state(&"ws".to_string())
            .mark_trans(Transit::to("ws".to_string()), ' ')
            .unwrap()
            .mark_trans(Transit::to("ws".to_string()), '\t')
            .unwrap()
            .mark_trans(Transit::to("ws".to_string()), '\n')
            .unwrap()
            .accept();
        builder
            .accept(&"lbr".to_string())
            .tokenize(&"lbr".to_string(), &"LBR".to_string())
            .accept(&"rbr".to_string())
            .tokenize(&"rbr".to_string(), &"RBR".to_string());

        let cdfa: EncodedCDFA<String> = builder.build().unwrap();

        let input = "   {  {  {{{\t}}}\n {} }  }   { {}\n }   {  {  {{{\t}}}\n {} }  } xyz  { {}\n }   {  {  {{{\t}}}\n {} }  }   { {}\n } ".to_string();
        let chars: Vec<char> = input.chars().collect();

        let lexer = lex::def_lexer();

        //exercise
        let result = lexer.lex(&chars[..], &cdfa);

        //verify
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(err.sequence, "xyz  { {}\n");
        assert_eq!(err.line, 4);
        assert_eq!(err.character, 10);
    }

    #[test]
    fn lex_chain_simple() {
        //setup
        let mut builder: EncodedCDFABuilder<String, String> = EncodedCDFABuilder::new();
        builder
            .set_alphabet("fourive".chars())
            .mark_start(&"start".to_string());
        builder
            .state(&"start".to_string())
            .mark_chain(Transit::to("four".to_string()), "four".chars())
            .unwrap()
            .mark_chain(Transit::to("five".to_string()), "five".chars())
            .unwrap();
        builder
            .accept(&"four".to_string())
            .tokenize(&"four".to_string(), &"FOUR".to_string())
            .accept(&"five".to_string())
            .tokenize(&"five".to_string(), &"FIVE".to_string());

        let cdfa: EncodedCDFA<String> = builder.build().unwrap();

        let input = "fivefourfourfourfivefivefourfive".to_string();
        let chars: Vec<char> = input.chars().collect();

        let lexer = lex::def_lexer();

        //exercise
        let tokens = lexer.lex(&chars[..], &cdfa).unwrap();

        //verify
        assert_eq!(
            tokens_string(&tokens),
            "\
FIVE <- 'five'
FOUR <- 'four'
FOUR <- 'four'
FOUR <- 'four'
FIVE <- 'five'
FIVE <- 'five'
FOUR <- 'four'
FIVE <- 'five'
"
        );
    }

    #[test]
    fn lex_chain_def() {
        //setup
        let mut builder: EncodedCDFABuilder<String, String> = EncodedCDFABuilder::new();
        builder
            .set_alphabet("fordk".chars())
            .mark_start(&"start".to_string());
        builder
            .state(&"start".to_string())
            .mark_chain(Transit::to("FOR".to_string()), "for".chars())
            .unwrap()
            .default_to(Transit::to("id".to_string()))
            .unwrap();
        builder
            .accept(&"FOR".to_string())
            .tokenize(&"FOR".to_string(), &"FOR".to_string())
            .accept(&"id".to_string())
            .tokenize(&"id".to_string(), &"ID".to_string());
        builder
            .default_to(&"id".to_string(), Transit::to("id".to_string()))
            .unwrap();

        let cdfa: EncodedCDFA<String> = builder.build().unwrap();

        let input = "fdk".to_string();
        let chars: Vec<char> = input.chars().collect();

        let lexer = lex::def_lexer();

        //exercise
        let tokens = lexer.lex(&chars[..], &cdfa).unwrap();

        //verify
        assert_eq!(
            tokens_string(&tokens),
            "\
ID <- 'fdk'
"
        );
    }

    #[test]
    fn lex_context_sensitive() {
        //setup
        #[derive(PartialEq, Eq, Hash, Clone, Debug)]
        enum S {
            Start,
            A,
            BangIn,
            BangOut,
            Hidden,
            Num,
        }

        impl Data for S {
            fn to_string(&self) -> String {
                format!("{:?}", self)
            }
        }

        let mut builder: EncodedCDFABuilder<S, String> = EncodedCDFABuilder::new();
        builder
            .set_alphabet("a!123456789".chars())
            .mark_start(&S::Start);
        builder
            .state(&S::Start)
            .mark_trans(Transit::to(S::A), 'a')
            .unwrap()
            .mark_trans(Transit::to(S::BangIn), '!')
            .unwrap();
        builder
            .state(&S::A)
            .mark_trans(Transit::to(S::A), 'a')
            .unwrap()
            .tokenize(&"A".to_string())
            .accept();
        builder
            .state(&S::BangIn)
            .tokenize(&"BANG".to_string())
            .accept_to_from_all(&S::Hidden)
            .unwrap();
        builder
            .state(&S::BangOut)
            .tokenize(&"BANG".to_string())
            .accept_to_from_all(&S::Start)
            .unwrap();
        builder
            .state(&S::Hidden)
            .mark_range(Transit::to(S::Num), '1', '9')
            .unwrap()
            .mark_trans(Transit::to(S::BangOut), '!')
            .unwrap();
        builder.state(&S::Num).tokenize(&"NUM".to_string()).accept();

        let cdfa: EncodedCDFA<String> = builder.build().unwrap();

        let input = "!!aaa!!a!49913!a".to_string();
        let chars: Vec<char> = input.chars().collect();

        let lexer = lex::def_lexer();

        //exercise
        let tokens = lexer.lex(&chars[..], &cdfa).unwrap();

        //verify
        assert_eq!(
            tokens_string(&tokens),
            "\
BANG <- '!'
BANG <- '!'
A <- 'aaa'
BANG <- '!'
BANG <- '!'
A <- 'a'
BANG <- '!'
NUM <- '4'
NUM <- '9'
NUM <- '9'
NUM <- '1'
NUM <- '3'
BANG <- '!'
A <- 'a'
"
        );
    }

    #[test]
    fn multiple_acceptor_destinations() {
        //setup
        #[derive(PartialEq, Eq, Hash, Clone, Debug)]
        enum S {
            Start,
            A,
            LastA,
        }

        impl Data for S {
            fn to_string(&self) -> String {
                format!("{:?}", self)
            }
        }

        let mut builder: EncodedCDFABuilder<S, String> = EncodedCDFABuilder::new();
        builder.set_alphabet("a".chars()).mark_start(&S::Start);
        builder
            .mark_trans(&S::Start, Transit::to(S::A), 'a')
            .unwrap();
        builder.accept_to(&S::A, &S::Start, &S::LastA).unwrap();
        builder
            .mark_trans(&S::LastA, Transit::to(S::A), 'a')
            .unwrap();
        builder.accept_to(&S::A, &S::LastA, &S::Start).unwrap();
        builder.state(&S::A).tokenize(&"A".to_string());

        let cdfa: EncodedCDFA<String> = builder.build().unwrap();

        let input = "aaaa".to_string();
        let chars: Vec<char> = input.chars().collect();

        let lexer = lex::def_lexer();

        //exercise
        let tokens = lexer.lex(&chars[..], &cdfa).unwrap();

        //verify
        assert_eq!(
            tokens_string(&tokens),
            "\
A <- 'a'
A <- 'a'
A <- 'a'
A <- 'a'
"
        );
    }

    #[test]
    fn accept_from_all_twice() {
        //setup
        #[derive(PartialEq, Eq, Hash, Clone, Debug)]
        enum S {
            Start,
            A,
            LastA,
        }

        impl Data for S {
            fn to_string(&self) -> String {
                format!("{:?}", self)
            }
        }

        let mut builder: EncodedCDFABuilder<S, String> = EncodedCDFABuilder::new();
        builder.set_alphabet("a".chars()).mark_start(&S::Start);
        builder
            .mark_trans(&S::Start, Transit::to(S::A), 'a')
            .unwrap();
        builder.accept_to_from_all(&S::A, &S::LastA).unwrap();
        builder.accept_to_from_all(&S::A, &S::Start).unwrap();
        builder.state(&S::A).tokenize(&"A".to_string());

        let cdfa: EncodedCDFA<String> = builder.build().unwrap();

        let input = "aa".to_string();
        let chars: Vec<char> = input.chars().collect();

        let lexer = lex::def_lexer();

        //exercise
        let tokens = lexer.lex(&chars[..], &cdfa).unwrap();

        //verify
        assert_eq!(
            tokens_string(&tokens),
            "\
A <- 'a'
A <- 'a'
"
        );
    }

    #[test]
    fn non_consuming_transitions() {
        //setup
        #[derive(PartialEq, Eq, Hash, Clone, Debug)]
        enum S {
            Start1,
            Start2,
            A,
            B,
        }

        impl Data for S {
            fn to_string(&self) -> String {
                format!("{:?}", self)
            }
        }

        let mut builder: EncodedCDFABuilder<S, String> = EncodedCDFABuilder::new();
        builder.set_alphabet("ab".chars()).mark_start(&S::Start1);
        builder
            .mark_trans(
                &S::Start1,
                TransitBuilder::to(S::A).consumer(ConsumerStrategy::All).build(),
                'a'
            ).unwrap()
            .default_to(
                &S::Start1,
                TransitBuilder::to(S::Start2).consumer(ConsumerStrategy::None).build()
            ).unwrap();
        builder
            .mark_trans(
                &S::Start2,
                TransitBuilder::to(S::B).consumer(ConsumerStrategy::All).build(),
                'b'
            ).unwrap();
        builder.accept(&S::A);
        builder.accept(&S::B);
        builder.state(&S::A).tokenize(&"A".to_string());
        builder.state(&S::B).tokenize(&"B".to_string());

        let cdfa: EncodedCDFA<String> = builder.build().unwrap();

        let input = "ab".to_string();
        let chars: Vec<char> = input.chars().collect();

        let lexer = lex::def_lexer();

        //exercise
        let tokens = lexer.lex(&chars[..], &cdfa).unwrap();

        //verify
        assert_eq!(
            tokens_string(&tokens),
            "\
A <- 'a'
B <- 'b'
"
        );
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
