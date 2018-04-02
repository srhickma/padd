pub mod maximal_munch;

pub trait Scanner {
    fn scan<'a>(&self, input: &'a str, dfa: &'a DFA) -> Vec<Token>;
}

pub fn def_scanner() -> Box<Scanner> {
    return Box::new(maximal_munch::MaximalMunchScanner);
}

#[derive(PartialEq, Clone)]
pub struct Token {
    pub kind: Kind,
    pub lexeme: String,
}

pub type Kind = String;
pub type State<'a> = &'a str;

pub struct DFA<'a> {
    pub alphabet: &'a str,
    pub states: &'a [State<'a>],
    pub start: State<'a>,
    pub accepting: &'a [State<'a>],
    pub delta: fn(State, char) -> State,
    pub tokenizer: fn(State) -> &str,
}

impl<'a> DFA<'a> {
    fn has_transition(&self, c: char, state: State) -> bool {
        return self.alphabet.chars().any(|x| c == x) && self.transition(state, c) != "";
    }
    fn accepts(&self, state: State) -> bool {
        return self.accepting.contains(&state);
    }
    fn transition(&self, state: State<'a>, c: char) -> State<'a> {
        return (self.delta)(state, c);
    }
    fn tokenize(&self, state: State) -> Kind {
        return (self.tokenizer)(state).to_string();
    }
}