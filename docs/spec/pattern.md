# Formatting Patterns

Patterns are used to specify how a particular node in a parse tree should be formatted back into a string.
When a parse tree node is constructed, the grammar production which was used to build the node is stored, so the
formatter can find and use the pattern assigned to that production.

Patterns consist of a simple string of text, enclosed between back-ticks, which is made up of filler characters,
variable substitutions, and subtree captures.

---

## Filler
Filler text consists of any characters which should be placed in the formatted string at their relative positions in the
pattern.

**Example:** Suppose the pattern for production `p` is `abc`.
If a parse tree node constructed from `p` is formatted, the result string will be `abc`.
Filler text is not entirely useful on its own, but we will see how it becomes useful in combination with other pattern
segments in the [examples](#examples) section.

### Escaping

The following characters can be escaped in a pattern:

| Pattern String | Resulting Character |
|----------------|---------------------|
| \n             | newline             |
| \t             | tab                 |
| \r             | carriage return     |
| \\\\           | backslash           |

If any other characters are prefixed by a backslash, the backslash will be ignored. Note that newlines, tabs, and
carriage returns can be included directly in the pattern string, however it is preferred to use the escaped versions
for better readability.

---

## Substitution
Substitution segments are used to "inject" the current value of a formatting variable into the formatted string at
runtime.
Substitution is denoted by `[VARIABLE]` where `VARIABLE` is the identifier of the variable whose value should be used.

**Example:** Suppose the pattern for production `p` is `abc[x]`.
If a parse tree node constructed from `p` is formatted, the resulting string will be `abcy`, where `y` is the
value of variable `x` in the current scope.

**Note:** If a variable is substituted before it is defined in a capture, the empty string will be substituted in
its place.

---

## Captures
Capture segments are used to insert the formatted string of a child node in the parse tree, and to update the variable
scope used when formatting the child node.
Captures are denoted by `{...}` where `...` is replaced with the capture information.

### Simple Capture
The simplest form of a capture segment is when a child node is captured based on its index in the right-hand-side of
of the pattern's associated production.
This is achieved by inserting `{x}` into a pattern, where `x` is the index (starting at 0) of the child node to be
captured.
An error is emitted if the index is out of bounds.

**Example:** Suppose the pattern for production `p`, with right-hand-side `A B`, is `first={0} second={1}`.
If a parse tree node constructed from `p` whose `A` child has formatted string `a` and whose `B` child has formatted
string `b`, then the resulting formatted string for the node will be `first=a second=b`.

**Note:** A single child can be captured in a pattern more than once, however the string will be recomputed for each
capture.
This is necessary since the variable scope may differ between captures, however an optimization could be made to cache
formatted strings of captures with identical scopes.

### Implicit Indices
In all but a few cases it is desirable to format all children of a node, maintaining their order in the parse tree.
In this case no index specification is required, and one can simply insert `{}`.
The index will implicitly be assigned as the number captures already seen in the pattern, hence the first capture will
be assigned index 0, the second index 1, and so on.

**Example:** The above simple captures example can be simplified to `first={} second={}`.

**Note:** Simple captures and those with implicit indices can be mixed, however this can be difficult to read.
As a rule of thumb, if any index must be specified, all the rest should be as well.

**Example:** The pattern `{2}{}{}` is functionally equivalent to `{2}{1}{2}`.

### Variable Definition
Formatting variables are assigned inside captures, and have the syntax `{x;v1=p1;v2=p2;...}`, where `x` is the optional
child index, `vi` is a variable name, and `pi` is the pattern to evaluate to determine the value of `vi`.
The `pi` patterns can contain filler and substitutions but not captures, since there are no children to be captured.

**Example:** Suppose the pattern `{;x=one} {;x=two}` is used to format production `prod = s | rule rule`, and pattern
`[x]` is used to format `rule`.
Then any parse tree constructed from `prod` will be formatted into the string `one two`.

---

## Examples
**Example:** A simple specification which illustrates some properties of patterns is the following:
```text
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
```
This specification will parse a sequence of `a`s and `b`s and format them separated by spaces, with a prefix of
`SEPARATED:`.
So an input of `abbaba` will be formatted as `SEPARATED: a b b a b a`.

**Example:** The following snippet illustrates how code indentation can be maintained using a `prefix` variable,
representing the currently desired indentation, and an `indent` variable, representing the desired indentation increment:
```text
file
    | definitions `{;indent=    }`;
...
block
    | LBRACE statements RBRACE `{}\n{;prefix=[prefix][indent]}[prefix]{}`;
...
statement `[prefix]{}`
    | variable_declaration
    | block
    | ...;
```
Using this approach, each statement will be indented with `n` repetitions of `indent`, where `n` is the level of
block-nesting the statement occurs in.

**Example:** On a similar topic, one can use patterns to enforce that statements (e.g. if statements) are _not_ inlined:
```text
if_statement
    | IF LPAREN expr RPAREN forced_statement `{} {}{}{} {}`;
...
forced_statement
    | block
    | inline_statement `\{\n[prefix][indent]{;prefix=[prefix][indent]}\n[prefix]\}`;
```
The above patterns will effectively add braces surrounding any inlined if-statements, such that `if (x) y;` will be
converted into:
```text
if (x) {
    y;
}
```
Of course, the above specification leaves out much of the detail, however it is the `forced_statement` pattern which is
most interesting.

For more examples, take a look at the [concept.rs](https://github.com/srhickma/padd/blob/master/tests/concept.rs)
integration tests, and the [example specifications](https://github.com/srhickma/padd/tree/master/tests/spec).
