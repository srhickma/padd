pub fn replace_escapes(input: &str) -> String {
    let mut res = String::with_capacity(input.as_bytes().len());
    let mut i = 0;
    let mut last_char: char = ' ';
    for c in input.chars() {
        let mut hit_double_slash = false;
        if i != 0 && last_char == '\\' {
            res.push(match c {
                'n' => '\n',
                't' => '\t',
                '\'' => '\'',
                '\\' => {
                    last_char = ' '; //Stop \\\\ -> \\\ rather than \\
                    hit_double_slash = true;
                    '\\'
                }
                _ => c,
            });
        } else if c != '\\' {
            res.push(c);
        }
        if !hit_double_slash {
            last_char = c;
        }
        i += 1;
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_escapes_full() {
        //setup
        let input = "ffffnt\'ff\\n\\t\\\\\\\\ffff\\ff\'\\f\\\'fff";

        //exercise
        let res = replace_escapes(input);

        //verify
        assert_eq!(res, "ffffnt\'ff\n\t\\\\ffffff\'f\'fff");
    }
}
