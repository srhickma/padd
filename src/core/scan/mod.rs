use std::collections::HashMap;

pub mod maximal_munch;

pub trait Scanner {
    fn scan<'a, 'b, 'c>(&self, input: &'a str, dfa: &'b DFA<'c>) -> Vec<Token>;
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
pub type State<'a> = &'a str;

static NULL_STATE: &'static str = "";

pub struct DFA<'a> {
    pub alphabet: &'a str,
    pub start: State<'a>,
    pub td: Box<TransitionDelta<'a>>,
}

impl<'a> DFA<'a> {
    fn has_transition(&self, c: char, state: State<'a>) -> bool {
        self.alphabet.chars().any(|x| c == x) && self.transition(state, c) != ""
    }
    fn accepts(&self, state: State) -> bool {
        self.td.tokenize(state).len() > 0
    }
    fn transition(&self, state: State<'a>, c: char) -> State<'a> {
        self.td.transition(state, c)
    }
    fn tokenize(&self, state: State) -> Kind {
        self.td.tokenize(state)
    }
}

pub trait TransitionDelta<'a> {
    fn transition(&self, state: State<'a>, c: char) -> State<'a>;
    fn tokenize(&self, state: State) -> Kind;
}

pub struct CompileTransitionDelta {
    pub delta: fn(State, char) -> State,
    pub tokenizer: fn(State) -> &str,
}

impl<'a> TransitionDelta<'a> for CompileTransitionDelta {
    fn transition(&self, state: State<'a>, c: char) -> State<'a> {
        (self.delta)(state, c)
    }
    fn tokenize(&self, state: State) -> Kind {
        (self.tokenizer)(state).to_string()
    }
}

pub struct RuntimeTransitionDelta<'a> {
    pub delta: HashMap<State<'a>, HashMap<char, State<'a>>>,
    pub tokenizer: HashMap<State<'a>, &'a str>,
}

impl<'a> TransitionDelta<'a> for RuntimeTransitionDelta<'a> {
    fn transition(&self, state: State<'a>, c: char) -> State<'a> {
        match self.delta.get(state) {
            Some(hm) => match hm.get(&c) {
                Some(s) => s,
                None => NULL_STATE,
            },
            None => NULL_STATE,
        }
    }
    fn tokenize(&self, state: State) -> Kind {
        match self.tokenizer.get(state) {
            Some(s) => s.to_string(),
            None => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_binary() {
        //setup
        let alphabet = "01";
        let start: State = "start";
        let accepting: [State; 2] = ["0", "not0"];
        let delta: fn(State, char) -> State = |state, c| match (state, c) {
            ("start", '0') => "0",
            ("start", '1') => "not0",
            ("not0", _) => "not0",
            (&_, _) => "",
        };
        let tokenizer: fn(State) -> &str = |state| match state {
            "0" => "ZERO",
            "not0" => "NZ",
            _ => "",
        };

        let dfa = DFA{
            alphabet: &alphabet,
            start,
            td: Box::new(CompileTransitionDelta{
                delta,
                tokenizer,
            }),
        };

        let input = "000011010101";

        let scanner = def_scanner();

        //execute
        let tokens = scanner.scan(&input, &dfa);

        //verify
        let ts = tokens_string(&tokens);
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
        let alphabet = "{} \t\n";
        let start: State = "start";
        let accepting: [State; 3] = ["lbr", "rbr", "ws"];
        let delta: fn(State, char) -> State = |state, c| match (state, c) {
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
        let tokenizer: fn(State) -> &str = |state| match state {
            "lbr" => "LBRACKET",
            "rbr" => "RBRACKET",
            "ws" => "WHITESPACE",
            _ => "",
        };

        let dfa = DFA{
            alphabet: &alphabet,
            start,
            td: Box::new(CompileTransitionDelta{
                delta,
                tokenizer,
            }),
        };

        let input = "  {{\n}{}{} \t{} \t{}}";

        let scanner = def_scanner();

        //execute
        let tokens = scanner.scan(&input, &dfa);

        //verify
        let ts = tokens_string(&tokens);
        println!("{}", ts);
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

    fn tokens_string(tokens: &Vec<Token>) -> String {
        let mut res = String::new();

        for token in tokens {
            res = format!("{}\nkind={} lexeme={}", res, token.kind, token.lexeme)
        }
        return res;
    }
}