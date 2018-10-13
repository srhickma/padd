use std::collections::HashMap;

use core::data::Data;
use core::parse::Tree;
use core::parse::Production;
use core::fmt::pattern::*;

mod pattern;

pub struct Formatter {
    pattern_map: HashMap<String, Pattern>,
}

impl Formatter {
    pub fn create(patterns: Vec<PatternPair>) -> Result<Formatter, BuildError> {
        let mut pattern_map = HashMap::new();
        for pattern_pair in patterns {
            pattern_map.insert(
                pattern_pair.production.to_string(),
                generate_pattern(&pattern_pair.pattern[..], &pattern_pair.production)?,
            );
        }
        Ok(Formatter {
            pattern_map,
        })
    }

    pub fn format<'a>(&self, parse: &'a Tree) -> String {
        let format_job = FormatJob {
            parse,
            pattern_map: &self.pattern_map,
        };
        format_job.run()
    }
}

pub type BuildError = pattern::BuildError;

struct FormatJob<'a> {
    parse: &'a Tree,
    pattern_map: &'a HashMap<String, Pattern>,
}

impl<'a> FormatJob<'a> {
    fn run(&self) -> String {
        return self.recur(self.parse, &HashMap::new());
    }

    fn recur(&self, node: &Tree, scope: &HashMap<String, String>) -> String {
        if node.is_leaf() {
            if node.is_null() {
                return String::new();
            }
            return node.lhs.lexeme.clone();
        }

        let pattern = self.pattern_map.get(&node.production()[..]);
        match pattern {
            Some(ref p) => self.fill_pattern(p, &node.children, scope),
            None => { //Reconstruct one after the other
                let mut res = String::new();
                for child in &node.children {
                    res = format!("{}{}", res, self.recur(child, scope));
                }
                return res;
            }
        }
    }

    fn fill_pattern(&self, pattern: &Pattern, children: &Vec<Tree>, scope: &HashMap<String, String>) -> String {
        let mut res: String = String::new();
        for seg in &pattern.segments {
            match seg {
                &Segment::Filler(ref s) => res = format!("{}{}", res, s),
                &Segment::Substitution(ref s) => match scope.get(s) {
                    Some(value) => res = format!("{}{}", res, value),
                    None => {}
                },
                &Segment::Capture(ref c) => res = format!("{}{}", res, self.evaluate_capture(c, children, scope)),
            };
        }
        res
    }

    fn evaluate_capture(&self, capture: &Capture, children: &Vec<Tree>, outer_scope: &HashMap<String, String>) -> String {
        if capture.declarations.len() > 0 {
            let mut inner_scope = outer_scope.clone();
            for decl in &capture.declarations {
                match &decl.value {
                    &Some(ref pattern) => {
                        inner_scope.insert(decl.key.clone(), self.fill_pattern(pattern, children, outer_scope));
                    }
                    &None => {
                        inner_scope.remove(&decl.key);
                    }
                }
            }
            match children.get(capture.child_index) {
                Some(child) => return self.recur(child, &inner_scope),
                None => panic!("Pattern index out of bounds: index={} children={}", capture.child_index, children.len()),
            }
        } else {
            match children.get(capture.child_index) {
                Some(child) => return self.recur(child, outer_scope),
                None => panic!("Pattern index out of bounds: index={} children={}", capture.child_index, children.len()),
            }
        }
    }
}

pub struct PatternPair {
    pub production: Production,
    pub pattern: String,
}
