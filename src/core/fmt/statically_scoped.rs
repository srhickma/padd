use core::fmt::Formatter;
use core::fmt::PatternPair;
use core::parse::Tree;
use std::collections::HashMap;

pub struct StaticallyScopedFormatter;

impl Formatter for StaticallyScopedFormatter {
    fn reconstruct(&self, parse: &Tree, patterns: &[PatternPair]) -> String {
        //TODO generate pattern structs from pattern pairs and populate hashmap
        return StaticallyScopedFormatter::reconstruct_internal(parse, &HashMap::new());
    }
}

impl StaticallyScopedFormatter {
    //TODO better interface hiding
    pub fn reconstruct_internal(node: &Tree, scope: &HashMap<String, String>) -> String {
        if node.is_leaf() {
            if node.is_null() {
                return String::new();
            }
            return node.lhs.lexeme.clone();
        }
        let mut res = String::new();
        for child in &node.children {
            //TODO need to somehow capture the production -> pattern map on the formatter so that
            //recursive calls to reconstruct_internal can access the patterns
        }
        return res;
    }
}
