# ADR-0052. JSON layout

- Date: 2026-10-12
- Status: amended by ADR-0073

## Context

One-line JSON costs an agent the fewest tokens and lets a program read a
document with one line read, but some tools have readers who need
indented JSON. Changing the layout at a terminal would break OUTPUT-1.

## Decision

- Every whole JSON document printed on one line by default, at a terminal
  and through a pipe alike.
- A tool set `Guide::json_layout(JsonLayout::Pretty, reason)` to indent
  its documents as serde_json's pretty formatter does. crumbkit re-indents
  the one-line bytes with a plain function, because serde_json's pretty
  formatter would generate each document's serialization code a second
  time and made shelf's binary 7.5% larger, also for tools that never set
  the layout.
- A stream kept one document per line in every layout, because a reader
  of JSON Lines reads one line at a time.
- `JsonLayout` was a `#[non_exhaustive]` enum with only `Pretty`, because
  a `Compact` variant would only name the default and a later layout can
  add a variant.
- The output budget counts the indented bytes, so the opt-out report
  lists the layout with that cost: indenting made shelf's example
  documents 20 to 62 percent larger.

## Consequences

`docs/behaviour/output/json-layout.md` describes the current behaviour.

Considered: serde_json's pretty formatter, a formatter that wraps it, an indented default at a terminal and one line through a pipe, a `Compact` variant, a choice of serde_json formatter or indent width, a budget counted in the bytes of the one-line form, a separate pseudo-terminal golden file for indented output, an environment variable or a flag that picks the layout.
