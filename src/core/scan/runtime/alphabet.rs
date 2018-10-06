use std::collections::HashSet;

pub trait Alphabet {
    fn contains(&self, c: char) -> bool;
}

pub struct HashedAlphabet {
    alphabet: HashSet<char>
}

impl HashedAlphabet {
    pub fn new() -> HashedAlphabet {
        HashedAlphabet {
            alphabet: HashSet::new()
        }
    }

    pub fn insert(&mut self, c: char) {
        self.alphabet.insert(c);
    }
}

impl Alphabet for HashedAlphabet {
    fn contains(&self, c: char) -> bool {
        self.alphabet.contains(&c)
    }
}
