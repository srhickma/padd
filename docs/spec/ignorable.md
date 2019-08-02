# Ignorable Tokens

Ignorable tokens are tokens which can be ignored by the parser (and grammar), and will not be added to the parse tree
when ignored.
If the grammar being used includes these tokens as terminals, preference will be given to parse trees which incorporate
the symbol as leaves versus those which ignore it.
Therefore ignorable symbols are useful if a certain type of token should be formatted in certain cases, but removed in
all other cases.

Ignorable tokens are specified by including `ignore TOKEN` anywhere in the top-level of a specification, where `TOKEN`
is the token kind that should be ignored.

**Example:** The following specification will format nested `a`,`b` pairs with a `c` in the middle, with all other `c`
characters removed.
```text
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
```

---

## Explicit Parsing

As mentioned above, parse trees which include ignorable terminal symbols as leaves are favoured over those which do not.
To be specific, the number of tokens which are ignored by a parse tree is added to the overall weight of the tree,
and the lowest weight parse tree is always chosen for formatting.

**Note:** Ignorable tokens should not be defined unless they are explicitly used in the grammar at least once.
If a terminal is never used in the grammar, it is more efficient to simply avoid tokenizing the terminal at all using
the [default acceptor](cdfa.md#default-acceptor).

---

## Performance Considerations

Using ignorable terminal symbols adds ambiguity to a parse (proportional to the number of ignorable tokens in the lex),
and thus makes parse recognition slower.
Due to the preference for non-ignored terminals, parse trees using ignorable tokens must be weighted, which adds
computational complexity to the parse tree construction phase as well (even if no ignorable tokens are scanned).

For these reasons, it is best to use ignorable tokens when such tokens occur relatively infrequently compared to other
tokens (e.g. code comments).
In these scenarios, using ignorable tokens will be orders of magnitude faster than using a grammar which incorporates
the tokens directly (e.g. as optionals), and will result in a much cleaner grammar.
If ignorable tokens are expected to make up more than 50% or so of all tokens, then it may be more efficient to
explicitly include the tokens in your grammar as optionals.
Performance in such cases will vary heavily on the grammar, so manual testing should be used to determine which method
is more efficient.
