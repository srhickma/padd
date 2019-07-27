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
```
cdfa {
    # State definitions
}
```
The contents of a CDFA region is a set of state definitions, which (primarily) determine the set of transitions out of
each state. The first state in the first CDFA region is used as the starting state.

Multiple CDFA regions can be used in a single specification, and the result will be as if all state definitions were in
a single region.
For example, the following two specifications are equivalent:
```
cdfa {
    # State 'a'
}

cdfa {
    # State 'b'
}
```
```
cdfa {
    # State 'a'
    # State 'b'
}
```

---

## States

---

## Matchers

### Simple

### Chain

### Range

### Combined

### Default

### Matcher Precedence

---

## Transitions

### Consume All

### Consume None

---

## Acceptors

### State Acceptors

### Transition Acceptors

### Default Acceptor

### Acceptor Destinations
