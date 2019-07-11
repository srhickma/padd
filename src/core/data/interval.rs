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

pub struct IntervalTree<T: Ord> {
    root: Option<IntervalTreeNode<T>>,
}

impl<T: Ord> IntervalTree<T> {
    pub fn new() -> Self {
        IntervalTree {
            root: None,
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
//    left: Option<IntervalTreeNode<T>>,
//    right: Option<IntervalTreeNode<T>>,
}

impl<T: Ord> IntervalTreeNode<T> {

}

#[cfg(test)]
mod tests {
    use super::*;
}
