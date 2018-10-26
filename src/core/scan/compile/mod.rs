use core::scan::Token;
use core::scan::Kind;
use core::scan::Error;

mod maximal_munch;

pub trait Scanner<State: PartialEq + Clone>: 'static + Send + Sync {
    fn scan<'a, 'b>(&self, input: &'a str, dfa: &'b DFA<State>) -> Result<Vec<Token<String>>, Error>;
}

pub fn def_scanner<State: PartialEq + Clone>() -> Box<Scanner<State>> {
    Box::new(maximal_munch::MaximalMunchScanner)
}

pub struct DFA<State: PartialEq + Clone> {
    pub alphabet: String,
    pub start: State,
    pub td: Box<TransitionDelta<State>>,
}

impl<State: PartialEq + Clone> DFA<State> {
    fn has_transition(&self, c: char, state: &State) -> bool {
        self.alphabet.chars().any(|x| c == x) && self.transition(state, c) != self.td.fail_state()
    }

    fn accepts(&self, state: &State) -> bool {
        self.td.tokenize(state).is_some()
    }

    fn transition(&self, state: &State, c: char) -> State {
        self.td.transition(state, c)
    }

    fn tokenize(&self, state: &State) -> Option<Kind> {
        self.td.tokenize(state)
    }
}

pub trait TransitionDelta<State> {
    fn transition(&self, state: &State, c: char) -> State;
    fn tokenize(&self, state: &State) -> Option<Kind>;
    fn fail_state(&self) -> State;
}

pub struct CompileTransitionDelta<State> {
    delta: fn(State, char) -> State,
    tokenizer: fn(State) -> String,
    fail_state: State,
}

impl<State: PartialEq + Clone> TransitionDelta<State> for CompileTransitionDelta<State> {
    fn transition<'a>(&'a self, state: &State, c: char) -> State {
        (self.delta)(state.clone(), c)
    }

    fn tokenize(&self, state: &State) -> Option<Kind> {
        let kind = (self.tokenizer)(state.clone());
        if kind.is_empty() {
            None
        } else {
            Some(kind)
        }
    }

    fn fail_state(&self) -> State {
        self.fail_state.clone()
    }
}

impl<State: PartialEq + Clone> CompileTransitionDelta<State> {
    pub fn build<'a>(delta: fn(State, char) -> State, tokenizer: fn(State) -> String, fail_state: State) -> CompileTransitionDelta<State> {
        CompileTransitionDelta {
            delta,
            tokenizer,
            fail_state,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_binary() {
        //setup
        #[derive(PartialEq, Clone)]
        enum S {
            START,
            ZERO,
            NOTZERO,
            FAIL,
        }

        let alphabet = "01".to_string();
        let delta: fn(S, char) -> S = |state, c| match (state, c) {
            (S::START, '0') => S::ZERO,
            (S::START, '1') => S::NOTZERO,
            (S::NOTZERO, _) => S::NOTZERO,
            (_, _) => S::FAIL,
        };
        let tokenizer: fn(S) -> String = |state| match state {
            S::ZERO => "ZERO",
            S::NOTZERO => "NZ",
            _ => "",
        }.to_string();

        let dfa = DFA {
            alphabet,
            start: S::START,
            td: Box::new(CompileTransitionDelta::build(delta, tokenizer, S::FAIL)),
        };

        let input = "000011010101";

        let scanner = def_scanner();

        //exercise
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
        #[derive(PartialEq, Clone)]
        enum S {
            START,
            WS,
            LBR,
            RBR,
            FAIL,
        }

        let alphabet = "{} \t\n".to_string();
        let delta: fn(S, char) -> S = |state, c| match (state, c) {
            (S::START, ' ') => S::WS,
            (S::START, '\t') => S::WS,
            (S::START, '\n') => S::WS,
            (S::START, '{') => S::LBR,
            (S::START, '}') => S::RBR,
            (S::WS, ' ') => S::WS,
            (S::WS, '\t') => S::WS,
            (S::WS, '\n') => S::WS,
            (_, _) => S::FAIL,
        };
        let tokenizer: fn(S) -> String = |state| match state {
            S::LBR => "LBRACKET",
            S::RBR => "RBRACKET",
            S::WS => "WHITESPACE",
            _ => "",
        }.to_string();

        let dfa = DFA {
            alphabet,
            start: S::START,
            td: Box::new(CompileTransitionDelta::build(delta, tokenizer, S::FAIL)),
        };

        let input = "  {{\n}{}{} \t{} \t{}}";

        let scanner = def_scanner();

        //exercise
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
        #[derive(PartialEq, Clone)]
        enum S {
            START,
            WS,
            LBR,
            RBR,
            FAIL,
        }

        let alphabet = "{} \t\n".to_string();
        let delta: fn(S, char) -> S = |state, c| match (state, c) {
            (S::START, ' ') => S::WS,
            (S::START, '\t') => S::WS,
            (S::START, '\n') => S::WS,
            (S::START, '{') => S::LBR,
            (S::START, '}') => S::RBR,
            (S::WS, ' ') => S::WS,
            (S::WS, '\t') => S::WS,
            (S::WS, '\n') => S::WS,
            (_, _) => S::FAIL,
        };
        let tokenizer: fn(S) -> String = |state| match state {
            S::LBR => "LBRACKET",
            S::RBR => "RBRACKET",
            S::WS => "_",
            _ => "",
        }.to_string();

        let dfa = DFA {
            alphabet,
            start: S::START,
            td: Box::new(CompileTransitionDelta::build(delta, tokenizer, S::FAIL)),
        };

        let input = "  {{\n}{}{} \t{} \t{}}";

        let scanner = def_scanner();

        //exercise
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
        #[derive(PartialEq, Clone)]
        enum S {
            START,
            WS,
            LBR,
            RBR,
            FAIL,
        }

        let alphabet = "{} \t\n".to_string();
        let delta: fn(S, char) -> S = |state, c| match (state, c) {
            (S::START, ' ') => S::WS,
            (S::START, '\t') => S::WS,
            (S::START, '\n') => S::WS,
            (S::START, '{') => S::LBR,
            (S::START, '}') => S::RBR,
            (S::WS, ' ') => S::WS,
            (S::WS, '\t') => S::WS,
            (S::WS, '\n') => S::WS,
            (_, _) => S::FAIL,
        };
        let tokenizer: fn(S) -> String = |state| match state {
            S::LBR => "LBRACKET",
            S::RBR => "RBRACKET",
            S::WS => "_",
            _ => "",
        }.to_string();

        let dfa = DFA {
            alphabet,
            start: S::START,
            td: Box::new(CompileTransitionDelta::build(delta, tokenizer, S::FAIL)),
        };

        let input = "  {{\n}{}{} \tx{} \t{}}";

        let scanner = def_scanner();

        //exercise
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
        #[derive(PartialEq, Clone)]
        enum S {
            START,
            WS,
            LBR,
            RBR,
            FAIL,
        }

        let alphabet = "{} \t\n".to_string();
        let delta: fn(S, char) -> S = |state, c| match (state, c) {
            (S::START, ' ') => S::WS,
            (S::START, '\t') => S::WS,
            (S::START, '\n') => S::WS,
            (S::START, '{') => S::LBR,
            (S::START, '}') => S::RBR,
            (S::WS, ' ') => S::WS,
            (S::WS, '\t') => S::WS,
            (S::WS, '\n') => S::WS,
            (_, _) => S::FAIL,
        };
        let tokenizer: fn(S) -> String = |state| match state {
            S::LBR => "LBRACKET",
            S::RBR => "RBRACKET",
            S::WS => "_",
            _ => "",
        }.to_string();

        let dfa = DFA {
            alphabet,
            start: S::START,
            td: Box::new(CompileTransitionDelta::build(delta, tokenizer, S::FAIL)),
        };

        let input = "   {  {  {{{\t}}}\n {} }  }   { {}\n }   {  {  {{{\t}}}\n {} }  } xyz  { {}\n }   {  {  {{{\t}}}\n {} }  }   { {}\n } ";

        let scanner = def_scanner();

        //exercise
        let tokens = scanner.scan(&input, &dfa);

        //verify
        assert!(tokens.is_err());
        let err = tokens.err().unwrap();
        assert_eq!(err.sequence, "xyz  { {}\n");
        assert_eq!(err.line, 4);
        assert_eq!(err.character, 10);
    }

    fn tokens_string(tokens: &Vec<Token<String>>) -> String {
        let mut res = String::new();

        for token in tokens {
            res = format!("{}\nkind={} lexeme={}", res, token.kind, token.lexeme)
        }
        res
    }
}
