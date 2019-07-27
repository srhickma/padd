# Alphabet

An alphabet is an optional specification region, where the alphabet of the language being formatted can be specified.
Specifying an alphabet restricts the input characters which can be scanned to only those present in the alphabet, and a
scan error is emitted if a character outside the alphabet is consumed. Alphabets have the form
`alphabet 'SOME_CHARACTERS'`, where `SOME_CHARACTERS` are the characters included in the alphabet.

**Example:** The following alphabet allows only alphanumeric characters to be scanned:
```
alphabet 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'
```

### Escaping
The following characters can be escaped in an alphabet:

| Alphabet String | Allowed Character |
|-----------------|-------------------|
| \n              | newline           |
| \t              | tab               |
| \r              | carriage return   |
| \'              | single-quote      |
| \\\\            | backslash         |

If any other characters are prefixed by a backslash, the backslash will be ignored. Note that newlines, tabs, and
carriage returns can be included directly in the alphabet string, however it is preferred to use the escaped versions
for better readability.

### Default
If no alphabet region is specified, then the lexer will allow all characters to be scanned. This approach is generally
preferred, as many non-trivial languages support large alphabets, which would be cumbersome to write explicitly.
Alphabets should only be specified if it is a strict requirement of the language that only certain characters should be
formatted.

### Example
The following is an alphabet which could be used for a simple programming language:
```
alphabet '<>=+-*/%(){},;:! \t\nABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789'
```
