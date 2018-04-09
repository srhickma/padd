use core::fmt::Formatter;
use core::fmt::PatternPair;
use core::parse::Tree;
use std::collections::HashMap;

pub struct StaticallyScopedFormatter;

impl Formatter for StaticallyScopedFormatter {
    fn reconstruct(&self, parse: &Tree, patterns: &[PatternPair]) -> String {
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
            println!("{}", node.production());
            //res = formatter(&tree.lhs.kind, &child.lhs.kind, &res, recon(child, formatter)); //This is where we add custom formatting
        }
        return res;
    }
}
