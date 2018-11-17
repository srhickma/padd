extern crate padd;

use padd::FormatJobRunner;
use padd::Stream;

#[test]
fn test_def_input_matcher() {
    //setup
    let spec = "
'abcdefghijklmnopqrstuvwxyz'

start 'a' -> a;

a^ACC
'a' -> fail
_ -> a;

s -> acc s `{}\\n{}`
-> acc;

acc -> ACC;
    ".to_string();

    let input = "abasdfergrthergerghera".to_string();
    let mut iter = input.chars();
    let mut getter = || iter.next();
    let mut stream = Stream::from(&mut getter);

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(&mut stream).unwrap();

    //verify
    assert_eq!(res,
               "ab
asdfergrthergergher
a"
    );
}

#[test]
fn test_ignore_tokens() {
    //setup
    let spec = "
'a \n\t'

start 'a' -> a
' ' | '\n' | '\t' -> ^_;

a^ACC
'a' -> a
_ -> fail;

s -> acc s `{} {}`
-> acc;

acc -> ACC;
    ".to_string();

    let input = "aaaa \t \n  a aa  aa ".to_string();
    let mut iter = input.chars();
    let mut getter = || iter.next();
    let mut stream = Stream::from(&mut getter);

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(&mut stream).unwrap();

    //verify
    assert_eq!(res, "aaaa a aa aa");
}

#[test]
fn test_advanced_operators() {
    //setup
    let spec = "
        'inj '
        start
            'in' -> ^IN
            ' ' -> ^_
            _ -> ^ID;
        ID | IN
            ' ' -> fail
            _ -> ID;
        s
            -> x s
            -> x;
        x
            -> ID
            -> IN ``;".to_string();

    let input = "i ij ijjjijijiji inj in iii".to_string();
    let mut iter = input.chars();
    let mut getter = || iter.next();
    let mut stream = Stream::from(&mut getter);

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(&mut stream).unwrap();

    //verify
    assert_eq!(res, "iijijjjijijijiinjiii");
}

#[test]
fn test_single_reference_optional_shorthand() {
    //setup
    let spec = "
'ab'

start
  'a' -> ^A
  'b' -> ^B;

s -> A [b] s
  ->;

b -> B `\n{}\n`;
    ".to_string();

    let input = "ababaaaaababaaba".to_string();
    let mut iter = input.chars();
    let mut getter = || iter.next();
    let mut stream = Stream::from(&mut getter);

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(&mut stream).unwrap();

    //verify
    assert_eq!(res, "a\nb\na\nb\naaaaa\nb\na\nb\naa\nb\na");
}

#[test]
fn test_multiple_reference_optional_shorthand() {
    //setup
    let spec = "
'ab'

start
  'a' -> ^A
  'b' -> ^B;

s -> A [b] s
  ->;

b -> B [b] `\n{} {}`;
    ".to_string();

    let input = "abbbabaaaabbbbababaaba".to_string();
    let mut iter = input.chars();
    let mut getter = || iter.next();
    let mut stream = Stream::from(&mut getter);

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(&mut stream).unwrap();

    //verify
    assert_eq!(res, "a\nb \nb \nb a\nb aaaa\nb \nb \nb \nb a\nb a\nb aa\nb a");
}

#[test]
fn test_optional_shorthand_state_order() {
    //setup
    let spec = "
'ab'

start
  'a' -> ^A
  'b' -> ^B;

s -> [A] [B];
    ".to_string();

    let input = "ab".to_string();
    let mut iter = input.chars();
    let mut getter = || iter.next();
    let mut stream = Stream::from(&mut getter);

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(&mut stream).unwrap();

    //verify
    assert_eq!(res, "ab");
}

#[test]
fn test_escapable_patterns() {
    //setup
    let spec = "
' \t\n{}'

# dfa
start
 ' ' | '\t' | '\n' -> ^_
 '{' -> ^LBRACKET
 '}' -> ^RBRACKET;

# grammar
s -> s b `\\\\[@LAYER s\\\\={} b\\\\={}\\\\]`
  ->;
b -> LBRACKET s RBRACKET `[prefix]{}{;prefix=[prefix]\t}[prefix]{}`;
    ".to_string();

    let input = " {{} }  { {}}".to_string();
    let mut iter = input.chars();
    let mut getter = || iter.next();
    let mut stream = Stream::from(&mut getter);

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(&mut stream).unwrap();

    //verify
    assert_eq!(res, "[@LAYER s=[@LAYER s= b={[@LAYER s= b=\t{\t}]}] b={[@LAYER s= b=\t{\t}]}]");
}
