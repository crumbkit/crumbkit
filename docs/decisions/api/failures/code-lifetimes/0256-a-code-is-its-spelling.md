# ADR-0256. A code is its spelling

- Date: 2026-11-17
- Status: accepted
- Amends: ADR-0001 and ADR-0205

## Context

A `Code` held three `&str` segments, which sit apart in memory. A tool whose
libraries spell a code as one text, such as `tool::area::reason`, could not
return a declared code as one `&'static str` for its library's trait. One tool
kept a second table of its 70 codes in both forms, and another split the text
at run time and called `expect`.

ADR-0001 rejected a derive that checks a `code = "a::b::c"` string, because
the check would run in a proc macro that cannot see a constant.

## Decision

- A `Code` holds one spelling and the byte offsets of its area and reason.
  Its three segments are slices of the spelling.
- `Code::from_spelling(spelling)` is a `const fn` that replaces `Code::new`.
  It accepts text with exactly two `::` separators where each segment follows
  ADR-0156, and in a `const` it refuses anything else as a compile error that
  names the broken part. This keeps the compile-time check that ADR-0001
  wanted, without a derive.
- `Code::as_str` returns the spelling as declared. For a code of crumbkit's
  own it reads `crumbkit::cli::usage`, and the printed spelling writes the
  tool's name there.
- `Code::runtime` takes one spelled text in place of three segments and
  returns `RuntimeCodeError::Parts` for text that does not split into three.
  `Spelling::split` is removed, because `Code::runtime` parses the text.
- `CodeBuf` owns one `Box<str>` and two offsets in place of three boxes.
- `ToolName::code` is replaced by `ToolName::code_component`, which returns
  the component a code prints with, because a code with the tool's component
  swapped in can no longer be lent as one borrowed text.
- `Spelling::spell` writes the tool's component followed by the rest of the
  declared spelling. No printed code changes.

## Consequences

A tool writes `Code::from_spelling("shelf::books::not_found")` where it wrote
`Code::new("shelf", "books", "not_found")`, and returns `X.as_str()` from its
library trait. Code built from text in a `const` panics outside a `const`
when the text is bad, which the rustdoc warns about, and a tool that learns a
spelling at run time calls `Code::runtime`.

Considered: a `Display` for `Code`, which would let a code print without the
tool's name; a `code!` macro; and a crate of codes alone without clap. Each
is left out of this change.
