# Language Specification

Specifications determine how some input text should be formatted, primarily through the definition of a lexer, parser,
and a set of patterns.
The lexer and parser are used to specify the language being formatted, and patterns are used to specify how a parse tree
in the language should be flattened back into a string.

Specifications are used either in the form of a string (with the library) or as a file (with the cli).

---

## Regions

Specifications are split into multiple "regions", for the purposes of easier lexing and improved logical separation.
Regions are typically made up of either a single statement, or a sequence of statements enclosed in braces.

### Alphabet
The [alphabet](alphabet.md) region is used to specify an alphabet for the language lexer, which enforces what characters
are allowed in the language.

### CDFA
The [CDFA](cdfa.md) region is used to specify the lexer itself, using a structure resembling a deterministic finite
automata.

### Grammar
The [grammar](grammar.md) region is used to specify the context-free grammar with which to parse the language, as well
as the [patterns](pattern.md) to use when formatting parse trees.

### Ignorable and Injectable Definitions
The [ignorable](ignorable.md) and [injectable](injectable.md) regions are used to mark terminal symbols which can be
omitted in the grammar, and will either be ignored in the resulting parse tree (ignorable) or injected into the parse
tree and included in formatting (injectable).

### Required vs. Optional
The CDFA and grammar regions are the only required regions, and at least one lexer state and grammar production must be
defined.
All other regions are optional, and can be omitted without error.

---

## Examples
Example specifications can be found under [`tests/spec/`](https://github.com/srhickma/padd/tree/master/tests/spec).
The [json](https://github.com/srhickma/padd/blob/master/tests/spec/json) and
[trailing_whitespace](https://github.com/srhickma/padd/blob/master/tests/spec/trailing_whitespace) specifications are
good examples of fairly simple (but still useful) specifications, which are great for getting started.
The [java8](https://github.com/srhickma/padd/blob/master/tests/spec/java8) specification is a good example of a
formatter for a full programming language, and provides a useful reference for complex topics like formatting comments,
line breaking, and enforcing non-whitespace code conventions.

For more specific examples of how each specification feature works, a good reference is the
[concept.rs](https://github.com/srhickma/padd/blob/master/tests/concept.rs) file, which contains feature-specific
integration tests.

For help resolving errors encountered when creating specifications, a good first resource is the 
[lib.rs](https://github.com/srhickma/padd/blob/master/src/lib.rs) file, which contains many failure scenario tests.
