use std::collections::HashSet;

/// Alphabet: Trait representing a lexing alphabet.
pub trait Alphabet {
    /// Returns true if the alphabet contains `c`, false otherwise.
    fn contains(&self, c: char) -> bool;
}

/// Hashed Alphabet: Alphabet implementation using hash-based storage.
///
/// # Fields
///
/// * `alphabet` - internal implementation of alphabet using a hash set.
pub struct HashedAlphabet {
    alphabet: HashSet<char>,
}

impl HashedAlphabet {
    /// Returns a new empty alphabet.
    pub fn new() -> HashedAlphabet {
        HashedAlphabet {
            alphabet: HashSet::new(),
        }
    }

    /// Inserts character `c` into the alphabet.
    pub fn insert(&mut self, c: char) {
        self.alphabet.insert(c);
    }
}

impl Alphabet for HashedAlphabet {
    fn contains(&self, c: char) -> bool {
        self.alphabet.contains(&c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains() {
        //setup
        let mut alphabet = HashedAlphabet::new();

        //exercise/verify
        alphabet.insert('a');
        assert!(alphabet.contains('a'));
        assert!(!alphabet.contains('c'));
        assert!(!alphabet.contains('e'));

        alphabet.insert('e');
        assert!(alphabet.contains('a'));
        assert!(!alphabet.contains('c'));
        assert!(alphabet.contains('e'));

        alphabet.insert('c');
        assert!(alphabet.contains('a'));
        assert!(alphabet.contains('c'));
        assert!(alphabet.contains('e'));
    }
}
