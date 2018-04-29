use std::collections::HashMap;

pub mod maximal_munch;

pub trait Scanner {
    fn scan<'a, 'b>(&self, input: &'a str, dfa: &'b DFA) -> Result<Vec<Token>, ScanningError>;
}

pub fn def_scanner() -> Box<Scanner> {
    Box::new(maximal_munch::MaximalMunchScanner)
}

#[derive(PartialEq, Clone)]
pub struct Token {
    pub kind: Kind,
    pub lexeme: String,
}

impl Token {
    pub fn to_string(&self) -> String {
        format!("{} <- '{}'", self.kind, self.lexeme.replace('\n', "\\n").replace('\t', "\\t"))
    }
}

pub type Kind = String;
pub type State = String;

lazy_static! {
    static ref NULL_STATE: String = String::new();
}

pub struct DFA {
    pub alphabet: String,
    pub start: State,
    pub td: Box<TransitionDelta>,
}

impl DFA {
    fn has_transition(&self, c: char, state: &State) -> bool {
        self.alphabet.chars().any(|x| c == x) && self.transition(state, c) != ""
    }
    fn accepts(&self, state: &State) -> bool {
        self.td.tokenize(state).len() > 0
    }
    fn transition<'a>(&'a self, state: &'a State, c: char) -> &'a State {
        self.td.transition(state, c)
    }
    fn tokenize(&self, state: &State) -> Kind {
        self.td.tokenize(state)
    }
}

pub trait TransitionDelta {
    fn transition<'a>(&'a self, state: &'a State, c: char) -> &'a State;
    fn tokenize(&self, state: &State) -> Kind;
}

pub struct CompileTransitionDelta {
    pub state_map: HashMap<String, String>,
    pub delta: fn(&str, char) -> &str,
    pub tokenizer: fn(&str) -> &str,
}

impl TransitionDelta for CompileTransitionDelta {
    fn transition<'a>(&'a self, state: &'a State, c: char) -> &'a State {
        self.state_map.get(&(self.delta)(&state[..], c).to_string()).unwrap()
    }
    fn tokenize(&self, state: &State) -> Kind {
        (self.tokenizer)(&state[..]).to_string()
    }
}

impl CompileTransitionDelta {
    pub fn build(states: &[&str], delta: fn(&str, char) -> &str, tokenizer: fn(&str) -> &str) -> CompileTransitionDelta {
        let mut state_map = HashMap::new();
        for state in states {
            state_map.insert(state.to_string(), state.to_string());
        }
        CompileTransitionDelta{
            state_map,
            delta,
            tokenizer,
        }
    }
}

pub struct RuntimeTransitionDelta {
    pub delta: HashMap<State, HashMap<char, State>>,
    pub tokenizer: HashMap<State, Kind>,
}

impl TransitionDelta for RuntimeTransitionDelta {
    fn transition<'a>(&'a self, state: &'a State, c: char) -> &'a State {
        match self.delta.get(state) {
            Some(hm) => match hm.get(&c) {
                Some(s) => s,
                None => match hm.get(&'_') {
                    Some(s) => s,
                    None => &NULL_STATE,
                },
            },
            None => &NULL_STATE,
        }
    }
    fn tokenize(&self, state: &State) -> Kind {
        match self.tokenizer.get(state) {
            Some(s) => s.clone(),
            None => String::new(),
        }
    }
}

#[derive(Debug)]
pub struct ScanningError {
    pub sequence: String,
    pub character: usize,
    pub line: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_binary() {
        //setup
        let alphabet = "01".to_string();
        let states: [&str; 4] = ["start", "0", "not0", ""];
        let start: State = "start".to_string();
        let delta: fn(&str, char) -> &str = |state, c| match (state, c) {
            ("start", '0') => "0",
            ("start", '1') => "not0",
            ("not0", _) => "not0",
            (&_, _) => "",
        };
        let tokenizer: fn(&str) -> &'static str = |state| match state {
            "0" => "ZERO",
            "not0" => "NZ",
            _ => "",
        };

        let dfa = DFA{
            alphabet,
            start,
            td: Box::new(CompileTransitionDelta::build(&states, delta, tokenizer)),
        };

        let input = "000011010101";

        let scanner = def_scanner();

        //execute
        let tokens = scanner.scan(&input, &dfa);

        //verify
        let ts = tokens_string(&tokens.unwrap());
        assert_eq!(ts, "
kind=ZERO lexeme=0
kind=ZERO lexeme=0
kind=ZERO lexeme=0
kind=ZERO lexeme=0
kind=NZ lexeme=11010101"
        );
    }

    #[test]
    fn scan_brackets() {
        //setup
        let alphabet = "{} \t\n".to_string();
        let states: [&str; 5] = ["start", "lbr", "rbr", "ws", ""];
        let start: State = "start".to_string();
        let delta: fn(&str, char) -> &str = |state, c| match (state, c) {
            ("start", ' ') => "ws",
            ("start", '\t') => "ws",
            ("start", '\n') => "ws",
            ("start", '{') => "lbr",
            ("start", '}') => "rbr",
            ("ws", ' ') => "ws",
            ("ws", '\t') => "ws",
            ("ws", '\n') => "ws",
            (&_, _) => "",
        };
        let tokenizer: fn(&str) -> &'static str = |state| match state {
            "lbr" => "LBRACKET",
            "rbr" => "RBRACKET",
            "ws" => "WHITESPACE",
            _ => "",
        };

        let dfa = DFA{
            alphabet,
            start,
            td: Box::new(CompileTransitionDelta::build(&states, delta, tokenizer)),
        };

        let input = "  {{\n}{}{} \t{} \t{}}";

        let scanner = def_scanner();

        //execute
        let tokens = scanner.scan(&input, &dfa);

        //verify
        let ts = tokens_string(&tokens.unwrap());
        assert_eq!(ts, "
kind=WHITESPACE lexeme=  \nkind=LBRACKET lexeme={
kind=LBRACKET lexeme={
kind=WHITESPACE lexeme=\n
kind=RBRACKET lexeme=}
kind=LBRACKET lexeme={
kind=RBRACKET lexeme=}
kind=LBRACKET lexeme={
kind=RBRACKET lexeme=}
kind=WHITESPACE lexeme= \t
kind=LBRACKET lexeme={
kind=RBRACKET lexeme=}
kind=WHITESPACE lexeme= \t
kind=LBRACKET lexeme={
kind=RBRACKET lexeme=}
kind=RBRACKET lexeme=}"
        );
    }

    #[test]
    fn scan_ignore() {
        //setup
        let alphabet = "{} \t\n".to_string();
        let states: [&str; 5] = ["start", "lbr", "rbr", "ws", ""];
        let start: State = "start".to_string();
        let delta: fn(&str, char) -> &str = |state, c| match (state, c) {
            ("start", ' ') => "ws",
            ("start", '\t') => "ws",
            ("start", '\n') => "ws",
            ("start", '{') => "lbr",
            ("start", '}') => "rbr",
            ("ws", ' ') => "ws",
            ("ws", '\t') => "ws",
            ("ws", '\n') => "ws",
            (&_, _) => "",
        };
        let tokenizer: fn(&str) -> &'static str = |state| match state {
            "lbr" => "LBRACKET",
            "rbr" => "RBRACKET",
            "ws" => "_",
            _ => "",
        };

        let dfa = DFA{
            alphabet,
            start,
            td: Box::new(CompileTransitionDelta::build(&states, delta, tokenizer)),
        };

        let input = "  {{\n}{}{} \t{} \t{}}";

        let scanner = def_scanner();

        //execute
        let tokens = scanner.scan(&input, &dfa);

        //verify
        let ts = tokens_string(&tokens.unwrap());
        assert_eq!(ts, "
kind=LBRACKET lexeme={
kind=LBRACKET lexeme={
kind=RBRACKET lexeme=}
kind=LBRACKET lexeme={
kind=RBRACKET lexeme=}
kind=LBRACKET lexeme={
kind=RBRACKET lexeme=}
kind=LBRACKET lexeme={
kind=RBRACKET lexeme=}
kind=LBRACKET lexeme={
kind=RBRACKET lexeme=}
kind=RBRACKET lexeme=}"
        );
    }

    #[test]
    fn scan_fail_simple() {
        //setup
        let alphabet = "{} \t\n".to_string();
        let states: [&str; 5] = ["start", "lbr", "rbr", "ws", ""];
        let start: State = "start".to_string();
        let delta: fn(&str, char) -> &str = |state, c| match (state, c) {
            ("start", ' ') => "ws",
            ("start", '\t') => "ws",
            ("start", '\n') => "ws",
            ("start", '{') => "lbr",
            ("start", '}') => "rbr",
            ("ws", ' ') => "ws",
            ("ws", '\t') => "ws",
            ("ws", '\n') => "ws",
            (&_, _) => "",
        };
        let tokenizer: fn(&str) -> &'static str = |state| match state {
            "lbr" => "LBRACKET",
            "rbr" => "RBRACKET",
            "ws" => "_",
            _ => "",
        };

        let dfa = DFA{
            alphabet,
            start,
            td: Box::new(CompileTransitionDelta::build(&states, delta, tokenizer)),
        };

        let input = "  {{\n}{}{} \tx{} \t{}}";

        let scanner = def_scanner();

        //execute
        let tokens = scanner.scan(&input, &dfa);

        //verify
        assert!(tokens.is_err());
        let err = tokens.err().unwrap();
        assert_eq!(err.sequence, "x{} \t{}}");
        assert_eq!(err.line, 2);
        assert_eq!(err.character, 8);
    }

    #[test]
    fn scan_fail_complex() {
        //setup
        let alphabet = "{} \t\n".to_string();
        let states: [&str; 5] = ["start", "lbr", "rbr", "ws", ""];
        let start: State = "start".to_string();
        let delta: fn(&str, char) -> &str = |state, c| match (state, c) {
            ("start", ' ') => "ws",
            ("start", '\t') => "ws",
            ("start", '\n') => "ws",
            ("start", '{') => "lbr",
            ("start", '}') => "rbr",
            ("ws", ' ') => "ws",
            ("ws", '\t') => "ws",
            ("ws", '\n') => "ws",
            (&_, _) => "",
        };
        let tokenizer: fn(&str) -> &'static str = |state| match state {
            "lbr" => "LBRACKET",
            "rbr" => "RBRACKET",
            "ws" => "_",
            _ => "",
        };

        let dfa = DFA{
            alphabet,
            start,
            td: Box::new(CompileTransitionDelta::build(&states, delta, tokenizer)),
        };

        let input = "   {  {  {{{\t}}}\n {} }  }   { {}\n }   {  {  {{{\t}}}\n {} }  } xyz  { {}\n }   {  {  {{{\t}}}\n {} }  }   { {}\n } ";

        let scanner = def_scanner();

        //execute
        let tokens = scanner.scan(&input, &dfa);

        //verify
        assert!(tokens.is_err());
        let err = tokens.err().unwrap();
        assert_eq!(err.sequence, "xyz  { {}\n");
        assert_eq!(err.line, 4);
        assert_eq!(err.character, 10);
    }

    fn tokens_string(tokens: &Vec<Token>) -> String {
        let mut res = String::new();

        for token in tokens {
            res = format!("{}\nkind={} lexeme={}", res, token.kind, token.lexeme)
        }
        return res;
    }
}