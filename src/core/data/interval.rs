use core::data::bst::{BinarySearchTree, AVLTree};

pub struct Interval<T: Ord> {
    start: T,
    end: T,
}

impl<T: Ord> Interval<T> {
    pub fn new(start: T, end: T) -> Self {
        Interval {
            start,
            end,
        }
    }

    pub fn contains(&self, value: &T) -> bool {
        self.start <= *value && self.end >= *value
    }
}

pub struct IntervalTree<'it, T: 'it + Ord> {
    tree: Box<BinarySearchTree<T, IntervalTreeNode<T>> + 'it>,
}

impl<'it, T: 'it + Ord> IntervalTree<'it, T> {
    pub fn new() -> Self {
        IntervalTree {
            tree: Box::new(AVLTree::new()),
        }
    }

    pub fn insert(&mut self, interval: Interval<T>) {

    }

    pub fn contains(&mut self, value: &T) -> bool {
        false
    }
}

struct IntervalTreeNode<T: Ord> {
    interval: Interval<T>,
    max: T,
}

#[cfg(test)]
mod tests {
    use super::*;
}
