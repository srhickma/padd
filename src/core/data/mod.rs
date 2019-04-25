use std::{fmt, hash::Hash};

pub mod map;

pub trait Data: PartialEq + Eq + Hash + Clone + fmt::Debug {
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
