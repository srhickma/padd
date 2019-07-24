use std::{
    cmp, error, fmt,
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
            (
                Rotation::Right,
                left_node.left_height() < left_node.right_height(),
            )
        } else {
            let right_node = node.right.as_ref().unwrap();
            (
                Rotation::Left,
                right_node.right_height() < right_node.left_height(),
            )
        };

        match rotation {
            Rotation::Right => {
                if double {
                    node.left = Some(IntervalMap::left_rotation(node.left.take().unwrap()));
                }
                IntervalMap::right_rotation(node)
            }
            Rotation::Left => {
                if double {
                    node.right = Some(IntervalMap::right_rotation(node.right.take().unwrap()));
                }
                IntervalMap::left_rotation(node)
            }
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

        if let Some(value) = self.left.as_ref().and_then(|left| left.get(key)) {
            return Some(value);
        }

        if *key < self.keys.start {
            return None;
        }

        self.right.as_ref().and_then(|right| right.get(key))
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

        self.right
            .as_ref()
            .map_or(false, |right| right.overlaps(keys))
    }

    fn needs_balance(&self) -> bool {
        let diff = i64::from(self.left_height()) - i64::from(self.right_height());
        diff * diff == 4
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
    use super::*;

    use std::f64;

    #[test]
    fn simple_map() {
        //setup
        let mut map: IntervalMap<u32, u32> = IntervalMap::new();

        //exercise/verify
        assert_eq!(map.get(&3), None);

        map.insert(Interval::from(1..=3), 32).unwrap();

        assert_eq!(map.get(&0), None);
        assert_eq!(map.get(&1), Some(&32));
        assert_eq!(map.get(&2), Some(&32));
        assert_eq!(map.get(&3), Some(&32));
        assert_eq!(map.get(&4), None);
        assert_balance(&map, 1);

        map.insert(Interval::from(6..14), 16).unwrap();

        assert_eq!(map.get(&1), Some(&32));
        assert_eq!(map.get(&5), None);
        assert_eq!(map.get(&6), Some(&16));
        assert_eq!(map.get(&10), Some(&16));
        assert_eq!(map.get(&13), Some(&16));
        assert_eq!(map.get(&14), None);
        assert_balance(&map, 2);

        map.insert(Interval::from(5..=5), 1).unwrap();

        assert_eq!(map.get(&4), None);
        assert_eq!(map.get(&5), Some(&1));
        assert_eq!(map.get(&6), Some(&16));
        assert_balance(&map, 3);
    }

    #[test]
    fn point_map() {
        //setup
        let mut map: IntervalMap<u32, u32> = IntervalMap::new();

        //exercise
        for i in 1..100000 {
            map.insert(Interval::from(i..=i), i).unwrap();
            assert_balance(&map, i as u32);
        }

        //verify
        for i in 1..100000 {
            assert_eq!(map.get(&i), Some(&(i)));
        }
    }

    #[test]
    fn division_map() {
        //setup
        let mut map: IntervalMap<u32, u32> = IntervalMap::new();

        //exercise
        for i in 1..10000 {
            map.insert(Interval::from(i * 10..(i + 1) * 10), i).unwrap();
            assert_balance(&map, i as u32);
        }

        //verify
        for i in 1..10000 {
            for j in i * 10..(i + 1) * 10 {
                assert_eq!(map.get(&j), Some(&(i)));
            }
        }
    }

    #[test]
    fn spread_map() {
        //setup
        let mut map: IntervalMap<u32, u32> = IntervalMap::new();

        //exercise
        map.insert(Interval::from(0..=100), 1).unwrap();
        map.insert(Interval::from(1000000..=10000000), 2).unwrap();
        map.insert(Interval::from(10000..=100000), 3).unwrap();

        //verify
        assert_eq!(map.get(&0), Some(&1));
        assert_eq!(map.get(&100), Some(&1));
        assert_eq!(map.get(&1000000), Some(&2));
        assert_eq!(map.get(&10000000), Some(&2));
        assert_eq!(map.get(&10000), Some(&3));
        assert_eq!(map.get(&100000), Some(&3));

        assert_balance(&map, 3);
    }

    #[test]
    fn middle_out_map() {
        //setup
        let mut map: IntervalMap<i32, i32> = IntervalMap::new();

        impl Bound for i32 {
            fn predecessor(&self) -> Self {
                self - 1
            }
        }

        //exercise
        for i in 1..10000 {
            map.insert(Interval::from(i..=i), i).unwrap();
            map.insert(Interval::from(-i..=-i), -i).unwrap();
            assert_balance(&map, (i * 2) as u32);
        }

        //verify
        for i in 1..10000 {
            assert_eq!(map.get(&i), Some(&(i)));
            assert_eq!(map.get(&-i), Some(&(-i)));
        }
    }

    #[test]
    fn overlapping_left() {
        //setup
        let mut map: IntervalMap<u32, u32> = IntervalMap::new();
        map.insert(Interval::from(43..=99), 12).unwrap();

        //execute
        let res = map.insert(Interval::from(25..67), 11);

        //verify
        assert!(res.is_err());
        assert_balance(&map, 1);
    }

    #[test]
    fn overlapping_right() {
        //setup
        let mut map: IntervalMap<u32, u32> = IntervalMap::new();
        map.insert(Interval::from(43..=99), 12).unwrap();

        //execute
        let res = map.insert(Interval::from(50..100), 11);

        //verify
        assert!(res.is_err());
        assert_balance(&map, 1);
    }

    #[test]
    fn overlapping_enclosed() {
        //setup
        let mut map: IntervalMap<u32, u32> = IntervalMap::new();
        map.insert(Interval::from(43..=99), 12).unwrap();

        //execute
        let res = map.insert(Interval::from(44..50), 11);

        //verify
        assert!(res.is_err());
        assert_balance(&map, 1);
    }

    #[test]
    fn overlapping_enclosing() {
        //setup
        let mut map: IntervalMap<u32, u32> = IntervalMap::new();
        map.insert(Interval::from(43..=99), 12).unwrap();

        //execute
        let res = map.insert(Interval::from(20..101), 11);

        //verify
        assert!(res.is_err());
        assert_balance(&map, 1);
    }

    #[test]
    fn overlapping_same() {
        //setup
        let mut map: IntervalMap<u32, u32> = IntervalMap::new();
        map.insert(Interval::from(43..=99), 12).unwrap();

        //execute
        let res = map.insert(Interval::from(43..=99), 11);

        //verify
        assert!(res.is_err());
        assert_balance(&map, 1);
    }

    #[test]
    fn overlapping_point() {
        //setup
        let mut map: IntervalMap<u32, u32> = IntervalMap::new();
        map.insert(Interval::from(4..=4), 12).unwrap();
        map.insert(Interval::from(20..=25), 12).unwrap();

        //execute/verify
        assert!(map.insert(Interval::from(3..=6), 11).is_err());
        assert!(map.insert(Interval::from(4..=4), 11).is_err());
        assert!(map.insert(Interval::from(4..=6), 11).is_err());
        assert!(map.insert(Interval::from(21..=21), 11).is_err());

        assert_balance(&map, 2);
    }

    fn assert_balance<K: Bound, V>(map: &IntervalMap<K, V>, size: u32) {
        let actual_height = map.root.as_ref().map_or(0, |root| root.height) as i64;
        let expected_height = f64::from(size).log2().ceil() as i64;
        assert!(actual_height == expected_height || actual_height == expected_height + 1);
    }
}
