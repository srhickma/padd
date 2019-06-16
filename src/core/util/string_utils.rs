/// Returns `input` with all backslash-escaped characters replaced, i.e. \n, \t, \\ are
/// replaced by "newline", "tab", and "\" characters, and all other backslashes are simply
/// removed.
pub fn replace_escapes(input: &str) -> String {
    let mut res = String::with_capacity(input.as_bytes().len());
    let mut last_char: char = ' ';
    for (i, c) in input.chars().enumerate() {
        let mut hit_double_slash = false;
        if i != 0 && last_char == '\\' {
            res.push(match c {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
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
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_escapes_empty() {
        //setup
        let input = "";

        //exercise
        let res = replace_escapes(input);

        //verify
        assert_eq!(res, "");
    }

    #[test]
    fn replace_escapes_single() {
        //setup
        let input = "\\n";

        //exercise
        let res = replace_escapes(input);

        //verify
        assert_eq!(res, "\n");
    }

    #[test]
    fn replace_escapes_chained() {
        //setup
        let input = "\\\\n\\n\\\\\\t";

        //exercise
        let res = replace_escapes(input);

        //verify
        assert_eq!(res, "\\n\n\\\t");
    }

    #[test]
    fn replace_escapes_full() {
        //setup
        let input = "ffffnt\'ff\\n\\t\\\\\\\\ffff\\ff\'\\f\\\'fff\\r";

        //exercise
        let res = replace_escapes(input);

        //verify
        assert_eq!(res, "ffffnt\'ff\n\t\\\\ffffff\'f\'fff\r");
    }
}
