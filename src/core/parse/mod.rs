use std::collections::HashMap;
use std::collections::HashSet;
use core::scan::Token;

mod earley;

pub trait Parser {
    fn parse<'a>(&self, scan: Vec<Token>, grammar: &Grammar<'a>) -> Option<Tree>;
}

pub fn def_parser() -> Box<Parser> {
    return Box::new(earley::EarleyParser);
}


#[derive(Clone)]
pub struct Tree {
    pub lhs: Token,
    pub children: Vec<Tree>,
}

impl Tree {
    pub fn print(&self){
        self.print_internal("".to_string(), true)
    }
    fn print_internal(&self, prefix: String, is_tail: bool) {
        if self.children.len() == 0 {
            println!("{}{}{} <- {}", prefix, if is_tail {"└── "} else {"├── "}, self.lhs.kind, self.lhs.lexeme);
        }
            else {
                println!("{}{}{}", prefix, if is_tail {"└── "} else {"├── "}, self.lhs.kind);
                let mut i = 0;
                let len = self.children.len();
                for child in &self.children {
                    if i == len - 1{
                        child.print_internal(format!("{}{}", prefix, if is_tail {"    "} else {"│   "}), true);
                    } else {
                        child.print_internal(format!("{}{}", prefix, if is_tail {"    "} else {"│   "}), false);
                    }
                    i += 1;
                }
            }
    }
}

pub struct Grammar<'a> {
    productions: &'a [Production<'a>],
    non_terminals: HashSet<&'a str>,
    terminals: HashSet<&'a str>,
    symbols: HashSet<&'a str>,
    start: &'a str,
    prods_exp: HashMap<&'a str, Vec<&'a Production<'a>>>
}

impl<'a> Grammar<'a> {
    pub fn from(productions: &'a [Production<'a>]) -> Grammar<'a> {
        let non_terminals: HashSet<&'a str> = productions.iter()
            .map(|prod| prod.lhs)
            .collect();
        let mut symbols: HashSet<&'a str> = productions.iter()
            .flat_map(|prod| prod.rhs.iter())
            .map(|&x| x)
            .collect();
        for non_terminal in &non_terminals {
            symbols.insert(non_terminal);
        }
        let terminals = symbols.difference(&non_terminals)
            .map(|&x| x)
            .collect();

        let mut prods_exp = HashMap::new();

        for prod in productions {
            if !prods_exp.contains_key(prod.lhs) {
                prods_exp.insert(prod.lhs, vec![]);
            }
            prods_exp.get_mut(prod.lhs).unwrap().push(prod);
        }

        return Grammar {
            productions,
            non_terminals,
            terminals,
            symbols,
            start: productions[0].lhs,
            prods_exp,
        };
    }
}

#[derive(PartialEq, Clone)]
pub struct Production<'a> {
    pub lhs: &'a str,
    pub rhs: &'a [&'a str],
}

impl<'a> Production<'a> {
    fn to_string(&self) -> String {
        let mut rhs: String = "".to_string();
        for s in self.rhs {
            rhs.push_str(s);
            rhs.push(' ');
        }
        return format!("{} -> {}", self.lhs, rhs);
    }
}