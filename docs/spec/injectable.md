# Injectable Tokens

Injectable tokens are tokens which can be parsed anywhere in the input string, and will be injected into the parse tree
after the fact.
If injectable tokens are specified explicitly in the grammar, they can be parsed normally as any other token.
Preference is given to the parse tree with the minimum number of injected tokens, so
[explicit parsing](#explicit-parsing) takes precedence over injection.

Ignorable tokens are specified by including `inject DIRECTION TOKEN PATTERN` anywhere in the top-level of a
specification, where `DIRECTION` is the [affinity](#affinity) of the injection, `TOKEN` is the token kind that should be
injected, and `PATTERN` is the [pattern](#pattern) to use when formatting injections of the token.

If multiple injections are specified for the same token, or a token is both injected and ignored, an error is emitted.

---

## Affinity

The affinity of an injection determines which direction the terminal prefers to be injected.
If `left` affinity is used, injections where the terminal is appended after the previous terminal are preferred, and if
`right` affinity is used, injections where the terminal is prepended before the next terminal are preferred.

**Example:** Suppose the lexer produces tokens `[A, B, C]` and the following specification is used, where `DIRECTION` is
an injection affinity:
```text
inject DIRECTION B

grammar {
    s | A C `{} {}`;
}
```
If `DIRECTION` is `left`, then the formatted output will have the form `ab c`, where `a`, `b`, and `c` are the lexemes
of `A`, `B`, and `C`, respectively.
Similarly if `right` is used, the formatted output will have the form `a bc`.

Currently the only two affinities available are `left` and `right`.

**Note:** Injection affinity is merely a _preference_, it is still possible to right-inject a terminal with left affinity
(e.g. if the terminal is the first token).
If an injectable terminal is injected between two non-injectable terminals in the scan, then it is guaranteed that it
will be injected with its preferred direction (ignoring [capture matching](#capture-matching)).

---

## Pattern

Since injectable terminals can be formatted, it is possible to specify a pattern with which to inject the terminal.
This pattern is specified the same way as any other pattern, and can even utilize the active variable scope at the point
of injection! All injection patterns have only a single captureable child, which will always contain the lexeme of the
terminal being injected.

**Example:** Suppose the lexer produces tokens `[A, B, C]` with lexemes `[a, b, c]` respectively, and the following
specification is used:
```text
inject left B `<{}>`

grammar {
    s | A C `{} {}`;
}
```
Then the the result of formatting will be `a<b> c`.

**Example:** Suppose the lexer produces tokens `[B, A, B, C, B]` with lexemes `[b, a, b, c, b]` respectively, and the
following specification is used:
```text
inject left B `<{}[x]>`

grammar {
    s | A t `{;x=1} {;x=2}`;
    
    t | C `{;x=3}`;
}
```
Then the the result of formatting will be `<b>a<b> c<b2>`.

---

## Capture Matching

Even if the parser is able to inject a token with its desired affinity, it is not guaranteed that the terminal being
attached to (i.e. injected against) will be captured by the associated pattern.
For this reason, it is possible for injections to change their direction during formatting, under the condition that
their preferred neighbour has not been captured.
If both neighbours of an injected token aren't captured, then the injection will be ignored.

**Example:** Suppose the lexer produces tokens `[A, B, C]` with lexemes `[a, b, c]` respectively, and
`inject left B` has been defined.
Now consider the following grammars:
```text
# This will produce "ab c"; Normal.
grammar {
    s | A C `{} {}`;
}
```

```text
# This will produce " bc"; Direction change!
grammar {
    s | A C ` {1}`;
}
```

```text
# This will produce " "; Ignored!
grammar {
    s | A C ` `;
}
```

---

## Explicit Parsing

If the grammar being used includes injected tokens as terminals, preference will be given to parse trees which
incorporate the symbol as leaves versus those which inject it.
To be specific, the number of tokens which are injected by a parse tree is added to the overall weight of the tree,
and the lowest weight parse tree is always chosen for formatting.

**Note:** In general it is more efficient to inject as many injectable tokens as possible, and only explicitly parse
those which absolutely require special formatting.
The performance impact of injection is much smaller than the performance impact of adding many optionals, which adds
significant complexity (both visually and computationally) to the grammar.

---

## Performance Considerations

Using injectable terminal symbols adds ambiguity to a parse (proportional to the number of injectable tokens in the
lex), and thus makes parse recognition slower.
Due to the preference for non-injected terminals, parse trees using injectable tokens must be weighted, which adds
computational complexity to the parse tree construction phase as well (even if no injectable tokens are scanned).

For these reasons, it is best to use injectable tokens when such tokens occur relatively infrequently compared to other
tokens (e.g. code comments).
In these scenarios, using injectable tokens will be orders of magnitude faster than using a grammar which incorporates
the tokens directly (e.g. as optionals), and will result in a much cleaner grammar.
If injectable tokens are expected to make up more than 50% or so of all tokens, then it may be more efficient to
explicitly include the tokens in your grammar as optionals.
Performance in such cases will vary heavily on the grammar, so manual testing should be used to determine which method
is more efficient.
