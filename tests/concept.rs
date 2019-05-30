extern crate padd;

use padd::{FormatJob, FormatJobRunner};

#[test]
fn test_def_input_matcher() {
    //setup
    let spec = "
alphabet 'abcdefghijklmnopqrstuvwxyz'

cdfa {
    start
        'a' -> a;

    a   ^ACC
        'a' -> fail
        _ -> a;
}

grammar {
    s
        | acc s `{}\\n{}`
        | acc;

    acc
        | ACC;
}
    "
    .to_string();

    let input = "abasdfergrthergerghera".to_string();

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(FormatJob::from_text(input)).unwrap();

    //verify
    assert_eq!(
        res,
        "ab
asdfergrthergergher
a"
    );
}

#[test]
fn test_ignore_tokens() {
    //setup
    let spec = "
alphabet 'a \n\t'

cdfa {
    start
        'a' -> a
        ' ' | '\n' | '\t' -> ^_;

    a   ^ACC
        'a' -> a
        _ -> fail;
}

grammar {
    s
        | acc s `{} {}`
        | acc;

    acc
        | ACC;
}
    "
    .to_string();

    let input = "aaaa \t \n  a aa  aa ".to_string();

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(FormatJob::from_text(input)).unwrap();

    //verify
    assert_eq!(res, "aaaa a aa aa");
}

#[test]
fn test_advanced_operators() {
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
    s
        | x s
        | x;
    x
        | ID
        | IN ``;
}
    "
    .to_string();

    let input = "i ij ijjjijijiji inj in iii".to_string();

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(FormatJob::from_text(input)).unwrap();

    //verify
    assert_eq!(res, "iijijjjijijijiinjiii");
}

#[test]
fn test_single_reference_optional_shorthand() {
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
        | A [b] s
        |;

    b
        | B `\n{}\n`;
}
    "
    .to_string();

    let input = "ababaaaaababaaba".to_string();

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(FormatJob::from_text(input)).unwrap();

    //verify
    assert_eq!(res, "a\nb\na\nb\naaaaa\nb\na\nb\naa\nb\na");
}

#[test]
fn test_multiple_reference_optional_shorthand() {
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
        | A [b] s
        |;

    b
        | B [b] `\n{} {}`;
}
    "
    .to_string();

    let input = "abbbabaaaabbbbababaaba".to_string();

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(FormatJob::from_text(input)).unwrap();

    //verify
    assert_eq!(
        res,
        "a\nb \nb \nb a\nb aaaa\nb \nb \nb \nb a\nb a\nb aa\nb a"
    );
}

#[test]
fn test_optional_shorthand_state_order() {
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
        | [A] [B];
}
    "
    .to_string();

    let input = "ab".to_string();

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(FormatJob::from_text(input)).unwrap();

    //verify
    assert_eq!(res, "ab");
}

#[test]
fn test_escapable_patterns() {
    //setup
    let spec = "
alphabet ' \t\n{}'

cdfa {
    start
        ' ' | '\t' | '\n' -> ^_
        '{' -> ^LBRACKET
        '}' -> ^RBRACKET;
}

grammar {
    s
        | s b `\\\\[@LAYER s\\\\={} b\\\\={}\\\\]`
        |;

    b
        | LBRACKET s RBRACKET `[prefix]{}{;prefix=[prefix]\t}[prefix]{}`;
}
    "
    .to_string();

    let input = " {{} }  { {}}".to_string();

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(FormatJob::from_text(input)).unwrap();

    //verify
    assert_eq!(
        res,
        "[@LAYER s=[@LAYER s= b={[@LAYER s= b=\t{\t}]}] b={[@LAYER s= b=\t{\t}]}]"
    );
}

#[test]
fn test_def_non_terminal_pattern() {
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
    "
    .to_string();

    let input = "abbaba".to_string();

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(FormatJob::from_text(input)).unwrap();

    //verify
    assert_eq!(res, "SEPARATED: a b b a b a");
}

#[test]
fn test_range_based_matcher() {
    //setup
    let spec = "
alphabet 'abcdefghijklmnopqrstuvwxyz'

cdfa {
    start
        'a' .. 'k' -> ^FIRST
        'l' .. 'z' -> ^LAST;
}

grammar {
    s
        | first last `{1} {0}`;

    first
        | first FIRST
        | FIRST;

    last
        | last LAST
        | LAST;
}
    "
    .to_string();

    let input = "abcdefghijklmnopqrstuvwxyz".to_string();

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(FormatJob::from_text(input)).unwrap();

    //verify
    assert_eq!(res, "lmnopqrstuvwxyz abcdefghijk");
}

#[test]
fn test_online_context_sensitive_scanner() {
    //setup
    let spec = "
alphabet 'a!123456789'

cdfa {
    start
        'a' -> a
        '!' -> ^_ -> hidden;

    a       ^A
        'a' -> a;

    hidden
        '1' .. '9' -> num
        '!' -> ^_ -> start;

    num     ^NUM
        '1' .. '9' -> num;
}

grammar {
    s
        | [regions];

    regions
        | regions region `{} {}`
        | region;

    region
        | A
        | NUM;
}
    "
    .to_string();

    let input = "!!aaa!!a!49913!a".to_string();

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(FormatJob::from_text(input)).unwrap();

    //verify
    assert_eq!(res, "aaa a 49913 a");
}

#[test]
fn test_region_based_scanner() {
    //setup
    let spec = "
alphabet 'abcdefghijklmnopqrstuvwxyz0123456789{}'

cdfa {
    start
        'region1' -> ^R1_DEC -> r1_dec
        'region2' -> ^R2_DEC -> r2_dec;

    r1_dec
       '{' -> ^LBRACE_R1 -> r1_body;

    r2_dec
       '{' -> ^LBRACE_R2 -> r2_body;

    r1_body
        'a' -> ^A
        'b' -> ^B
        '}' -> ^RBRACE -> start;

    r2_body
        '0' .. '9' -> num
        '}' -> ^RBRACE -> start;

    num     ^NUM
        '0' .. '9' -> num;
}

grammar {
    s
        | [regions];

    regions
        | regions region `{}\\n{}`
        | region;

    region
        | region1
        | region2;

    region1
        | R1_DEC LBRACE_R1 abs RBRACE `{} {}\\n\\t{}\\n{}`;

    abs
        | abs A
        | abs B
        | ;

    region2
        | R2_DEC LBRACE_R2 [NUM] RBRACE `{} {}\\n\\t{}\\n{}`;
}
    "
    .to_string();

    let input = "region1{abaaba}region1{bb}region2{558905}".to_string();

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(FormatJob::from_text(input)).unwrap();

    //verify
    assert_eq!(
        res,
        "region1 {
\tabaaba
}
region1 {
\tbb
}
region2 {
\t558905
}"
    );
}

#[test]
fn test_duplicate_spec_regions() {
    //setup
    let spec = "
alphabet 'something else'

cdfa {
    start
        'in' -> ^IN
        ' ' -> ^_
        _ -> ^ID;
}

grammar {
    s
        | x s
        | x;
}

cdfa {
    ID | IN
        ' ' -> fail
        _ -> ID;
}

grammar {
    x
        | ID
        | IN ``;
}

alphabet 'inj '
    "
    .to_string();

    let input = "i ij ijjjijijiji inj in iii".to_string();

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(FormatJob::from_text(input)).unwrap();

    //verify
    assert_eq!(res, "iijijjjijijijiinjiii");
}

#[test]
fn test_ignorable_terminals() {
    //setup
    let spec = "
alphabet 'abc'

cdfa {
    start
        'a' -> ^A
        'b' -> ^B
        'c' -> ^C;
}

ignore C

grammar {
    s
        | A s B `{} {} {}`
        | C;
}
    "
    .to_string();

    let input = "caacaccccbccbcbc".to_string();

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(FormatJob::from_text(input)).unwrap();

    //verify
    assert_eq!(res, "a a a c b b b");
}

#[test]
fn test_multiple_ignorable_terminals() {
    //setup
    let spec = "
alphabet 'abc'

cdfa {
    start
        'a' -> ^A
        'b' -> ^B
        'c' -> ^C;
}

ignore B
ignore C

grammar {
    s
        | A s B `{} {} {}`
        | C;
}
    "
    .to_string();

    let input = "bcababcacbcbcbcbcbcbbcbcb".to_string();

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(FormatJob::from_text(input)).unwrap();

    //verify
    assert_eq!(res, "a a a c b b b");
}

#[test]
fn test_inject_terminals_simple() {
    //setup
    let spec = "
alphabet 'abc'

cdfa {
    start
        'a' -> ^A
        'b' -> ^B
        'c' -> ^C;
}

inject left C ` <{}>`

grammar {
    s
        | A s B `{} {} {}`
        | ;
}
    "
    .to_string();

    let input = "acb".to_string();

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(FormatJob::from_text(input)).unwrap();

    //verify
    assert_eq!(res, "a <c>  b");
}

#[test]
fn test_inject_terminals_complex() {
    //setup
    let spec = "
alphabet 'abcd'

cdfa {
    start
        'a' -> ^A
        'b' -> ^B
        'c' -> ^C
        'd' -> ^D;
}

inject left C ` <{}>`
inject right D

grammar {
    s
        | A s d B `{} {}{} {}`
        | ;

    d
        | d D ` \\\\[{1}\\\\] `
        | ;
}
    "
    .to_string();

    let input = "cdaadaacbbcdbbdcd".to_string();

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(FormatJob::from_text(input)).unwrap();

    //verify
    assert_eq!(res, " <c>da a da a <c>  b b <c> [d]  b bd <c>d");
}

#[test]
fn test_inject_terminals_empty() {
    //setup
    let spec = "
alphabet 'a'

cdfa {
    start
        'a' -> ^A;
}

inject left A

grammar {
    s | ;
}
    "
    .to_string();

    let input = "a".to_string();

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(FormatJob::from_text(input)).unwrap();

    //verify
    assert_eq!(res, "a");
}

#[test]
fn test_ignored_terminal_injection() {
    //setup
    let spec = "
alphabet 'ab'

cdfa {
    start
        'a' -> ^A
        'b' -> ^B;
}

inject right B

grammar {
    s
        | A A `{}`;
}
    "
    .to_string();

    let input = "babab".to_string();

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(FormatJob::from_text(input)).unwrap();

    //verify
    assert_eq!(res, "bab");
}

#[test]
fn test_non_consuming_transitions() {
    //setup
    let spec = "
alphabet 'abc'

cdfa {
    start1
        'a' -> ^A1
        'b' ->> start2
        _ ->> start3;

    start2
        'b' -> b;

    b   ^B2
        'c' ->> start3;


    start3
        'c' -> ^C3;
}

grammar {
    s
        | s x `{} {}`
        | x;

    x
        | A1 `A1\\\\{{}\\\\}`
        | B2 `B2\\\\{{}\\\\}`
        | C3 `C3\\\\{{}\\\\}`;
}
    "
    .to_string();

    let input = "abcacb".to_string();

    let fjr = FormatJobRunner::build(&spec).unwrap();

    //exercise
    let res = fjr.format(FormatJob::from_text(input)).unwrap();

    //verify
    assert_eq!(res, "A1{a} C3{bc} A1{a} C3{c} B2{b}");
}
