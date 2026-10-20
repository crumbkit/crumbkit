# ADR-0114. Format characters are escaped

- Date: 2026-10-20
- Status: accepted
- Supersedes: ADR-0016, which it amends

## Context

Under ADR-0016, the table escaped each control character, a character of
Unicode general category Cc, so each row stays on one line. A row's own
text line, a headed key, a failure's text and a picker's labels use the
same escape. A format character, a character of category Cf, passed
through unchanged. A bidirectional override reorders the text after it at
a terminal, and a zero-width space hides a difference between two values
that look the same. An adopting tool escaped these characters itself.

## Decision

- Wherever crumbkit escapes control characters in text output, it also
  escapes each format character, in the `\u{...}` form that
  `char::escape_default` gives.
- crumbkit holds the 21 ranges of category Cf in Unicode 16.0 in its own
  table, which is updated by hand when a new Unicode version adds a format
  character.
- Prose text, bytes and JSON output are unchanged.

## Consequences

`docs/behaviour/output/README.md` states the rule. A soft hyphen, an emoji
sequence joined by zero-width joiners and a flag built from tag characters
print with escapes in text output.

Considered: a dependency for Unicode categories, which ADR-0003 weighs
against a table of 21 ranges; escaping only the bidirectional and
zero-width characters, which leaves the rest of category Cf unescaped and
needs its own list anyway.
