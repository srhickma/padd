use {
    core::{
        fmt::{self, Formatter},
        parse::{
            self,
            grammar::Grammar,
            Tree,
        },
        scan::{
            self,
            ecdfa::EncodedCDFA,
            Kind,
        },
    },
    std::{
        self,
        error,
    },
};

mod gen;
mod lang;
mod region;

pub static DEF_MATCHER: &'static str = "_";

pub fn parse_spec(input: &str) -> Result<Tree, ParseError> {
    lang::parse_spec(input)
}

pub fn generate_spec(parse: &Tree) -> Result<(EncodedCDFA<Kind>, Grammar, Formatter), GenError> {
    gen::generate_spec(parse)
}

#[derive(Debug)]
pub enum GenError {
    MatcherErr(String),
    MappingErr(String),
    CDFAErr(scan::CDFAError),
    PatternErr(fmt::BuildError),
    RegionErr(region::Error),
}

impl std::fmt::Display for GenError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            GenError::MatcherErr(ref err) => write!(f, "Matcher definition error: {}", err),
            GenError::MappingErr(ref err) => write!(f, "ECDFA to grammar mapping error: {}", err),
            GenError::CDFAErr(ref err) => write!(f, "ECDFA generation error: {}", err),
            GenError::PatternErr(ref err) => write!(f, "Pattern build error: {}", err),
            GenError::RegionErr(ref err) => write!(f, "{}", err),
        }
    }
}

impl error::Error for GenError {
    fn cause(&self) -> Option<&error::Error> {
        match *self {
            GenError::MatcherErr(_) => None,
            GenError::MappingErr(_) => None,
            GenError::CDFAErr(ref err) => Some(err),
            GenError::PatternErr(ref err) => Some(err),
            GenError::RegionErr(ref err) => Some(err),
        }
    }
}

impl From<scan::CDFAError> for GenError {
    fn from(err: scan::CDFAError) -> GenError {
        GenError::CDFAErr(err)
    }
}

impl From<fmt::BuildError> for GenError {
    fn from(err: fmt::BuildError) -> GenError {
        GenError::PatternErr(err)
    }
}

impl From<region::Error> for GenError {
    fn from(err: region::Error) -> GenError {
        GenError::RegionErr(err)
    }
}

#[derive(Debug)]
pub enum ParseError {
    ScanErr(scan::Error),
    ParseErr(parse::Error),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            ParseError::ScanErr(ref err) => write!(f, "Scan error: {}", err),
            ParseError::ParseErr(ref err) => write!(f, "Parse error: {}", err),
        }
    }
}

impl error::Error for ParseError {
    fn cause(&self) -> Option<&error::Error> {
        match *self {
            ParseError::ScanErr(ref err) => Some(err),
            ParseError::ParseErr(ref err) => Some(err),
        }
    }
}

impl From<scan::Error> for ParseError {
    fn from(err: scan::Error) -> ParseError {
        ParseError::ScanErr(err)
    }
}

impl From<parse::Error> for ParseError {
    fn from(err: parse::Error) -> ParseError {
        ParseError::ParseErr(err)
    }
}

#[cfg(test)]
mod tests {
    use core::{
        data::{
            Data,
            stream::StreamSource,
        },
        scan::Token,
    };

    use super::*;

    #[test]
    fn parse_spec_spaces() {
        //setup
        let input = "alphabet' 'cdfa{start;}grammar{s|s b;}";

        //exercise
        let tree = lang::parse_spec(input).unwrap();

        //verify
        assert_eq!(tree.to_string(),
                   "└── spec
    └── regions
        ├── regions
        │   ├── regions
        │   │   └── region
        │   │       └── alphabet
        │   │           ├── ALPHABET <- 'alphabet'
        │   │           └── CILC <- '' ''
        │   └── region
        │       └── cdfa
        │           ├── CDFA <- 'cdfa'
        │           ├── LBRACE <- '{'
        │           ├── states
        │           │   └── state
        │           │       ├── sdec
        │           │       │   └── targets
        │           │       │       └── ID <- 'start'
        │           │       ├── trans_opt
        │           │       │   └──  <- 'NULL'
        │           │       └── SEMI <- ';'
        │           └── RBRACE <- '}'
        └── region
            └── grammar
                ├── GRAMMAR <- 'grammar'
                ├── LBRACE <- '{'
                ├── prods
                │   └── prod
                │       ├── ID <- 's'
                │       ├── patt_opt
                │       │   └──  <- 'NULL'
                │       ├── rhss
                │       │   └── rhs
                │       │       ├── OR <- '|'
                │       │       ├── ids
                │       │       │   ├── ids
                │       │       │   │   ├── ids
                │       │       │   │   │   └──  <- 'NULL'
                │       │       │   │   └── ID <- 's'
                │       │       │   └── ID <- 'b'
                │       │       └── patt_opt
                │       │           └──  <- 'NULL'
                │       └── SEMI <- ';'
                └── RBRACE <- '}'"
        );
    }

    #[test]
    fn parse_spec_simple() {
        //setup
        let input = "
alphabet ' \t\n{}'

cdfa {
    start
        ' ' -> ws
        '\t' -> ws
        '\n' -> ws
        '{' -> lbr
        '}' -> rbr;

    ws  ^WHITESPACE
        ' ' -> ws
        '\t' -> ws
        '\n' -> ws;

    lbr ^LBRACKET;

    rbr ^RBRACKET;
}

grammar {
    s
        | s b
        |;
    b
        | LBRACKET s RBRACKET ``
        | w;

    w | WHITESPACE `[prefix]{0}\n\n{1;prefix=[prefix]\t}[prefix]{2}\n\n`;
}
        ";

        //exercise
        let tree = lang::parse_spec(input).unwrap();

        //verify
        assert_eq!(tree.to_string(),
                   "└── spec
    └── regions
        ├── regions
        │   ├── regions
        │   │   └── region
        │   │       └── alphabet
        │   │           ├── ALPHABET <- 'alphabet'
        │   │           └── CILC <- '' \\t\\n{}''
        │   └── region
        │       └── cdfa
        │           ├── CDFA <- 'cdfa'
        │           ├── LBRACE <- '{'
        │           ├── states
        │           │   ├── states
        │           │   │   ├── states
        │           │   │   │   ├── states
        │           │   │   │   │   └── state
        │           │   │   │   │       ├── sdec
        │           │   │   │   │       │   └── targets
        │           │   │   │   │       │       └── ID <- 'start'
        │           │   │   │   │       ├── trans_opt
        │           │   │   │   │       │   └── trans
        │           │   │   │   │       │       ├── trans
        │           │   │   │   │       │       │   ├── trans
        │           │   │   │   │       │       │   │   ├── trans
        │           │   │   │   │       │       │   │   │   ├── trans
        │           │   │   │   │       │       │   │   │   │   └── tran
        │           │   │   │   │       │       │   │   │   │       ├── mtcs
        │           │   │   │   │       │       │   │   │   │       │   └── mtc
        │           │   │   │   │       │       │   │   │   │       │       └── CILC <- '' ''
        │           │   │   │   │       │       │   │   │   │       ├── ARROW <- '->'
        │           │   │   │   │       │       │   │   │   │       └── trand
        │           │   │   │   │       │       │   │   │   │           └── ID <- 'ws'
        │           │   │   │   │       │       │   │   │   └── tran
        │           │   │   │   │       │       │   │   │       ├── mtcs
        │           │   │   │   │       │       │   │   │       │   └── mtc
        │           │   │   │   │       │       │   │   │       │       └── CILC <- ''\\t''
        │           │   │   │   │       │       │   │   │       ├── ARROW <- '->'
        │           │   │   │   │       │       │   │   │       └── trand
        │           │   │   │   │       │       │   │   │           └── ID <- 'ws'
        │           │   │   │   │       │       │   │   └── tran
        │           │   │   │   │       │       │   │       ├── mtcs
        │           │   │   │   │       │       │   │       │   └── mtc
        │           │   │   │   │       │       │   │       │       └── CILC <- ''\\n''
        │           │   │   │   │       │       │   │       ├── ARROW <- '->'
        │           │   │   │   │       │       │   │       └── trand
        │           │   │   │   │       │       │   │           └── ID <- 'ws'
        │           │   │   │   │       │       │   └── tran
        │           │   │   │   │       │       │       ├── mtcs
        │           │   │   │   │       │       │       │   └── mtc
        │           │   │   │   │       │       │       │       └── CILC <- ''{''
        │           │   │   │   │       │       │       ├── ARROW <- '->'
        │           │   │   │   │       │       │       └── trand
        │           │   │   │   │       │       │           └── ID <- 'lbr'
        │           │   │   │   │       │       └── tran
        │           │   │   │   │       │           ├── mtcs
        │           │   │   │   │       │           │   └── mtc
        │           │   │   │   │       │           │       └── CILC <- ''}''
        │           │   │   │   │       │           ├── ARROW <- '->'
        │           │   │   │   │       │           └── trand
        │           │   │   │   │       │               └── ID <- 'rbr'
        │           │   │   │   │       └── SEMI <- ';'
        │           │   │   │   └── state
        │           │   │   │       ├── sdec
        │           │   │   │       │   ├── targets
        │           │   │   │       │   │   └── ID <- 'ws'
        │           │   │   │       │   └── acceptor
        │           │   │   │       │       ├── HAT <- '^'
        │           │   │   │       │       ├── id_or_def
        │           │   │   │       │       │   └── ID <- 'WHITESPACE'
        │           │   │   │       │       └── accd_opt
        │           │   │   │       │           └──  <- 'NULL'
        │           │   │   │       ├── trans_opt
        │           │   │   │       │   └── trans
        │           │   │   │       │       ├── trans
        │           │   │   │       │       │   ├── trans
        │           │   │   │       │       │   │   └── tran
        │           │   │   │       │       │   │       ├── mtcs
        │           │   │   │       │       │   │       │   └── mtc
        │           │   │   │       │       │   │       │       └── CILC <- '' ''
        │           │   │   │       │       │   │       ├── ARROW <- '->'
        │           │   │   │       │       │   │       └── trand
        │           │   │   │       │       │   │           └── ID <- 'ws'
        │           │   │   │       │       │   └── tran
        │           │   │   │       │       │       ├── mtcs
        │           │   │   │       │       │       │   └── mtc
        │           │   │   │       │       │       │       └── CILC <- ''\\t''
        │           │   │   │       │       │       ├── ARROW <- '->'
        │           │   │   │       │       │       └── trand
        │           │   │   │       │       │           └── ID <- 'ws'
        │           │   │   │       │       └── tran
        │           │   │   │       │           ├── mtcs
        │           │   │   │       │           │   └── mtc
        │           │   │   │       │           │       └── CILC <- ''\\n''
        │           │   │   │       │           ├── ARROW <- '->'
        │           │   │   │       │           └── trand
        │           │   │   │       │               └── ID <- 'ws'
        │           │   │   │       └── SEMI <- ';'
        │           │   │   └── state
        │           │   │       ├── sdec
        │           │   │       │   ├── targets
        │           │   │       │   │   └── ID <- 'lbr'
        │           │   │       │   └── acceptor
        │           │   │       │       ├── HAT <- '^'
        │           │   │       │       ├── id_or_def
        │           │   │       │       │   └── ID <- 'LBRACKET'
        │           │   │       │       └── accd_opt
        │           │   │       │           └──  <- 'NULL'
        │           │   │       ├── trans_opt
        │           │   │       │   └──  <- 'NULL'
        │           │   │       └── SEMI <- ';'
        │           │   └── state
        │           │       ├── sdec
        │           │       │   ├── targets
        │           │       │   │   └── ID <- 'rbr'
        │           │       │   └── acceptor
        │           │       │       ├── HAT <- '^'
        │           │       │       ├── id_or_def
        │           │       │       │   └── ID <- 'RBRACKET'
        │           │       │       └── accd_opt
        │           │       │           └──  <- 'NULL'
        │           │       ├── trans_opt
        │           │       │   └──  <- 'NULL'
        │           │       └── SEMI <- ';'
        │           └── RBRACE <- '}'
        └── region
            └── grammar
                ├── GRAMMAR <- 'grammar'
                ├── LBRACE <- '{'
                ├── prods
                │   ├── prods
                │   │   ├── prods
                │   │   │   └── prod
                │   │   │       ├── ID <- 's'
                │   │   │       ├── patt_opt
                │   │   │       │   └──  <- 'NULL'
                │   │   │       ├── rhss
                │   │   │       │   ├── rhss
                │   │   │       │   │   └── rhs
                │   │   │       │   │       ├── OR <- '|'
                │   │   │       │   │       ├── ids
                │   │   │       │   │       │   ├── ids
                │   │   │       │   │       │   │   ├── ids
                │   │   │       │   │       │   │   │   └──  <- 'NULL'
                │   │   │       │   │       │   │   └── ID <- 's'
                │   │   │       │   │       │   └── ID <- 'b'
                │   │   │       │   │       └── patt_opt
                │   │   │       │   │           └──  <- 'NULL'
                │   │   │       │   └── rhs
                │   │   │       │       ├── OR <- '|'
                │   │   │       │       ├── ids
                │   │   │       │       │   └──  <- 'NULL'
                │   │   │       │       └── patt_opt
                │   │   │       │           └──  <- 'NULL'
                │   │   │       └── SEMI <- ';'
                │   │   └── prod
                │   │       ├── ID <- 'b'
                │   │       ├── patt_opt
                │   │       │   └──  <- 'NULL'
                │   │       ├── rhss
                │   │       │   ├── rhss
                │   │       │   │   └── rhs
                │   │       │   │       ├── OR <- '|'
                │   │       │   │       ├── ids
                │   │       │   │       │   ├── ids
                │   │       │   │       │   │   ├── ids
                │   │       │   │       │   │   │   ├── ids
                │   │       │   │       │   │   │   │   └──  <- 'NULL'
                │   │       │   │       │   │   │   └── ID <- 'LBRACKET'
                │   │       │   │       │   │   └── ID <- 's'
                │   │       │   │       │   └── ID <- 'RBRACKET'
                │   │       │   │       └── patt_opt
                │   │       │   │           └── PATTC <- '``'
                │   │       │   └── rhs
                │   │       │       ├── OR <- '|'
                │   │       │       ├── ids
                │   │       │       │   ├── ids
                │   │       │       │   │   └──  <- 'NULL'
                │   │       │       │   └── ID <- 'w'
                │   │       │       └── patt_opt
                │   │       │           └──  <- 'NULL'
                │   │       └── SEMI <- ';'
                │   └── prod
                │       ├── ID <- 'w'
                │       ├── patt_opt
                │       │   └──  <- 'NULL'
                │       ├── rhss
                │       │   └── rhs
                │       │       ├── OR <- '|'
                │       │       ├── ids
                │       │       │   ├── ids
                │       │       │   │   └──  <- 'NULL'
                │       │       │   └── ID <- 'WHITESPACE'
                │       │       └── patt_opt
                │       │           └── PATTC <- '`[prefix]{0}\\n\\n{1;prefix=[prefix]\\t}[prefix]{2}\\n\\n`'
                │       └── SEMI <- ';'
                └── RBRACE <- '}'"
        );
    }

    #[test]
    fn generate_spec_simple() {
        //setup
        let spec = "
alphabet ' \\t\\n{}'

cdfa {
    start
        ' ' | '\\t' | '\\n' -> ws
        '{' -> lbr
        '}' -> rbr;

    ws  ^WHITESPACE
        ' ' | '\\t' | '\\n' -> ws;

    lbr ^LBRACKET;

    rbr ^RBRACKET;
}

grammar {
    s
        | s b
        | ;

    b
        | LBRACKET s RBRACKET `[prefix]{0}\\n\\n{1;prefix=[prefix]\\t}[prefix]{2}\\n\\n`
        | w ;

    w   | WHITESPACE ``;
}
        ";

        let input = "  {  {  {{{\t}}}\n {} }  }   { {}\n } ".to_string();
        let mut iter = input.chars();
        let mut getter = || {
            iter.next()
        };
        let mut stream: StreamSource<char> = StreamSource::observe(&mut getter);

        let scanner = scan::def_scanner();
        let parser = parse::def_parser();

        //specification
        let tree = lang::parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, grammar, formatter) = generate_spec(&parse).unwrap();

        //input
        let tokens = scanner.scan(&mut stream, &cdfa);
        let tree = parser.parse(tokens.unwrap(), &grammar);
        let parse = tree.unwrap();

        //exercise
        let res = formatter.format(&parse);

        //verify
        assert_eq!(res,
                   "{

	{

		{

			{

				{

				}

			}

		}

		{

		}

	}

}

{

	{

	}

}\n\n"
        );
    }

    #[test]
    fn generate_spec_advanced_operators() {
        //setup
        let spec = "
alphabet 'inj '

cdfa {
    start
        'in' -> ^IN
        ' ' -> ^_
        _ -> ^ID;

    ID | IN
        ' ' -> fail
        _ -> ID;
}

grammar {
    s |;
}
        ";

        let input = "i ij ijjjijijiji inj in iii".to_string();
        let mut iter = input.chars();
        let mut getter = || {
            iter.next()
        };
        let mut stream: StreamSource<char> = StreamSource::observe(&mut getter);

        let scanner = scan::def_scanner();

        let tree = lang::parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, _, _) = generate_spec(&parse).unwrap();

        //exercise
        let tokens = scanner.scan(&mut stream, &cdfa).unwrap();

        let mut result = String::new();
        for token in tokens {
            result.push_str(&token.to_string());
            result.push('\n');
        }

        //verify
        assert_eq!(result, "\
ID <- 'i'
ID <- 'ij'
ID <- 'ijjjijijiji'
ID <- 'inj'
IN <- 'in'
ID <- 'iii'
");
    }

    #[test]
    fn default_matcher_conflict() {
        //setup
        let spec = "
alphabet ' c'

cdfa {
    start
        ' ' -> ^WS
        'c' -> id;

    id      ^ID
        'c' | '_' -> id;
}

grammar {
    s |;
}
        ";

        let input = "c c".to_string();
        let mut iter = input.chars();
        let mut getter = || {
            iter.next()
        };
        let mut stream: StreamSource<char> = StreamSource::observe(&mut getter);

        let scanner = scan::def_scanner();

        let tree = lang::parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, _, _) = generate_spec(&parse).unwrap();

        //exercise
        let tokens = scanner.scan(&mut stream, &cdfa).unwrap();

        //verify
        assert_eq!(tokens_string(tokens), "\nkind=ID lexeme=c\nkind=WS lexeme= \nkind=ID lexeme=c")
    }

    #[test]
    fn complex_id() {
        //setup
        let spec = "
alphabet ' ab_'

cdfa {
    start
        ' ' -> ws
        _ -> id;

    ws      ^_;

    id      ^ID
        'a' | 'b' | '_' -> id;
}

grammar {
    s
        | ids
        |;
    ids
        | ids ID
        | ID;
}
        ";

        let input = "a ababab _abab ab_abba_".to_string();
        let mut iter = input.chars();
        let mut getter = || {
            iter.next()
        };
        let mut stream: StreamSource<char> = StreamSource::observe(&mut getter);

        let scanner = scan::def_scanner();

        let tree = lang::parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, _, _) = generate_spec(&parse).unwrap();

        //exercise
        let tokens = scanner.scan(&mut stream, &cdfa).unwrap();

        //verify
        assert_eq!(tokens_string(tokens), "\nkind=ID lexeme=a\nkind=ID lexeme=ababab\nkind=ID lexeme=_abab\nkind=ID lexeme=ab_abba_")
    }

    #[test]
    fn multi_character_lexing() {
        //setup
        let spec = "
alphabet 'abcdefghijklmnopqrstuvwxyz '

cdfa {
    start
        'if' -> ^IF
        'else' -> ^ELSE
        'for' -> ^FOR
        'fob' -> ^FOB
        'final' -> ^FINAL
        ' ' -> ^_
        _ -> id;

    id  ^ID
        ' ' -> fail
        _ -> id;
}

grammar {
    s |;
}
        ";

        let input = "fdkgdfjgdjglkdjglkdjgljbnhbduhoifjeoigjeoghknhkjdfjgoirjt for if endif elseif somethign eldsfnj hi bob joe here final for fob else if id idhere fobre f ".to_string();
        let mut iter = input.chars();
        let mut getter = || {
            iter.next()
        };
        let mut stream: StreamSource<char> = StreamSource::observe(&mut getter);

        let scanner = scan::def_scanner();
        let tree = lang::parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, _, _) = generate_spec(&parse).unwrap();

        //exercise
        let tokens = scanner.scan(&mut stream, &cdfa).unwrap();

        //verify
        assert_eq!(tokens_string(tokens), "
kind=ID lexeme=fdkgdfjgdjglkdjglkdjgljbnhbduhoifjeoigjeoghknhkjdfjgoirjt
kind=FOR lexeme=for
kind=IF lexeme=if
kind=ID lexeme=endif
kind=ELSE lexeme=else
kind=IF lexeme=if
kind=ID lexeme=somethign
kind=ID lexeme=eldsfnj
kind=ID lexeme=hi
kind=ID lexeme=bob
kind=ID lexeme=joe
kind=ID lexeme=here
kind=FINAL lexeme=final
kind=FOR lexeme=for
kind=FOB lexeme=fob
kind=ELSE lexeme=else
kind=IF lexeme=if
kind=ID lexeme=id
kind=ID lexeme=idhere
kind=FOB lexeme=fob
kind=ID lexeme=re
kind=ID lexeme=f")
    }

    #[test]
    fn parse_spec_olap_trans() {
        //setup
        let input = "
alphabet 'inj '

cdfa {
    start
        'i' -> ki
        _ -> ^ID;

    ki
        'n' -> ^IN;

    ID | ki
        ' ' -> fail
        _ -> ID;
}

grammar {
    s
        | ID s
        | ID;
}
        ";

        //exercise
        let tree = lang::parse_spec(input).unwrap();

        //verify
        assert_eq!(tree.to_string(),
                   "└── spec
    └── regions
        ├── regions
        │   ├── regions
        │   │   └── region
        │   │       └── alphabet
        │   │           ├── ALPHABET <- 'alphabet'
        │   │           └── CILC <- ''inj ''
        │   └── region
        │       └── cdfa
        │           ├── CDFA <- 'cdfa'
        │           ├── LBRACE <- '{'
        │           ├── states
        │           │   ├── states
        │           │   │   ├── states
        │           │   │   │   └── state
        │           │   │   │       ├── sdec
        │           │   │   │       │   └── targets
        │           │   │   │       │       └── ID <- 'start'
        │           │   │   │       ├── trans_opt
        │           │   │   │       │   └── trans
        │           │   │   │       │       ├── trans
        │           │   │   │       │       │   └── tran
        │           │   │   │       │       │       ├── mtcs
        │           │   │   │       │       │       │   └── mtc
        │           │   │   │       │       │       │       └── CILC <- ''i''
        │           │   │   │       │       │       ├── ARROW <- '->'
        │           │   │   │       │       │       └── trand
        │           │   │   │       │       │           └── ID <- 'ki'
        │           │   │   │       │       └── tran
        │           │   │   │       │           ├── DEF <- '_'
        │           │   │   │       │           ├── ARROW <- '->'
        │           │   │   │       │           └── trand
        │           │   │   │       │               └── acceptor
        │           │   │   │       │                   ├── HAT <- '^'
        │           │   │   │       │                   ├── id_or_def
        │           │   │   │       │                   │   └── ID <- 'ID'
        │           │   │   │       │                   └── accd_opt
        │           │   │   │       │                       └──  <- 'NULL'
        │           │   │   │       └── SEMI <- ';'
        │           │   │   └── state
        │           │   │       ├── sdec
        │           │   │       │   └── targets
        │           │   │       │       └── ID <- 'ki'
        │           │   │       ├── trans_opt
        │           │   │       │   └── trans
        │           │   │       │       └── tran
        │           │   │       │           ├── mtcs
        │           │   │       │           │   └── mtc
        │           │   │       │           │       └── CILC <- ''n''
        │           │   │       │           ├── ARROW <- '->'
        │           │   │       │           └── trand
        │           │   │       │               └── acceptor
        │           │   │       │                   ├── HAT <- '^'
        │           │   │       │                   ├── id_or_def
        │           │   │       │                   │   └── ID <- 'IN'
        │           │   │       │                   └── accd_opt
        │           │   │       │                       └──  <- 'NULL'
        │           │   │       └── SEMI <- ';'
        │           │   └── state
        │           │       ├── sdec
        │           │       │   └── targets
        │           │       │       ├── targets
        │           │       │       │   └── ID <- 'ID'
        │           │       │       ├── OR <- '|'
        │           │       │       └── ID <- 'ki'
        │           │       ├── trans_opt
        │           │       │   └── trans
        │           │       │       ├── trans
        │           │       │       │   └── tran
        │           │       │       │       ├── mtcs
        │           │       │       │       │   └── mtc
        │           │       │       │       │       └── CILC <- '' ''
        │           │       │       │       ├── ARROW <- '->'
        │           │       │       │       └── trand
        │           │       │       │           └── ID <- 'fail'
        │           │       │       └── tran
        │           │       │           ├── DEF <- '_'
        │           │       │           ├── ARROW <- '->'
        │           │       │           └── trand
        │           │       │               └── ID <- 'ID'
        │           │       └── SEMI <- ';'
        │           └── RBRACE <- '}'
        └── region
            └── grammar
                ├── GRAMMAR <- 'grammar'
                ├── LBRACE <- '{'
                ├── prods
                │   └── prod
                │       ├── ID <- 's'
                │       ├── patt_opt
                │       │   └──  <- 'NULL'
                │       ├── rhss
                │       │   ├── rhss
                │       │   │   └── rhs
                │       │   │       ├── OR <- '|'
                │       │   │       ├── ids
                │       │   │       │   ├── ids
                │       │   │       │   │   ├── ids
                │       │   │       │   │   │   └──  <- 'NULL'
                │       │   │       │   │   └── ID <- 'ID'
                │       │   │       │   └── ID <- 's'
                │       │   │       └── patt_opt
                │       │   │           └──  <- 'NULL'
                │       │   └── rhs
                │       │       ├── OR <- '|'
                │       │       ├── ids
                │       │       │   ├── ids
                │       │       │   │   └──  <- 'NULL'
                │       │       │   └── ID <- 'ID'
                │       │       └── patt_opt
                │       │           └──  <- 'NULL'
                │       └── SEMI <- ';'
                └── RBRACE <- '}'"
        );
    }

    #[test]
    fn parse_spec_optional_shorthand() {
        //setup
        let spec = "
alphabet 'ab'

cdfa {
    start
        'a' -> ^A
        'b' -> ^B;
}

grammar {
    s
        | A [B] s
        |;
}
        ";

        //exercise
        let tree = lang::parse_spec(spec).unwrap();

        //verify
        assert_eq!(tree.to_string(),
                   "└── spec
    └── regions
        ├── regions
        │   ├── regions
        │   │   └── region
        │   │       └── alphabet
        │   │           ├── ALPHABET <- 'alphabet'
        │   │           └── CILC <- ''ab''
        │   └── region
        │       └── cdfa
        │           ├── CDFA <- 'cdfa'
        │           ├── LBRACE <- '{'
        │           ├── states
        │           │   └── state
        │           │       ├── sdec
        │           │       │   └── targets
        │           │       │       └── ID <- 'start'
        │           │       ├── trans_opt
        │           │       │   └── trans
        │           │       │       ├── trans
        │           │       │       │   └── tran
        │           │       │       │       ├── mtcs
        │           │       │       │       │   └── mtc
        │           │       │       │       │       └── CILC <- ''a''
        │           │       │       │       ├── ARROW <- '->'
        │           │       │       │       └── trand
        │           │       │       │           └── acceptor
        │           │       │       │               ├── HAT <- '^'
        │           │       │       │               ├── id_or_def
        │           │       │       │               │   └── ID <- 'A'
        │           │       │       │               └── accd_opt
        │           │       │       │                   └──  <- 'NULL'
        │           │       │       └── tran
        │           │       │           ├── mtcs
        │           │       │           │   └── mtc
        │           │       │           │       └── CILC <- ''b''
        │           │       │           ├── ARROW <- '->'
        │           │       │           └── trand
        │           │       │               └── acceptor
        │           │       │                   ├── HAT <- '^'
        │           │       │                   ├── id_or_def
        │           │       │                   │   └── ID <- 'B'
        │           │       │                   └── accd_opt
        │           │       │                       └──  <- 'NULL'
        │           │       └── SEMI <- ';'
        │           └── RBRACE <- '}'
        └── region
            └── grammar
                ├── GRAMMAR <- 'grammar'
                ├── LBRACE <- '{'
                ├── prods
                │   └── prod
                │       ├── ID <- 's'
                │       ├── patt_opt
                │       │   └──  <- 'NULL'
                │       ├── rhss
                │       │   ├── rhss
                │       │   │   └── rhs
                │       │   │       ├── OR <- '|'
                │       │   │       ├── ids
                │       │   │       │   ├── ids
                │       │   │       │   │   ├── ids
                │       │   │       │   │   │   ├── ids
                │       │   │       │   │   │   │   └──  <- 'NULL'
                │       │   │       │   │   │   └── ID <- 'A'
                │       │   │       │   │   └── COPTID <- '[B]'
                │       │   │       │   └── ID <- 's'
                │       │   │       └── patt_opt
                │       │   │           └──  <- 'NULL'
                │       │   └── rhs
                │       │       ├── OR <- '|'
                │       │       ├── ids
                │       │       │   └──  <- 'NULL'
                │       │       └── patt_opt
                │       │           └──  <- 'NULL'
                │       └── SEMI <- ';'
                └── RBRACE <- '}'"
        );
    }

    #[test]
    fn single_reference_optional_shorthand() {
        //setup
        let spec = "
alphabet 'ab'

cdfa {
    start
        'a' -> ^A
        'b' -> ^B;
}

grammar {
    s
        | A [B] s
        |;
}
        ";

        let input = "ababaaaba".to_string();
        let mut iter = input.chars();
        let mut getter = || {
            iter.next()
        };
        let mut stream: StreamSource<char> = StreamSource::observe(&mut getter);

        let scanner = scan::def_scanner();
        let tree = lang::parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, grammar, _) = generate_spec(&parse).unwrap();
        let parser = parse::def_parser();

        //exercise
        let tokens = scanner.scan(&mut stream, &cdfa).unwrap();
        let tree = parser.parse(tokens, &grammar).unwrap();

        //verify
        assert_eq!(tree.to_string(),
                   "└── s
    ├── A <- 'a'
    ├── opt#B
    │   └── B <- 'b'
    └── s
        ├── A <- 'a'
        ├── opt#B
        │   └── B <- 'b'
        └── s
            ├── A <- 'a'
            ├── opt#B
            │   └──  <- 'NULL'
            └── s
                ├── A <- 'a'
                ├── opt#B
                │   └──  <- 'NULL'
                └── s
                    ├── A <- 'a'
                    ├── opt#B
                    │   └── B <- 'b'
                    └── s
                        ├── A <- 'a'
                        ├── opt#B
                        │   └──  <- 'NULL'
                        └── s
                            └──  <- 'NULL'"
        );
    }

    #[test]
    fn def_pattern() {
        //setup
        let spec = "
alphabet 'ab'

cdfa {
    start
        'a' -> ^A
        'b' -> ^B;
}

grammar {
    s `{} {}`
        | s A
        | s B
        | `SEPARATED:`;
}
        ";

        //exercise
        let tree = lang::parse_spec(spec).unwrap();

        //verify
        assert_eq!(tree.to_string(),
                   "└── spec
    └── regions
        ├── regions
        │   ├── regions
        │   │   └── region
        │   │       └── alphabet
        │   │           ├── ALPHABET <- 'alphabet'
        │   │           └── CILC <- ''ab''
        │   └── region
        │       └── cdfa
        │           ├── CDFA <- 'cdfa'
        │           ├── LBRACE <- '{'
        │           ├── states
        │           │   └── state
        │           │       ├── sdec
        │           │       │   └── targets
        │           │       │       └── ID <- 'start'
        │           │       ├── trans_opt
        │           │       │   └── trans
        │           │       │       ├── trans
        │           │       │       │   └── tran
        │           │       │       │       ├── mtcs
        │           │       │       │       │   └── mtc
        │           │       │       │       │       └── CILC <- ''a''
        │           │       │       │       ├── ARROW <- '->'
        │           │       │       │       └── trand
        │           │       │       │           └── acceptor
        │           │       │       │               ├── HAT <- '^'
        │           │       │       │               ├── id_or_def
        │           │       │       │               │   └── ID <- 'A'
        │           │       │       │               └── accd_opt
        │           │       │       │                   └──  <- 'NULL'
        │           │       │       └── tran
        │           │       │           ├── mtcs
        │           │       │           │   └── mtc
        │           │       │           │       └── CILC <- ''b''
        │           │       │           ├── ARROW <- '->'
        │           │       │           └── trand
        │           │       │               └── acceptor
        │           │       │                   ├── HAT <- '^'
        │           │       │                   ├── id_or_def
        │           │       │                   │   └── ID <- 'B'
        │           │       │                   └── accd_opt
        │           │       │                       └──  <- 'NULL'
        │           │       └── SEMI <- ';'
        │           └── RBRACE <- '}'
        └── region
            └── grammar
                ├── GRAMMAR <- 'grammar'
                ├── LBRACE <- '{'
                ├── prods
                │   └── prod
                │       ├── ID <- 's'
                │       ├── patt_opt
                │       │   └── PATTC <- '`{} {}`'
                │       ├── rhss
                │       │   ├── rhss
                │       │   │   ├── rhss
                │       │   │   │   └── rhs
                │       │   │   │       ├── OR <- '|'
                │       │   │   │       ├── ids
                │       │   │   │       │   ├── ids
                │       │   │   │       │   │   ├── ids
                │       │   │   │       │   │   │   └──  <- 'NULL'
                │       │   │   │       │   │   └── ID <- 's'
                │       │   │   │       │   └── ID <- 'A'
                │       │   │   │       └── patt_opt
                │       │   │   │           └──  <- 'NULL'
                │       │   │   └── rhs
                │       │   │       ├── OR <- '|'
                │       │   │       ├── ids
                │       │   │       │   ├── ids
                │       │   │       │   │   ├── ids
                │       │   │       │   │   │   └──  <- 'NULL'
                │       │   │       │   │   └── ID <- 's'
                │       │   │       │   └── ID <- 'B'
                │       │   │       └── patt_opt
                │       │   │           └──  <- 'NULL'
                │       │   └── rhs
                │       │       ├── OR <- '|'
                │       │       ├── ids
                │       │       │   └──  <- 'NULL'
                │       │       └── patt_opt
                │       │           └── PATTC <- '`SEPARATED:`'
                │       └── SEMI <- ';'
                └── RBRACE <- '}'"
        );
    }

    #[test]
    fn range_based_matchers() {
        //setup
        let spec = "
alphabet 'abcdefghijklmnopqrstuvwxyz'

cdfa {
    start
        'a'..'d' -> ^A
        'e'..'k' | 'l' -> ^B
        'm'..'m' -> ^C
        'n'..'o' -> ^D
        _ -> ^E;

    E
        'p'..'z' -> E;
}

grammar {
    s |;
}
        ";

        let input = "abcdefghijklmnopqrstuvwxyz".to_string();
        let mut iter = input.chars();
        let mut getter = || iter.next();
        let mut stream: StreamSource<char> = StreamSource::observe(&mut getter);

        let scanner = scan::def_scanner();
        let tree = lang::parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, _, _) = generate_spec(&parse).unwrap();

        //exercise
        let tokens = scanner.scan(&mut stream, &cdfa).unwrap();

        //verify
        let mut res_string = String::new();
        for token in tokens {
            res_string = format!("{}\nkind={} lexeme={}", res_string, token.kind, token.lexeme);
        }

        assert_eq!(res_string, "
kind=A lexeme=a
kind=A lexeme=b
kind=A lexeme=c
kind=A lexeme=d
kind=B lexeme=e
kind=B lexeme=f
kind=B lexeme=g
kind=B lexeme=h
kind=B lexeme=i
kind=B lexeme=j
kind=B lexeme=k
kind=B lexeme=l
kind=C lexeme=m
kind=D lexeme=n
kind=D lexeme=o
kind=E lexeme=pqrstuvwxyz")
    }

    #[test]
    fn context_sensitive_scanner() {
        //setup
        let spec = "
alphabet 'a!123456789'

cdfa {
    start
        'a' -> a
        '!' -> bang_in;

    bang_in ^BANG -> hidden;

    a       ^A
        'a' -> a;

    hidden
        '1' .. '9' -> num
        '!' -> ^BANG -> start;

    num     ^NUM;
}

grammar {
    s |;
}
        ";

        let input = "!!aaa!!a!49913!a".to_string();
        let mut iter = input.chars();
        let mut getter = || iter.next();
        let mut stream: StreamSource<char> = StreamSource::observe(&mut getter);

        let scanner = scan::def_scanner();
        let tree = lang::parse_spec(spec);
        let parse = tree.unwrap();
        let (cdfa, _, _) = generate_spec(&parse).unwrap();

        //exercise
        let tokens = scanner.scan(&mut stream, &cdfa).unwrap();

        //verify
        assert_eq!(tokens_string(tokens), "
kind=BANG lexeme=!
kind=BANG lexeme=!
kind=A lexeme=aaa
kind=BANG lexeme=!
kind=BANG lexeme=!
kind=A lexeme=a
kind=BANG lexeme=!
kind=NUM lexeme=4
kind=NUM lexeme=9
kind=NUM lexeme=9
kind=NUM lexeme=1
kind=NUM lexeme=3
kind=BANG lexeme=!
kind=A lexeme=a")
    }

    //TODO add test(s) for duplicated regions

    fn tokens_string(tokens: Vec<Token<String>>) -> String {
        let mut res_string = String::new();
        for token in tokens {
            res_string = format!("{}\nkind={} lexeme={}", res_string, token.kind, token.lexeme);
        }
        res_string
    }
}
