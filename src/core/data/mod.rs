use std::{fmt, hash::Hash};

pub mod map;
pub mod interval;

/// A trait encapsulating some piece of data, with useful requirements for equality, comparison, and debugging.
pub trait Data: PartialEq + Eq + Hash + Clone + fmt::Debug + Send + Sync {
    /// Returns a string representation of the data.
    fn to_string(&self) -> String;
}

impl Data for usize {
    fn to_string(&self) -> String {
        format!("{}", self)
    }
}

impl Data for String {
    fn to_string(&self) -> String {
        self.clone()
    }
}
