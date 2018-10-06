use std::fmt;

pub mod stream;
pub mod map;

pub trait Data: PartialEq + Clone + fmt::Debug {
    fn to_string(&self) -> String;
}

impl Data for usize {
    fn to_string(&self) -> String {
        format!("{}", self)
    }
}
