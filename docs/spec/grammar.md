# Grammar

The grammar specification region is used to define the grammar which will be used to parse the tokens produced by the
CDFA, as well as the [patterns](pattern.md) used to format the resulting parse tree.

---

## Grammar Region

The grammar region is a required region in specifications, and has the form:
```text
grammar {
    # Production definitions
}
```
The contents of a grammar region is a set of production definitions, which determine the productions of the grammar.
The first production left-hand-side declared in the first grammar region will be used as the starting symbol of the
grammar.

Multiple grammar regions can be used in a single specification, and the result will be as if all production definitions
were concatenated into a single region.
For example, the following two specifications are equivalent:
```text
grammar {
    # Production 'a'
}
...
grammar {
    # Production 'b'
}
```
```text
grammar {
    # Production 'a'
    # Production 'b'
}
```

---

## Basic Syntax
A grammar region consists of definitions of productions using the following syntax:
```text
LHS_SYMBOL
    | RHS_SYMBOL_1 RHS_SYMBOL_2 ...
    ...
    | ... RHS_SYMBOL_X;
```
Where the `LHS_SYMBOL` is the left-hand-side symbol for all productions in the definition, and the symbols following
each `|` are the right-hand-sides.

**Example:**
```text
lhs_symbol
    | rhs_symbol_1 rhs_symbol_2
    | rhs_symbol_3;
```
This example results in the following BNF grammar:
```text
lhs_symbol ::= rhs_symbol_1 rhs_symbol_2 | rhs_symbol_3
```

**Note:** Extra newlines and other whitespace between tokens is ignored when parsing grammars, so the above example
could be represented equivalently as `lhs_symbol | rhs_symbol_1 rhs_symbol_2 | rhs_symbol_3;`, however it is convention
to separate production right-hand-sides onto separate lines.

### Epsilon Productions
Epsilon productions allow a non-terminal symbol to be completed without consuming any tokens.
In other words, an epsilon productions specifies that the left-hand-side symbol is _nullable_ in the grammar.
Epsilon productions can be specified using the pipe symbol `|` followed only by whitespace until the next `|` or `;`.

**Example:** The following productions allow `lines` to represent a sequence one or more `LINE` tokens:
```text
lines
    | lines LINE
    |;
```
This example illustrates how epsilon productions are _one_ way of representing lists via recursion.

---

## Optional Shorthand
It is quite common to have grammar productions in which a particular symbol is optional, meaning the rule can be parsed
with or without the symbol being completed.
Typically this would be implemented using the following technique:
```text
grammar {
    s
        | a_opt;

    a_opt
        | a
        |;
}
```
To simplify this concept, the shorthand `[SYMBOL]` can be used in a production right-hand-side to specify that `SYMBOL`
may optionally be parsed at that position.

**Example:** The above example could be re-written using optional shorthand as follows:
```text
grammar {
    s
        | [a];
}
```

Internally this is handled by creating an optional state similar to the `a_opt` state shown above.
If `[SYMBOL]` is used multiple times for some `SYMBOL`, only a single internal optional state will be created and used.

---

## Inline Lists
The typical method for representing lists or repetitions in a context-free grammar is via recursion.
While this method is simple, very long lists will cause the parse tree to be very deep, nagatively impacting performance
and increasing the risk of a stack overflow during parsing (or formatting).
To avoid this, lists can be represented in a production using "inline" syntax as `{SYMBOL}` where `SYMBOL` is any symbol
which repeats at least once.

**Example:** The following example (taken from the
[trailing whitespace eliminator](https://github.com/srhickma/padd/blob/master/tests/spec/trailing_whitespace))
illustrates how to parse a file into lines (and newlines) using an inline list:
```text
grammar {
    file
        | {line}
        |;

    line
        | LINE
        | NEWLINE `\n`;
}
```

Inline list elements will be formatted by simply concatenating the formatted string for each of the lists elements.
When an inline list is [captured](pattern.md#captures) in a pattern, the formatted string of the entire list is inserted
at the capture point.

---

## Patterns
Formatting [patterns](pattern.md) are specified in the grammar region, since patterns are mapped to specific
productions, or sets of productions.
The pattern of a production determines how any subtree of the parse produced by that production will be formatted.
If a production does not receive any pattern, a default pattern in assigned, which simply concatenates the formatted
strings of all children in order.
The following are the two methods for assigning patterns to productions:

### Production Patterns
Patterns can be specified for a single production in a grammar by including the pattern directly after the list of
right-hand-side symbols of the production.

**Example:**
```text
lhs_symbol
    | rhs_symbol_1 `PATTERN_1`
    | rhs_symbol_2 `PATTERN_2`;
```
This set of production definitions produces the productions `lhs_symbol -> rhs_symbol_1` and
`lhs_symbol -> rhs_symbol_2` with patterns `PATTERN_1` and `PATTERN_2`, respectively.

### Default Patterns
A default pattern can be specified for an entire set of production definitions by including the pattern immediately
after the left-hand-side symbol.

**Example:**
```text
lhs_symbol `PATTERN_1`
    | rhs_symbol_1
    | rhs_symbol_2
    | rhs_symbol_3 `PATTERN_2`;
```
This production definition produces the productions `lhs_symbol -> rhs_symbol_1`, `lhs_symbol -> rhs_symbol_2`, and
`lhs_symbol -> rhs_symbol_3`, with patterns `PATTERN_1`, `PATTERN_1`, and `PATTERN_2`, respectively.
