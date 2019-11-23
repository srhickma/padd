[![Codacy Badge](https://api.codacy.com/project/badge/Grade/42b5507d0a5949ada8f63c1fcc1021e6)](https://www.codacy.com/manual/srhickma/padd?utm_source=github.com&amp;utm_medium=referral&amp;utm_content=srhickma/padd&amp;utm_campaign=Badge_Grade)
[![Build Status](https://travis-ci.com/srhickma/padd.svg?branch=master)](https://travis-ci.com/srhickma/padd)
[![Test Coverage](https://codecov.io/gh/srhickma/padd/branch/master/graph/badge.svg)](https://codecov.io/gh/srhickma/padd)
[![Documentation](https://img.shields.io/badge/docs-mkdocs-blue)](https://padd.srhickma.dev)

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
The padd formatter uses a specification language (defined [here](https://github.com/srhickma/padd/blob/master/src/core/spec/lang.rs)) to specify the alphabet of a language, a compressed DFA ([CDFA](https://padd.srhickma.dev/spec/cdfa/)) to lex the language, a [grammar](https://padd.srhickma.dev/spec/grammar/) to parse it, and optional [formatter patterns](https://padd.srhickma.dev/spec/pattern/) inside the grammar to indicate how the finished parse tree should be reconstructed. Example specifications can be found [here](https://github.com/srhickma/padd/tree/master/tests/spec), and more information about specifications can be found [here](https://padd.srhickma.dev/spec/).

## CLI Usage
The `padd` cli can be used to format files or directories in place, overwriting the existing files if formatting is successfull. For more advanced usage information, see the [docs](https://padd.srhickma.dev/cli/).

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
```txt
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
