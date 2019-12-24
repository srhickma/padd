use std::{error, fmt};

/// TODO
pub struct Trie<Value> {
    root: HeapNode<Value>,
}

impl<Value> Trie<Value> {
    /// TODO
    pub fn new() -> Self {
        Self { root: new_node() }
    }

    /// TODO
    pub fn insert(&mut self, key: &[u8], value: Value) -> Result<(), Error> {
        self.root.insert(key, 0, value)
    }

    /// TODO
    pub fn remove(&mut self, key: &[u8]) {
        self.root.remove(key);
    }

    /// TODO
    pub fn search(&self, key: &[u8]) -> Option<&Value> {
        self.root.search(key, 0)
    }
}

/// Wrapper around boxed node.
type HeapNode<Value> = Box<Node<Value>>;

/// Helper function to create `HeapNode` objects (can't implement `new` for type alias).
fn new_node<Value>() -> HeapNode<Value> {
    Box::new(Node::new(None))
}

/// Node: Represents a tree-node in a trie.
///
/// # Type Parameters
///
/// * `Value` - the type of trie values.
///
/// # Fields
///
/// TODO
struct Node<Value> {
    left: Option<HeapNode<Value>>,
    right: Option<HeapNode<Value>>,
    value: Option<Value>,
}

// TODO investigate using a non-binary trie.
impl<Value> Node<Value> {
    /// Returns a new leaf node with value `value`.
    fn new(value: Option<Value>) -> Self {
        Self {
            left: None,
            right: None,
            value,
        }
    }

    /// TODO
    fn insert(&mut self, key: &[u8], key_idx: u8, value: Value) -> Result<(), Error> {
        if key.is_empty() {
            if self.value.is_some() {
                return Err(Error::DuplicateErr);
            }

            self.value = Some(value);
            return Ok(());
        }

        let shift_offset = 7 - key_idx;
        let mask = 1 << shift_offset;
        let bit = (key[0] & mask) >> shift_offset;

        let mut key_suffix = key;
        if key_idx == 7 {
            key_suffix = &key[1..];
        }

        let next_node = if bit == 1 {
            if self.left.is_none() {
                self.left = Some(new_node());
            }

            self.left.as_mut().unwrap()
        } else {
            if self.right.is_none() {
                self.right = Some(new_node());
            }

            self.right.as_mut().unwrap()
        };

        next_node.insert(key_suffix, (key_idx + 1) % 8, value)
    }

    /// TODO
    fn remove(&mut self, key: &[u8]) {
        panic!("Unimplemented!");
        // TODO
    }

    /// TODO
    fn search(&self, key: &[u8], key_idx: u8) -> Option<&Value> {
        if key.is_empty() {
            return self.value.as_ref();
        }

        let shift_offset = 7 - key_idx;
        let mask = 1 << shift_offset;
        let bit = (key[0] & mask) >> shift_offset;

        let mut key_suffix = key;
        if key_idx == 7 {
            key_suffix = &key[1..];
        }

        let next_node = if bit == 1 {
            match &self.left {
                Some(left) => left,
                None => return None,
            }
        } else {
            match &self.right {
                Some(right) => right,
                None => return None,
            }
        };

        next_node.search(key_suffix, (key_idx + 1) % 8)
    }
}

/// TODO
#[derive(Debug)]
pub enum Error {
    DuplicateErr,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::DuplicateErr => write!(f, "Duplicate key"),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

#[cfg(test)]
mod tests {
    extern crate uuid;

    use super::*;

    use {
        self::uuid::Uuid,
        std::{collections::HashMap, str},
    };

    #[test]
    fn simple_inserts() {
        let mut trie: Trie<u32> = Trie::new();

        trie.insert("something".as_bytes(), 1).unwrap();
        trie.insert("something else".as_bytes(), 2).unwrap();
        trie.insert("sab".as_bytes(), 3).unwrap();

        assert_eq!(trie.search("something".as_bytes()), Some(&1));
        assert_eq!(trie.search("something else".as_bytes()), Some(&2));
        assert_eq!(trie.search("sab".as_bytes()), Some(&3));
        assert_eq!(trie.search("something else more".as_bytes()), None);
        assert_eq!(trie.search("some".as_bytes()), None);
        assert_eq!(trie.search("s".as_bytes()), None);
        assert_eq!(trie.search("a".as_bytes()), None);
        assert_eq!(trie.search("project".as_bytes()), None);
    }

    #[test]
    fn many_inserts() {
        let mut trie: Trie<u32> = Trie::new();
        let mut map: HashMap<String, u32> = HashMap::new();

        for i in 0..10000 {
            let key = Uuid::new_v4().to_string();

            trie.insert(key.as_bytes(), i).unwrap();
            map.insert(key, i);
        }

        for (key, val) in map.iter() {
            assert_eq!(trie.search(key.as_bytes()), Some(val));
        }

        for _ in 0..10000 {
            let key = Uuid::new_v4().to_string();

            assert_eq!(trie.search(key.as_bytes()), None);
        }
    }

    #[test]
    fn exhaustive_searching() {
        let mut trie: Trie<usize> = Trie::new();
        let mut map: HashMap<String, usize> = HashMap::new();

        // Insert some 8-bit values.
        for i in 32..=126 {
            let key = [i as u8];

            trie.insert(&key, i).unwrap();
            map.insert(str::from_utf8(&key).unwrap().to_string(), i);
        }

        // Insert some random values of differing lengths.
        for length in 6..15 {
            for i in 1..length * length * length {
                let mut key = Uuid::new_v4().to_string();
                key = key.chars().rev().collect();
                key.truncate(length);

                trie.insert(key.as_bytes(), i).unwrap();
                map.insert(key, i);
            }
        }

        // Trie contains correct values for all inserted pairs.
        for (key, val) in map.iter() {
            assert_eq!(trie.search(key.as_bytes()), Some(val));
        }

        // Trie does not contain any 16-bit entries.
        for i in 0..255 {
            for j in 0..255 {
                let key = [i as u8, j as u8];

                assert_eq!(trie.search(&key), None);
            }
        }

        // Trie does not contain arbitrary random values.
        for _ in 0..10000 {
            let key = Uuid::new_v4().to_string();

            assert_eq!(trie.search(key.as_bytes()), None);
        }
    }

    #[test]
    fn insert_duplicate() {
        // TODO
    }

    #[test]
    fn removal() {
        // TODO
    }

    #[test]
    fn heavy_usage() {
        // TODO
    }

    #[test]
    fn non_prefix_free() {
        // TODO
    }
}
