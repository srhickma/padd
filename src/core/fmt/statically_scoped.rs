use core::fmt::Formatter;
use core::parse::Tree;

pub struct StaticallyScopedFormatter;

impl Formatter for StaticallyScopedFormatter {
    fn reconstruct(&self, parse: &Option<Tree>) -> String {
        return self.reconstruct_internal(parse);
    }
}

impl StaticallyScopedFormatter {
    fn reconstruct_internal(&self, tree: &Option<Tree>) -> String {
        return String::new();
    }
}