[![Build Status](https://travis-ci.com/srhickma/padd.svg?branch=master)](https://travis-ci.com/srhickma/padd)
[![Test Coverage](https://codecov.io/gh/srhickma/padd/branch/master/graph/badge.svg)](https://codecov.io/gh/srhickma/padd)

# padd
Fast and Automatic Formatting of Context-Free Languages. Inspired by the automatic formatting of 
Golang.

## Goal
The goal of padd is to act as a generic language formatter, capable of formatting any language that can be represented with a context-free grammar. A single specification file is used to specify how a language should be lexed, parsed, and then formatted during reconstruction.

The initial purpose of padd was as a whitespace formatter for programming languages, but can also be used for any CFGs.

## Project Structure
The main library source is located under `src/core`, and the cli specific source is located under `src/cli`.
Integration tests and example specifications are located under `tests`.

## Formatter Specifications
The padd formatter uses a specification language (defined [here](https://github.com/srhickma/padd/blob/master/src/core/spec/lang.rs)) to specify the alphabet of a language, a compressed DFA ([CDFA](https://github.com/srhickma/padd/wiki/CDFA-Specification)) to lex the language, a [grammar](https://github.com/srhickma/padd/wiki/Grammar-Specification) to parse it, and optional [formatter patterns](https://github.com/srhickma/padd/wiki/Formatter-Patterns) inside the grammar to indicate how the finished parse tree should be reconstructed. Example specifications can be found [here](https://github.com/srhickma/padd/tree/master/tests/spec), and more information about specifications can be found [here](https://github.com/srhickma/padd/wiki/Specifications).

## CLI Usage
The `padd` cli can be used to format files or directories in place, overwriting the existing files if formatting is successfull. For more advanced usage information, see [CLI-Usage](https://github.com/srhickma/padd/wiki/CLI-Usage).

### Simple Formatting
```shell
$ ./padd fmt <specification file> -t <target path>
```
### Example
Format all `*.java` files in `~/some-java-project` on 4 worker threads using the java8 specification:
```shell
$ ./padd fmt tests/spec/java8 -t ~/some-java-project --threads 4 -m ".*\.java"
```

## Library Usage
```rust
extern crate padd;

use padd::{FormatJobRunner, FormatJob};

fn main() {
    // Specification String
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
    ".to_string();

    let input = "abbaba".to_string();

    // Formatter Creation
    let fjr = FormatJobRunner::build(&spec).unwrap();

    // Format Input
    let res = fjr.format(FormatJob::from_text(input)).unwrap();

    // Verify Output
    assert_eq!(res, "SEPARATED: a b b a b a");
}
```

## Simple Example: Balanced Brackets
The specification file:
```
alphabet ' \t\n{}'

cdfa {
    start
        ' ' | '\t' | '\n' -> ^_
        '{' -> ^LBRACKET
        '}' -> ^RBRACKET;
}

grammar {
    s
        | s b
        |;

    b
        | LBRACKET s RBRACKET `[prefix]{}\n\n{;prefix=[prefix]\t}[prefix]{}\n\n`;
}
```
The input:
```
  {  {  {{{ }}}
   {} }  }   { {}
    }
```
The output:
```txt
{

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

}
```
