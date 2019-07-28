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

---

## Optional Shorthand

---

## Patterns

### Production Patterns

### Default Patterns
