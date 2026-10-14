# ADR-0044. Command reference

- Date: 2026-10-12
- Status: accepted

## Context

People read a tool's commands on the web or in a repository, where a
Markdown reference is the usual form. A hand-written reference goes
stale when the help changes, and crumbkit prints no Markdown at run time
(ADR-0016).

## Decision

- The published crumbkit-markdown crate rendered one CommonMark file from
  the `commands list` JSON document, so the reference needs no access to
  the tool's code and says what the help says. It uses crumbkit's own
  wording for the effects, writes and stream lines.
- Each command got a section in depth-first order with its description,
  subcommand links, arguments, examples, columns, guards and exit codes.
- Help text was escaped, because help is plain text and a `*` or `<`
  must print as written.
- Links used GitHub's heading anchors, and `render` fails on two commands
  with one anchor, because a link to the second would reach the first.
  It also fails on a document that holds one page of a longer listing, so
  the reference never leaves a command out.
- `cargo crumbkit docs` wrote the file, and crumbkit-test's
  `check_command_reference` compares the checked-in file with a fresh
  render, so a change to the help that leaves the reference stale fails
  CI.

## Consequences

`docs/behaviour/help/reference.md` describes the current behaviour.

Considered: one file per command, a reference read from the clap tree in process, a template engine, a Markdown library that escapes text, a list of anchors without links, a reference that keeps Markdown written in help text, `--check` on `cargo crumbkit docs`, the document id of each command's output, help topics in the reference.
