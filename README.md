[![Build Status](https://travis-ci.org/srhickma/padd.svg?branch=master)](https://travis-ci.org/srhickma/padd)
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

## CLI Usage
By default, the padd cli will overwrite files in place if they are formatted successfully.

### Single File Formatting
```shell
$ ./padd <spec file> -t <target file>
```
### Directory/Project Formatting
Without file name matching (formats every file):
```shell
$ ./padd <spec file> -d <directory>
```
With file name matching:
```shell
$ ./padd <spec file> -d <directory> -m <regex>
```
### Interactive Mode
```shell
$ ./padd <spec file>
```
The cli will stay active and you can type file paths separated by newlines, and they will be formatted.

## Library Usage
### Simple Formatter Usage
```rust
extern crate padd;

use padd::FormatJobRunner;

fn main() {
    //Specification String
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

    //Formatter Creation
    let fjr = FormatJobRunner::build(&spec).unwrap();

    //Format Input String
    let res = fjr.format(&"i ij ijjjijijiji inj in iii".to_string()).unwrap();

    //Verify Output
    assert_eq!(res, "iijijjjijijijiinjiii");
}
```

## Simple Example: Balanced Brackets
The specification file:
```
# sigma
' \t\n{}'

# dfa
start
 ' ' | '\t' | '\n' -> ^_
 '{' -> ^LBRACKET
 '}' -> ^RBRACKET;

# grammar
s -> s b
  ->;
b -> LBRACKET s RBRACKET `[prefix]{0}\n\n{1;prefix=[prefix]\t}[prefix]{2}\n\n`;
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

## More Examples
For more example specifications, look in `tests/spec/`.
