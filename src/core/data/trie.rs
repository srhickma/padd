use std::{error, fmt};

/// Trie: A simple trie data structure, supporting generic value types and byte-slice keys.
pub struct Trie<Value> {
    root: HeapNode<Value>,
}

impl<Value> Trie<Value> {
    /// Returns a new empty trie.
    pub fn new() -> Self {
        Self { root: new_node() }
    }

    /// Inserts `value` at position `key` in the trie, or returns an error if there is already a
    /// value associated with that key.
    pub fn insert(&mut self, key: &[u8], value: Value) -> Result<(), Error> {
        self.root.insert(KeySeq::from(key), value)
    }

    /// Removes the value associated with `key` in the trie, if such a value exists.
    #[allow(dead_code)]
    pub fn remove(&mut self, key: &[u8]) {
        self.root.remove(KeySeq::from(key));
    }

    /// Returns a reference to the value associated with `key` in the trie, or `None` if there is
    /// no such value.
    #[allow(dead_code)]
    pub fn search(&self, key: &[u8]) -> Option<&Value> {
        self.root.search(KeySeq::from(key))
    }

    /// Returns the value associated with the longest prefix of `key` for which a value exists, as
    /// well as the length of the key associated with the value. `None` is returned if no prefix of
    /// `key` corresponds to a value in the trie.
    pub fn longest_match(&self, key: &[u8]) -> Option<(&Value, usize)> {
        self.root.longest_match(KeySeq::from(key), None)
    }
}

/// The number of key bits to multiplex on during traversal of each trie node.
const MUX_WIDTH: u8 = 4;

/// The maximum number of child nodes for each internal trie node.
/// It must be the case that TREE_WIDTH = MUX_WIDTH^2.
const TREE_WIDTH: usize = 16;

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
/// * `children` - an array of child nodes for this node, or `None` for non-existent children.
/// * `value` - the value stored at this node of the trie, or `None` if this node is internal.
struct Node<Value> {
    children: [Option<HeapNode<Value>>; TREE_WIDTH],
    value: Option<Value>,
}

impl<Value> Node<Value> {
    /// Returns a new leaf node with value `value`.
    fn new(value: Option<Value>) -> Self {
        Self {
            children: Default::default(),
            value,
        }
    }

    /// Inserts `value` in the sub-trie rooted at this node at `key`.
    fn insert(&mut self, key: KeySeq, value: Value) -> Result<(), Error> {
        if key.is_empty() {
            if self.value.is_some() {
                return Err(Error::DuplicateErr);
            }

            self.value = Some(value);
            return Ok(());
        }

        self.children[key.mux()]
            .get_or_insert_with(new_node)
            .insert(key.next(), value)
    }

    /// Removes the value associated with `key` in the sub-trie rooted at this node, if one exists.
    fn remove(&mut self, key: KeySeq) {
        if key.is_empty() {
            self.value = None;
        }

        if let Some(node) = &mut self.children[key.mux()] {
            node.remove(key.next());
        }
    }

    /// Returns a reference to the value associated with `key` in the sub-trie rooted at this node,
    /// or `None` if there is no such value.
    fn search(&self, key: KeySeq) -> Option<&Value> {
        if key.is_empty() {
            return self.value.as_ref();
        }

        match &self.children[key.mux()] {
            Some(node) => node.search(key.next()),
            None => None,
        }
    }

    /// Returns the value associated with the longest prefix of `key` in the sub-trie rooted at this
    /// node for which a value exists, as well as the length of the key associated with the value.
    /// `None` is returned if no prefix of `key` corresponds to a value in the sub-trie.
    fn longest_match<'value, 'scope: 'value>(
        &'scope self,
        key: KeySeq,
        mut last_match: Option<(&'value Value, usize)>,
    ) -> Option<(&'value Value, usize)> {
        if let Some(value) = self.value.as_ref() {
            last_match = Some((value, key.consumed()));
        }

        if key.is_empty() {
            return last_match;
        }

        match &self.children[key.mux()] {
            Some(node) => node.longest_match(key.next(), last_match),
            None => last_match,
        }
    }
}

/// Key Sequence: Represents a suffix of a trie key.
///
/// # Fields
///
/// * `key` - a slice of the remaining key bytes.
/// * `idx` - the index of the first bit of the key suffix in the first byte of the key slice.
/// * `consumed` - the number of bytes of the original key which have been consumed.
struct KeySeq<'key> {
    key: &'key [u8],
    idx: u8,
    consumed: usize,
}

impl<'key> From<&'key [u8]> for KeySeq<'key> {
    fn from(key: &'key [u8]) -> KeySeq<'key> {
        Self {
            key,
            idx: 0,
            consumed: 0,
        }
    }
}

impl<'key> KeySeq<'key> {
    /// Returns the next multiplexed child node index corresponding with the key sequence.
    fn mux(&self) -> usize {
        let shift_offset = 8 - MUX_WIDTH - self.idx;
        let mask = (TREE_WIDTH as u8 - 1) << shift_offset;
        ((self.key[0] & mask) >> shift_offset) as usize
    }

    /// Consumes the head multiplexing bits of the sequence, and returns the tail key sequence.
    fn next(mut self) -> Self {
        if self.idx == 8 - MUX_WIDTH {
            self.key = &self.key[1..];
            self.consumed += 1;
        }
        self.idx = (self.idx + MUX_WIDTH) % 8;

        self
    }

    /// Returns the number of bytes of the key consumed so far.
    fn consumed(&self) -> usize {
        self.consumed
    }

    /// Returns true if the key has been entirely consumed, false otherwise.
    fn is_empty(&self) -> bool {
        self.key.is_empty()
    }
}

/// Error: Represents an error encountered when using a trie.
///
/// # Types
///
/// * `DuplicateErr` - indicates that a trie already stores a value for a particular key.
#[derive(Debug, PartialEq)]
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
        let mut trie: Trie<u32> = Trie::new();

        trie.insert("k1".as_bytes(), 1).unwrap();
        trie.insert("k2".as_bytes(), 2).unwrap();
        trie.insert("k3".as_bytes(), 3).unwrap();
        trie.insert("k4".as_bytes(), 4).unwrap();
        trie.insert("k5".as_bytes(), 5).unwrap();
        trie.insert("k6".as_bytes(), 6).unwrap();

        assert_eq!(trie.insert("k1".as_bytes(), 11), Err(Error::DuplicateErr));
        assert_eq!(trie.insert("k2".as_bytes(), 12), Err(Error::DuplicateErr));
        assert_eq!(trie.insert("k3".as_bytes(), 13), Err(Error::DuplicateErr));
        assert_eq!(trie.insert("k4".as_bytes(), 14), Err(Error::DuplicateErr));
        assert_eq!(trie.insert("k5".as_bytes(), 15), Err(Error::DuplicateErr));
        assert_eq!(trie.insert("k6".as_bytes(), 16), Err(Error::DuplicateErr));

        assert_eq!(trie.insert("k0".as_bytes(), 0), Ok(()));

        assert_eq!(trie.search("k0".as_bytes()), Some(&0));
        assert_eq!(trie.search("k1".as_bytes()), Some(&1));
        assert_eq!(trie.search("k2".as_bytes()), Some(&2));
        assert_eq!(trie.search("k3".as_bytes()), Some(&3));
        assert_eq!(trie.search("k4".as_bytes()), Some(&4));
        assert_eq!(trie.search("k5".as_bytes()), Some(&5));
        assert_eq!(trie.search("k6".as_bytes()), Some(&6));
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
        let mut trie: Trie<u32> = Trie::new();

        trie.insert("abc".as_bytes(), 1).unwrap();
        trie.insert("abcd".as_bytes(), 2).unwrap();
        trie.insert("abcdefgh".as_bytes(), 3).unwrap();
        trie.insert("a".as_bytes(), 4).unwrap();
        trie.insert("ab".as_bytes(), 5).unwrap();
        trie.insert("".as_bytes(), 6).unwrap();

        assert_eq!(trie.search("abc".as_bytes()), Some(&1));
        assert_eq!(trie.search("abcd".as_bytes()), Some(&2));
        assert_eq!(trie.search("abcdefgh".as_bytes()), Some(&3));
        assert_eq!(trie.search("a".as_bytes()), Some(&4));
        assert_eq!(trie.search("ab".as_bytes()), Some(&5));
        assert_eq!(trie.search("".as_bytes()), Some(&6));

        assert_eq!(trie.search("b".as_bytes()), None);
        assert_eq!(trie.search("c".as_bytes()), None);
        assert_eq!(trie.search("abcde".as_bytes()), None);
        assert_eq!(trie.search("xyz".as_bytes()), None);
        assert_eq!(trie.search("bc".as_bytes()), None);
    }

    #[test]
    fn longest_match() {
        // TODO
    }
}
