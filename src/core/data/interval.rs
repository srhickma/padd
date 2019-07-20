use std::{
    cmp,
    error,
    fmt,
    ops::{Range, RangeInclusive},
};

pub trait Bound: Ord + Clone {
    fn predecessor(&self) -> Self;
}

fn max_bound<B: Bound>(b1: &B, b2: &B) -> B {
    if *b1 > *b2 {
        b1.clone()
    } else {
        b2.clone()
    }
}

pub struct Interval<B: Bound> {
    start: B,
    end: B,
}

impl<B: Bound> From<Range<B>> for Interval<B> {
    fn from(range: Range<B>) -> Self {
        Interval {
            start: range.start,
            end: range.end.predecessor(),
        }
    }
}

impl<B: Bound> From<RangeInclusive<B>> for Interval<B> {
    fn from(range: RangeInclusive<B>) -> Self {
        Interval {
            start: range.start().clone(),
            end: range.end().clone(),
        }
    }
}

pub struct IntervalMap<Key: Bound, Value> {
    root: Option<HeapNode<Key, Value>>,
}

impl<Key: Bound, Value> IntervalMap<Key, Value> {
    pub fn new() -> Self {
        IntervalMap { root: None }
    }

    pub fn get(&self, key: &Key) -> Option<&Value> {
        match self.root {
            None => None,
            Some(ref root) => root.get(key),
        }
    }

    pub fn insert(&mut self, keys: Interval<Key>, value: Value) -> Result<(), Error> {
        if let Some(root) = &self.root {
            if root.overlaps(&keys) {
                return Err(Error::OverlapErr);
            }
        }

        self.root = Some(IntervalMap::insert_rec_opt(self.root.take(), keys, value));
        Ok(())
    }

    fn insert_rec(
        mut node: HeapNode<Key, Value>,
        keys: Interval<Key>,
        value: Value,
    ) -> HeapNode<Key, Value> {
        if keys.start <= node.keys.start {
            node.left = Some(IntervalMap::insert_rec_opt(node.left.take(), keys, value));
        } else {
            node.right = Some(IntervalMap::insert_rec_opt(node.right.take(), keys, value));
        }

        node.update_from_children();
        if node.needs_balance() {
            node = IntervalMap::balance(node)
        }
        node
    }

    fn insert_rec_opt(
        node_opt: Option<HeapNode<Key, Value>>,
        keys: Interval<Key>,
        value: Value,
    ) -> HeapNode<Key, Value> {
        if let Some(node) = node_opt {
            IntervalMap::insert_rec(node, keys, value)
        } else {
            new_node(keys, value)
        }
    }

    fn balance(mut node: HeapNode<Key, Value>) -> HeapNode<Key, Value> {
        enum Rotation {
            Right,
            Left,
        }

        let (rotation, double) = if node.left_height() > node.right_height() {
            let left_node = node.left.as_ref().unwrap();
            (Rotation::Right, left_node.left_height() < left_node.right_height())
        } else {
            let right_node = node.right.as_ref().unwrap();
            (Rotation::Left, right_node.right_height() < right_node.left_height())
        };

        match rotation {
            Rotation::Right => {
                if double {
                    node.left = Some(IntervalMap::left_rotation(node.left.take().unwrap()));
                }
                IntervalMap::right_rotation(node)
            },
            Rotation::Left => {
                if double {
                    node.right = Some(IntervalMap::right_rotation(node.right.take().unwrap()));
                }
                IntervalMap::left_rotation(node)
            },
        }
    }

    fn right_rotation(mut node: HeapNode<Key, Value>) -> HeapNode<Key, Value> {
        let mut left = node.left.take().unwrap();

        node.left = left.right.take();
        node.update_from_children();
        left.right = Some(node);
        left.update_from_children();

        left
    }

    fn left_rotation(mut node: HeapNode<Key, Value>) -> HeapNode<Key, Value> {
        let mut right = node.right.take().unwrap();

        node.right = right.left.take();
        node.update_from_children();
        right.left = Some(node);
        right.update_from_children();

        right
    }
}

type HeapNode<Key, Value> = Box<Node<Key, Value>>;

fn new_node<Key: Bound, Value>(keys: Interval<Key>, value: Value) -> HeapNode<Key, Value> {
    Box::new(Node::new(keys, value))
}

struct Node<Key: Bound, Value> {
    keys: Interval<Key>,
    value: Value,
    left: Option<Box<Node<Key, Value>>>,
    right: Option<Box<Node<Key, Value>>>,
    max_end: Key,
    height: u32,
}

impl<Key: Bound, Value> Node<Key, Value> {
    fn new(keys: Interval<Key>, value: Value) -> Self {
        Node {
            max_end: keys.end.clone(),
            keys,
            value,
            left: None,
            right: None,
            height: 1,
        }
    }

    fn update_from_children(&mut self) {
        self.height = cmp::max(self.left_height(), self.right_height()) + 1;

        if let Some(left) = &self.left {
            self.max_end = max_bound(&self.keys.end, &left.max_end);
        }
        if let Some(right) = &self.right {
            self.max_end = max_bound(&self.max_end, &right.max_end);
        }
    }

    fn get(&self, key: &Key) -> Option<&Value> {
        if *key > self.max_end {
            return None;
        }

        if self.keys.start <= *key && *key <= self.keys.end {
            return Some(&self.value);
        }

        if let Some(value) = self.left.as_ref().map_or(None, |left| left.get(key)) {
            return Some(value);
        }

        if *key < self.keys.start {
            return None;
        }

        self.right.as_ref().map_or(None, |right| right.get(key))
    }

    fn overlaps(&self, keys: &Interval<Key>) -> bool {
        if keys.start > self.max_end {
            return false;
        }

        if self.keys.start <= keys.end && keys.start <= self.keys.end {
            return true;
        }

        if self.left.as_ref().map_or(false, |left| left.overlaps(keys)) {
            return true;
        }

        if keys.end < self.keys.start {
            return false;
        }

        self.right.as_ref().map_or(false, |right| right.overlaps(keys))
    }

    fn needs_balance(&self) -> bool {
        let diff = self.left_height() as i64 - self.right_height() as i64;
        diff * diff == 2
    }

    fn left_height(&self) -> u32 {
        self.left.as_ref().map_or(0, |left| left.height)
    }

    fn right_height(&self) -> u32 {
        self.right.as_ref().map_or(0, |right| right.height)
    }
}

#[derive(Debug)]
pub enum Error {
    OverlapErr,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::OverlapErr => write!(f, "Intervals overlap"),
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
    // TODO(shane)
}
