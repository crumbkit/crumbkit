# ADR-0226. A reviewed failure gives each variant a fixed sentence

- Date: 2026-11-16
- Status: accepted
- Supersedes: ADR-0001, ADR-0007 and ADR-0097, which it amends

## Context

A tool that shows a failure to anyone, including a person who did not run
the command, needs a sentence that a reviewer wrote in advance and that no
run-time value can change. `Failure` had `Display` for the message and the
`recovery`, `alternatives`, `fields` and `url` methods, and no such
sentence. A tool that carries many coded failures also wants the build to
fail when one of them has no reviewed sentence. The derive already refuses a
variant without exactly one of `recovery`, `hint` and `no_next_step`, so the
repair of each variant was already required and only the sentence was
missing.

## Decision

- `Failure::sentence(&self) -> Option<&'static str>` returns a fixed
  sentence, and `None` is the default. `Error::sentence` returns the
  sentence of the failure inside.
- The failure document gains an optional string `sentence`, which is
  present only when the failure has one. The document keeps the id
  `crumbkit/failure/v1`, because a reader that ignores an unknown optional
  property reads the new document unchanged. The text form keeps printing
  `Display`.
- The derive accepts `sentence = "..."` on a variant or a struct. The value
  is a string literal on one line with no `{`, so the derive refuses an
  empty literal, a literal with a line break and a literal with a brace.
  Static text keeps run-time values out of reviewed text, which a
  `format!` placeholder as `hint` has would let in.
- `#[failure(reviewed)]` on the type makes a missing `sentence` on any
  variant that does not forward, or on the struct, a compile error that
  names it. A variant that forwards returns the wrapped failure's sentence.
  A type without `reviewed` is unchanged, so a tool adds the key to one
  enum at a time.
- crumbkit-test gets no `check_sentences`. A hand-written impl and a code
  from `Code::runtime` escape the check, and a tool that wants it moves the
  impl to the derive.

## Consequences

The check covers only types that derive `Failure`. A tool with many coded
sites generates the `sentence` keys from its reviewed table, adds
`reviewed` to each enum as it fills, and lets the compiler list what remains.
A `Failure` written by hand still compiles without a sentence.

Considered: a required `sentence` method on the trait, which checks
hand-written impls too and forces a sentence onto every failure inside
crumbkit and its examples; a `Cow<str>` sentence with placeholders, which
weakens the review; a `check_sentences` function in crumbkit-test, which
needs a second place that lists the types to check.
