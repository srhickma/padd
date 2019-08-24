use {
    core::{
        data::{
            interval::{Bound, Interval, IntervalMap},
            map::{CEHashMap, CEHashMapIterator},
            Data,
        },
        lex::{
            alphabet::{Alphabet, HashedAlphabet},
            CDFABuilder, CDFAError, Transit, TransitBuilder, TransitionResult, CDFA,
        },
        parse::grammar::GrammarSymbol,
        util::encoder::Encoder,
    },
    std::{collections::HashMap, ops::RangeInclusive, usize},
};

pub struct EncodedCDFABuilder<State: Data, Symbol: GrammarSymbol> {
    encoder: Encoder<State>,
    alphabet: Option<HashedAlphabet>,
    accepting: HashMap<usize, Option<usize>>,
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

    pub fn state<'scope, 'state: 'scope>(
        &'scope mut self,
        state: &'state State,
    ) -> EncodedCDFAStateBuilder<'scope, 'state, State, Symbol> {
        EncodedCDFAStateBuilder {
            ecdfa_builder: self,
            state,
        }
    }

    fn encode_transit(&mut self, transit: Transit<State>) -> Transit<usize> {
        let mut builder = TransitBuilder::to(self.encoder.encode(&transit.dest));

        if let Some(acceptor_destination) = transit.acceptor_destination {
            builder.accept_to(self.encoder.encode(&acceptor_destination));
        }

        builder.consumer(transit.consumer).build()
    }
}

impl<State: Data, Symbol: GrammarSymbol> CDFABuilder<State, Symbol, EncodedCDFA<Symbol>>
    for EncodedCDFABuilder<State, Symbol>
{
    fn new() -> Self {
        EncodedCDFABuilder {
            encoder: Encoder::new(),
            alphabet: None,
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
        let mut alphabet = HashedAlphabet::new();
        chars.for_each(|c| alphabet.insert(c));
        self.alphabet = Some(alphabet);
        self
    }

    fn accept(&mut self, state: &State) -> &mut Self {
        let state_encoded = self.encoder.encode(state);

        self.accepting.entry(state_encoded).or_insert(None);
        self
    }

    fn accept_to(&mut self, state: &State, to: &State) -> &mut Self {
        let state_encoded = self.encoder.encode(state);
        let to_encoded = self.encoder.encode(to);

        self.accepting.insert(state_encoded, Some(to_encoded));
        self
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
        let transit_encoded = self.encode_transit(transit);

        {
            let t_trie = self.get_transition_trie(from_encoded);
            t_trie.insert(on, transit_encoded)?;
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
        let transit_encoded = self.encode_transit(transit);

        {
            let t_trie = self.get_transition_trie(from_encoded);

            let mut chars: Vec<char> = Vec::new();
            on.for_each(|c| chars.push(c));
            t_trie.insert_chain(&chars, transit_encoded)?;
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
        let from_encoded = self.encoder.encode(from);
        let transit_encoded = self.encode_transit(transit);

        {
            let t_trie = self.get_transition_trie(from_encoded);
            t_trie.insert_range((start as u32)..=(end as u32), transit_encoded)?;
        }

        Ok(self)
    }

    fn mark_range_for_all<'state_o: 'state_i, 'state_i>(
        &mut self,
        sources: impl Iterator<Item = &'state_i &'state_o State>,
        transit: Transit<State>,
        start: char,
        end: char,
    ) -> Result<&mut Self, CDFAError>
    where
        State: 'state_o,
    {
        for source in sources {
            self.mark_range(&source, transit.clone(), start, end)?;
        }

        Ok(self)
    }

    fn default_to(
        &mut self,
        from: &State,
        transit: Transit<State>,
    ) -> Result<&mut Self, CDFAError> {
        let from_encoded = self.encoder.encode(from);
        let transit_encoded = self.encode_transit(transit);

        match {
            let t_trie = self.get_transition_trie(from_encoded);

            t_trie.set_default(transit_encoded)
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

    pub fn accept_to(&mut self, to: &State) -> &mut Self {
        self.ecdfa_builder.accept_to(self.state, to);
        self
    }

    pub fn mark_trans(
        &mut self,
        transit: Transit<State>,
        on: char,
    ) -> Result<&mut Self, CDFAError> {
        self.ecdfa_builder.mark_trans(self.state, transit, on)?;
        Ok(self)
    }

    pub fn mark_chain(
        &mut self,
        transit: Transit<State>,
        on: impl Iterator<Item = char>,
    ) -> Result<&mut Self, CDFAError> {
        self.ecdfa_builder.mark_chain(self.state, transit, on)?;
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
        self.ecdfa_builder.default_to(self.state, transit)?;
        Ok(self)
    }

    pub fn tokenize(&mut self, token: &Symbol) -> &mut Self {
        self.ecdfa_builder.tokenize(self.state, token);
        self
    }
}

pub struct EncodedCDFA<Symbol: GrammarSymbol> {
    alphabet: Option<HashedAlphabet>,
    accepting: HashMap<usize, Option<usize>>,
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
            None => TransitionResult::Fail,
            Some(t_trie) => t_trie.transition(input),
        }
    }

    fn alphabet_contains(&self, c: char) -> bool {
        match self.alphabet {
            Some(ref alphabet) => alphabet.contains(c),
            None => true,
        }
    }

    fn accepts(&self, state: &usize) -> bool {
        self.accepting.contains_key(state)
    }

    fn default_acceptor_destination(&self, state: &usize) -> Option<usize> {
        match self.accepting.get(state) {
            Some(Some(acceptor_destination)) => Some(*acceptor_destination),
            _ => None,
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
    ranges: IntervalMap<u32, Transit<usize>>,
    default: Option<Transit<usize>>,
}

impl Bound for u32 {
    fn predecessor(&self) -> Self {
        self - 1
    }
}

impl TransitionTrie {
    fn new() -> TransitionTrie {
        TransitionTrie {
            root: TransitionNode {
                children: HashMap::new(),
                transit: None,
            },
            ranges: IntervalMap::new(),
            default: None,
        }
    }

    fn transition(&self, input: &[char]) -> TransitionResult<usize> {
        if input.is_empty() {
            return TransitionResult::Fail;
        }

        match self.transition_explicit(input) {
            TransitionResult::Fail => match self.transition_range(input) {
                TransitionResult::Fail => self.transition_default(),
                result => result,
            },
            result => result,
        }
    }

    fn transition_explicit(&self, input: &[char]) -> TransitionResult<usize> {
        let mut curr: &TransitionNode = &self.root;

        if curr.children.is_empty() {
            TransitionResult::Fail
        } else {
            let mut best_result = TransitionResult::Fail;

            let mut cursor: usize = 0;
            while !curr.leaf() {
                curr = match input.get(cursor) {
                    None => return best_result,
                    Some(c) => match curr.get_child(*c) {
                        None => return best_result,
                        Some(child) => child,
                    },
                };

                cursor += 1;

                if let Some(transit) = &curr.transit {
                    best_result = TransitionResult::ok(transit, cursor);
                }
            }

            best_result
        }
    }

    fn transition_range(&self, input: &[char]) -> TransitionResult<usize> {
        let value = input[0] as u32;
        match self.ranges.get(&value) {
            None => TransitionResult::Fail,
            Some(transit) => TransitionResult::direct(transit),
        }
    }

    fn transition_default(&self) -> TransitionResult<usize> {
        match self.default {
            None => TransitionResult::Fail,
            Some(ref dest) => TransitionResult::direct(dest),
        }
    }

    fn insert(&mut self, c: char, transit: Transit<usize>) -> Result<(), CDFAError> {
        TransitionTrie::insert_internal(c, &mut self.root, true, transit)
    }

    fn insert_chain(&mut self, chars: &[char], transit: Transit<usize>) -> Result<(), CDFAError> {
        TransitionTrie::insert_chain_internal(0, &mut self.root, chars, transit)
    }

    fn insert_range(
        &mut self,
        range: RangeInclusive<u32>,
        transit: Transit<usize>,
    ) -> Result<(), CDFAError> {
        self.ranges.insert(Interval::from(range), transit)?;
        Ok(())
    }

    fn insert_chain_internal(
        i: usize,
        node: &mut TransitionNode,
        chars: &[char],
        transit: Transit<usize>,
    ) -> Result<(), CDFAError> {
        if i == chars.len() {
            return Ok(());
        }

        let c = chars[i];
        TransitionTrie::insert_internal(c, node, i == chars.len() - 1, transit.clone())?;
        TransitionTrie::insert_chain_internal(i + 1, node.get_child_mut(c).unwrap(), chars, transit)
    }

    fn insert_internal(
        c: char,
        node: &mut TransitionNode,
        last: bool,
        transit: Transit<usize>,
    ) -> Result<(), CDFAError> {
        if !node.has_child(c) {
            let child = TransitionNode {
                children: HashMap::new(),
                transit: if last { Some(transit) } else { None },
            };
            node.add_child(c, child);
        } else if last {
            let child = node.get_child_mut(c).unwrap();
            if child.transit.is_some() {
                return Err(CDFAError::BuildErr(
                    "Transition trie contains duplicate matchers".to_string()
                ));
            } else {
                child.transit = Some(transit);
            }
        }
        Ok(())
    }

    fn set_default(&mut self, transit: Transit<usize>) -> Result<(), CDFAError> {
        if self.default.is_some() {
            Err(CDFAError::BuildErr(
                "Default matcher used twice".to_string(),
            ))
        } else {
            self.default = Some(transit);
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
    transit: Option<Transit<usize>>,
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
        data::Data,
        lex::{self, ConsumerStrategy, Token, TransitBuilder},
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
        match result.err().unwrap() {
            lex::Error::UnacceptedErr(err) => {
                assert_eq!(err.sequence, "x{} \t{}}");
                assert_eq!(err.line, 2);
                assert_eq!(err.character, 8);
            }
            _ => panic!("Unexpected error type"),
        }
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
        match result.err().unwrap() {
            lex::Error::UnacceptedErr(err) => {
                assert_eq!(err.sequence, "xyz  { {}\n");
                assert_eq!(err.line, 4);
                assert_eq!(err.character, 10);
            }
            _ => panic!("Unexpected error type"),
        }
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
            .accept_to(&S::Hidden);
        builder
            .state(&S::BangOut)
            .tokenize(&"BANG".to_string())
            .accept_to(&S::Start);
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
            .mark_trans(
                &S::Start,
                TransitBuilder::to(S::A).accept_to(S::LastA).build(),
                'a',
            )
            .unwrap();
        builder
            .mark_trans(
                &S::LastA,
                TransitBuilder::to(S::A).accept_to(S::Start).build(),
                'a',
            )
            .unwrap();
        builder.state(&S::A).accept().tokenize(&"A".to_string());

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
        builder.accept_to(&S::A, &S::LastA);
        builder.accept_to(&S::A, &S::Start);
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
    fn multiple_transition_acceptor_destinations() {
        //setup
        #[derive(PartialEq, Eq, Hash, Clone, Debug)]
        enum S {
            Start,
            A,
            A1,
            A2,
            B1,
            B2,
        }

        impl Data for S {
            fn to_string(&self) -> String {
                format!("{:?}", self)
            }
        }

        let mut builder: EncodedCDFABuilder<S, String> = EncodedCDFABuilder::new();
        builder.set_alphabet("12a".chars()).mark_start(&S::Start);
        builder
            .state(&S::Start)
            .mark_trans(TransitBuilder::to(S::A).accept_to(S::B1).build(), '1')
            .unwrap()
            .mark_trans(TransitBuilder::to(S::A).accept_to(S::B2).build(), '2')
            .unwrap();
        builder
            .state(&S::B1)
            .mark_trans(Transit::to(S::A1), 'a')
            .unwrap();
        builder
            .state(&S::B2)
            .mark_trans(Transit::to(S::A2), 'a')
            .unwrap();
        builder.state(&S::A).accept().tokenize(&"A".to_string());
        builder
            .state(&S::A1)
            .accept_to(&S::Start)
            .tokenize(&"A1".to_string());
        builder
            .state(&S::A2)
            .accept_to(&S::Start)
            .tokenize(&"A2".to_string());

        let cdfa: EncodedCDFA<String> = builder.build().unwrap();

        let input = "1a2a1a".to_string();
        let chars: Vec<char> = input.chars().collect();

        let lexer = lex::def_lexer();

        //exercise
        let tokens = lexer.lex(&chars[..], &cdfa).unwrap();

        //verify
        assert_eq!(
            tokens_string(&tokens),
            "\
A <- '1'
A1 <- 'a'
A <- '2'
A2 <- 'a'
A <- '1'
A1 <- 'a'
"
        );
    }

    #[test]
    fn acceptor_destination_precedence() {
        //setup
        #[derive(PartialEq, Eq, Hash, Clone, Debug)]
        enum S {
            Start,
            A,
            A1,
            A2,
            B1,
            B2,
        }

        impl Data for S {
            fn to_string(&self) -> String {
                format!("{:?}", self)
            }
        }

        let mut builder: EncodedCDFABuilder<S, String> = EncodedCDFABuilder::new();
        builder.set_alphabet("12a".chars()).mark_start(&S::Start);
        builder
            .state(&S::Start)
            .mark_trans(TransitBuilder::to(S::A).accept_to(S::B1).build(), '1')
            .unwrap()
            .mark_trans(Transit::to(S::A), '2')
            .unwrap();
        builder
            .state(&S::B1)
            .mark_trans(Transit::to(S::A1), 'a')
            .unwrap();
        builder
            .state(&S::B2)
            .mark_trans(Transit::to(S::A2), 'a')
            .unwrap();
        builder
            .state(&S::A)
            .accept_to(&S::B2)
            .tokenize(&"A".to_string());
        builder
            .state(&S::A1)
            .accept_to(&S::Start)
            .tokenize(&"A1".to_string());
        builder
            .state(&S::A2)
            .accept_to(&S::Start)
            .tokenize(&"A2".to_string());

        let cdfa: EncodedCDFA<String> = builder.build().unwrap();

        let input = "1a2a1a".to_string();
        let chars: Vec<char> = input.chars().collect();

        let lexer = lex::def_lexer();

        //exercise
        let tokens = lexer.lex(&chars[..], &cdfa).unwrap();

        //verify
        assert_eq!(
            tokens_string(&tokens),
            "\
A <- '1'
A1 <- 'a'
A <- '2'
A2 <- 'a'
A <- '1'
A1 <- 'a'
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
                TransitBuilder::to(S::A)
                    .consumer(ConsumerStrategy::All)
                    .build(),
                'a',
            )
            .unwrap()
            .default_to(
                &S::Start1,
                TransitBuilder::to(S::Start2)
                    .consumer(ConsumerStrategy::None)
                    .build(),
            )
            .unwrap();
        builder
            .mark_trans(
                &S::Start2,
                TransitBuilder::to(S::B)
                    .consumer(ConsumerStrategy::All)
                    .build(),
                'b',
            )
            .unwrap();
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

    #[test]
    fn large_range_transition() {
        //setup
        #[derive(PartialEq, Eq, Hash, Clone, Debug)]
        enum S {
            Start,
        }

        impl Data for S {
            fn to_string(&self) -> String {
                format!("{:?}", self)
            }
        }

        let mut builder: EncodedCDFABuilder<S, String> = EncodedCDFABuilder::new();
        builder.mark_start(&S::Start);
        builder
            .state(&S::Start)
            .mark_range(Transit::to(S::Start), '\0', '嶲')
            .unwrap()
            .accept()
            .tokenize(&"START".to_string());

        let cdfa: EncodedCDFA<String> = builder.build().unwrap();

        let input = "1234567890".to_string();
        let chars: Vec<char> = input.chars().collect();

        let lexer = lex::def_lexer();

        //exercise
        let tokens = lexer.lex(&chars[..], &cdfa).unwrap();

        //verify
        assert_eq!(tokens_string(&tokens), "START <- '1234567890'\n");
    }

    #[test]
    fn non_prefix_free_transitions() {
        //setup
        #[derive(PartialEq, Eq, Hash, Clone, Debug)]
        enum S {
            Start,
            A,
            AB,
            ABC,
        }

        impl Data for S {
            fn to_string(&self) -> String {
                format!("{:?}", self)
            }
        }

        let mut builder: EncodedCDFABuilder<S, String> = EncodedCDFABuilder::new();
        builder.mark_start(&S::Start);
        builder
            .state(&S::Start)
            .mark_trans(Transit::to(S::A), 'a')
            .unwrap()
            .mark_chain(Transit::to(S::AB), "ab".chars())
            .unwrap()
            .mark_chain(Transit::to(S::ABC), "abc".chars())
            .unwrap();

        builder
            .state(&S::A)
            .accept()
            .tokenize(&"A".to_string());

        builder
            .state(&S::AB)
            .accept()
            .tokenize(&"AB".to_string());

        builder
            .state(&S::ABC)
            .accept()
            .tokenize(&"ABC".to_string());

        let cdfa: EncodedCDFA<String> = builder.build().unwrap();

        let input = "aababca".to_string();
        let chars: Vec<char> = input.chars().collect();

        let lexer = lex::def_lexer();

        //exercise
        let tokens = lexer.lex(&chars[..], &cdfa).unwrap();

        //verify
        assert_eq!(tokens_string(&tokens), "\
A <- 'a'
AB <- 'ab'
ABC <- 'abc'
A <- 'a'
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
