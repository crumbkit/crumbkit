# ADR-0213. A row declares its text line as one value

- Date: 2026-11-12
- Status: accepted
- Supersedes: ADR-0064 and ADR-0072, which it amends

## Context

Under ADR-0064 a row type declared its own text line in two places.
`Row::LINE` held the form that help shows, and `Row::text_line` built each
row's line. crumbkit read `text_line` only when `LINE` was set. A type that
overrode `text_line` and forgot `LINE` printed a table, and nothing reported
that its method was never called. A type that set `LINE` and forgot
`text_line` printed every row as compact JSON.

## Decision

- `Row::LINE` holds an `Option<Line<Self>>`, and `Row::text_line` is
  removed. `Line::new("<POINTER> -> <TARGET>", Self::line)` takes the form
  and a function `fn(&R) -> Option<String>`.
- The function may return `None`, and that row prints as one line of compact
  JSON with every field, as a `text_line` that returned `None` did. A row
  type that wants only the form writes `Line::new("<FORM>", |_| None)`.
- Help, `commands list`, crumbkit-markdown and crumbkit-test read the form
  through `LeafDocument::line()`, which still returns `Option<&'static
  str>`, so their output does not change.
- `Line` holds a function pointer, so it has `Clone`, `Copy` and `Debug`
  written without a bound on the row type, and no `PartialEq`.

## Consequences

A row type cannot declare a text line without its help form, or a form
without the function that builds the line. A row type that overrode
`text_line` moves its body into a method or a closure that it passes to
`Line::new`. A caller outside crumbkit that asked a row for its line writes
`R::LINE.and_then(|line| line.text(row))`.
`docs/behaviour/output/shapes/README.md` describes the current behaviour.

Considered: a function that returns `String`, which would remove the JSON
fallback that ADR-0064 documents; and a second constructor that takes only
the form, which would bring back a line whose two halves are declared
apart.
