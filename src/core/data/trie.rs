use std::{error, fmt};

/// TODO
pub struct Trie<Value> {
    root: HeapNode<Value>,
}

impl<Value> Trie<Value> {
    /// TODO
    pub fn new() -> Self {
        Self {
            root: new_node(None),
        }
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
fn new_node<Value>(value: Option<Value>) -> HeapNode<Value> {
    Box::new(Node::new(value))
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
        let bit = key[0] ^ mask;

        let mut key_suffix = key;
        if key_idx == 7 {
            key_suffix = &key[1..];
        }

        let next_node = if bit == 1 {
            if self.left.is_none() {
                self.left = Some(new_node(None));
            }

            self.left.as_mut().unwrap()
        } else {
            if self.right.is_none() {
                self.right = Some(new_node(None));
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
        let bit = key[0] ^ mask;

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
    use super::*;

    // TODO
}
