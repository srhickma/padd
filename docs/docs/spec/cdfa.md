# Compressed Deterministic Finite Automata

Compressed deterministic finite automata (CDFAs) are a modified version of a traditional deterministic finite automata
(DFA), with modifications to make language specification less verbose and more powerful (hence "compressed"), without
losing any of the performance benefits of traditional DFAs over regular expressions and non-deterministic finite
automata (NFAs).
In fact, the internal CDFA implementation (called an Encoded Compressed Deterministic Finite Automata, or ECDFA), is 
actually _more efficient_ than a simple DFA for representing complex structures like character ranges and chains
(e.g. keywords).
A more in-depth overview of the ECDFA implementation and design decisions is given in the
[architecture](../architecture.md) section.

As a brief introduction, DFAs consist of states and transitions, one state is designated the "start" state, and states
can be designated "accepting".
Transitions consist of a start state, a destination state, and a character over which the transition occurs.
In other words, a DFA is a directed graph where states are vertices and transitions are edges.
When starting to scan some input text, we begin in the start state and follow transitions as we read input characters.
If there is no transition leaving the current state for the next input character, or the entire input has been
exhausted, the scan in complete.
If the final state is "accepting" the scanned characters are consumed and become a single "token", otherwise the scan
fails.
This process is repeated until either the scan fails or the entire input has been consumed, yielding a sequence of
tokens.
These tokens are what is passed to the parser.

The padd lexer is a "longest-match" lexer, which is a slight variation of the example above, in which tokens are
greedily scanned to be as long as possible.

---

## CDFA Region

The CDFA region is a required region in specifications, and has the form:
```text
cdfa {
    # State definitions
}
```
The contents of a CDFA region is a set of state definitions, which (primarily) determine the set of transitions out of
each state. The first state in the first CDFA region is used as the starting state.

Multiple CDFA regions can be used in a single specification, and the result will be as if all state definitions were in
a single region.
For example, the following two specifications are equivalent:
```text
cdfa {
    # State 'a'
}
...
cdfa {
    # State 'b'
}
```
```text
cdfa {
    # State 'a'
    # State 'b'
}
```

---

## States

CDFA states are denoted by string names which, by convention, are written in snake-case.
At a bare minimum, each state definition consists of a the state name followed by a list of transitions.

**Example:** The following is a simple state definition for `my_state`, which has three transitions, one to
`dest_state1`, one to `dest_state2`, and one to `dest_state3`:
```text
my_state
    MATCHERS -> dest_state1
    MATCHERS -> dest_state2
    MATCHERS -> dest_state3;
```
In this example `MATCHERS` represents the location where input matchers would be specified, determining under which
conditions each transition should be taken.

Multiple state definitions can be used for a single state, in which case the transitions across all definitions will be
combined.

**Example:** The following specification is equivalent to the previous example:
```text
my_state
    MATCHERS -> dest_state1
    MATCHERS -> dest_state2;

my_state
    MATCHERS -> dest_state3;
```

Multiple states can share transitions by separating their names using the pipe character `|`.

**Example:**
```text
my_state | other_state
    MATCHERS -> dest_state1
    MATCHERS -> dest_state2
    MATCHERS -> dest_state3;
```

**Note:** when transitions are applied to multiple states, so are any state acceptors and state acceptor destinations.
See the section on [acceptors](#acceptors) for more information.

---

## Matchers

Matchers are what determine which transition should be taken from a state given some input text.
In the case of a DFA, all input matchers are single characters (i.e. a "simple" matcher), however CDFAs support more
powerful pattern matching.

### Simple
Simple matchers allow a transition to be taken if the next input character matches a specified character.
Simple matchers have the form `'c'`, where `c` is the character under which the transition should be taken.

**Example:**
```text
my_state
    'a' -> got_an_a
    'b' -> got_a_b
    'c' -> got_a_c;
```

### Chain
Chain matchers are very similar to simple matchers, however they allow a transition to be taken when a _sequence_ of
characters is scanned, not just a single character.
Chain matchers have the form `'some_characters'`, where the associated transition will be taken if the remaining input
text has prefix `some_characters`.

**Example:**
```text
my_state
    'int' -> int_state
    'bool' -> bool_state
    'string' -> string_state;
```
From this example it becomes obvious that chain matchers lend themselves very well to scanning language keywords.

### Range
Range matchers are a simplification for a range of simple matchers, which allows any character in range of consecutive
characters to be matched.
The ordering of characters is determined by their unicode code-points.
Range matchers have the form `'a' .. 'z'`, where `a` is the the lower bound (inclusive) of the range, and `z` is the
upper bound (also inclusive).

**Example:**
```text
my_state
    'a' .. 'z' -> lowercase
    'A' .. 'Z' -> uppercase
    '0' .. '9' -> numeric;
```

### Combined
Matchers can be combined for a single state with a logical OR using the pipe symbol `|`.
In this case, if _any_ of the matchers for the transition are matched by the remaining input, then the transition is
taken.

**Example:**
```text
my_state
    ' ' | '\t' | '\n' | '\r' -> whitespace
    'a' .. 'z' -> text;
```

### Default
A default matcher can be used to match any single character, in the event that no other matcher in the state definition
is satisfied. The default matcher is designated by `_`.

**Example:**
```text
my_state
    '0' .. '9' -> numeric
    _ -> non_numeric;
```

### Precedence
Matcher precedence is determined as follows:

* If there is either a simple or chain matcher which matches the remaining input, its associated transition will be
taken. Otherwise,
* If there is a range matcher which matches the remaining input, its associated transition will be taken. Otherwise,
* If the state has a default matcher, its associated transition will be taken. Otherwise,
* No transition is taken.

### Collision
Simple matchers and chain matchers are stored together in a trie, and an error is emitted if the trie is not
prefix free.
Due to this, for any given input, at most one simple or chain matcher can match the input.

Range matchers are allowed to overlap simple and chain matchers (since they have lower precedence), however all ranges
in a single state definition must be pairwise-disjoint, otherwise an error is emitted.

The default matcher can be used at most once per state definition, and (obviously) overlaps with all other matchers.
This is allowed, since the default matcher has the lowest precedence of all.

Due to the above restrictions, it is guaranteed that all matching is deterministic.
The same input in the same state will result in the same transition taken.

---

## Transitions

There are two types of transitions which can be used in a state definition, consume-all and consume-none, which differ
in how they advance the scan cursor after taking the transition.

### Consume All
Consume-all transitions, denoted by `->`, are the standard form of transition.
When a consume-all transition is taken, the scan cursor is advanced past the input prefix which was matched, and the
next iteration matches the remaining input.

**Example:**
```text
my_state
    'a' -> other_state;
```
In this example, if the input in `my_state` is `abc`, the transition to `other_state` will be taken and the remaining
input will now be `bc`.

### Consume None
Consume-none transitions, denoted by `->>`, are used to perform a transition without consuming any of the input
(i.e. the scan cursor is not advanced).

**Example:**
```text
my_state
    'a' ->> other_state;
```
In this example, if the input in `my_state` is `abc`, we will transition to `other_state` and the remaining input will
still be `abc`.

This type of consumer is less common, however a typical use of consume-none transitions is to effectively "import" the
transitions of one state into another, avoiding duplication.

**Example:** Consider the following two state definition:
```text
my_state
    'a' -> got_a_overridden
    'b' -> got_b
    'c' -> got_c
    'd' -> got_d;

other_state
    'a' -> got_a
    'b' -> got_b
    'c' -> got_c
    'd' -> got_d;
```
Using a consume-none transition, this can be simplified to:
```text
my_state
    'a' -> got_a_overridden
    _ ->> other_state;

other_state
    'a' -> got_a
    'b' -> got_b
    'c' -> got_c
    'd' -> got_d;
```

**Note:** It is possible to create an infinite loop using consume-none transitions, so care must be taken when writing
a CDFA to avoid these scenarios.
Currently, these loops are not detected during specification parsing or at runtime, so loops will cause the formatter to
hang.
While it is non-trivial to detect loops at parse time, it is on the road-map to enable loop detection during lexing.

---

## Acceptors

Acceptors are used to specify which states are accepting, as well as how accepting states should be tokenized.

### State Acceptors
The simplest form of acceptor is a state acceptor, which can be used to mark a state as accepting.
State acceptors have the form `^TOKEN` where `TOKEN` is the token which will be produced when the state is tokenized.
The convention for naming token kinds is to use uppercase snake-case.

**Example:**
```text
my_state ^MY_STATE
    'a' -> other_state;
```
In this example, `my_state` is marked as accepting, and when it is tokenized, a `MY_STATE` token will be produced.

### Transition Acceptors
Transition acceptors are syntactic sugar around state acceptors which is useful for allowing the acceptance of states
with no outbound transitions to be inlined into a transition.
The same `^TOKEN` syntax is used, however it is written in the destination of the transition itself, in place of a
destination state.

**Example:**
```text
my_state
    'a' -> ^TOKEN;
```
In this example (assuming no explicit definition of `TOKEN`), when the transition is taken a token of kind `TOKEN` will
be produced, and the lexer will return to the start state.

Under the hood, the specification generator will translate the above specification into:
```text
my_state
    'a' -> TOKEN;

TOKEN ^TOKEN;
```
Due to this, one can use `TOKEN` as they would any other state, even though it has not been explicitly declared.

### Default Acceptor
If a state should be accepted but not tokenized, then a default acceptor should be used.
Default acceptors have the form `^_`.

**Example:** If your formatter ignores whitespace, the following can be used to avoid sending whitespace tokens to the
parser (and over-complicating your grammar):
```text
start
    ' ' | '\t' | '\r' | '\n' -> ^_;
    ...
```

### Acceptor Destinations
To allow lexers to scan regions of input which are syntactically different, without an over-simplified scanner and a
overworked parser (which is much slower), it is useful to allow different regions of text to be scanned using different
starting states.
Acceptor destinations enable this behaviour by allowing a "destination" state to be specified after an acceptor, which
determines the next start state after the acceptor is used.
If no acceptor destination is specified, the current start state is re-used, so "sub-states" like identifier lexers can
be re-used in multiple regions without changing the start state.
The syntax for acceptor destinations is `^TOKEN -> DEST`, where `^TOKEN` is the acceptor, and `DEST` is the destination
state.

**Example:**
```text
my_state
    'a' -> ^A -> start_after_a
    'b' -> ^B -> start_after_b;
```

**Note:** acceptor destinations can be used for both state and transition acceptors, as well as default acceptors.

Acceptor destinations are used heavily in the internal specification lexer, allowing the lexer to scan each region type
using tailored logic.

**Example:** The following is a simple example of how acceptor destinations can be used to scan logical "regions" of
the input:
```text
cdfa {
    start
        'region1' -> ^R1 -> r1_dec
        'region2' -> ^R2 -> r2_dec;

    r1_dec
       '{' -> ^LBRACE -> r1_body;

    r2_dec
       '{' -> ^LBRACE -> r2_body;

    r1_body
        'a' -> ^A
        'b' -> ^B
        '}' -> ^RBRACE -> start;

    r2_body
        '0' .. '9' -> num
        '}' -> ^RBRACE -> start;

    num ^NUM
        '0' .. '9' -> num;
}
```
This example uses two regions, `region1` and `region2` which can internally contain the characters `{'a', 'b'}` and
numbers, respectively.
So the following input would be scanned successfully (ignoring whitespace):
```text
region1 {
    abba
}
region2 {
    1234567890
}
```
However the following input would result in a lexing error (ignoring whitespace):
```text
region1 {
    1234567890
}
region2 {
    abba
}
```

From the above example, it becomes obvious how powerful acceptor destinations can be.
Many context-aware checks can be performed in the (faster) scanning phase, allowing for a simpler and faster parser grammar.
