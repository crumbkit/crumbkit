# ADR-0049. Terminal detection

- Date: 2026-10-12
- Status: accepted

## Context

Many CLIs change their output when a stream is a terminal, so a person,
a script and an agent see different bytes, and a golden file of a piped
run does not pin what a person sees. Some things, such as help colour,
progress and questions, only make sense for a person at a terminal.

## Decision

- A tool's result, failure and notes printed the same bytes whether a
  stream is a terminal or a pipe and whether an agent runs the tool
  (OUTPUT-1), so every reader sees the same output and a piped golden
  file also pins the terminal.
- Detection changed only help colour, progress and questions.
- A tool could turn detection off with
  `Guide::without_terminal_detection` and a reason, which the opt-out
  report lists with its costs. The setting has no environment variable,
  because callers already turn colour, questions and progress off with
  `NO_COLOR`, `--no-input` and `--json`.
- shelf's terminal test ran every example on pseudo-terminals and
  compared the bytes with the piped golden files. It uses rustix's safe
  `pty` functions, which need no `unsafe` and add no crate to the lock
  file, and runs only on macOS and Linux, because Windows has no such API
  without `unsafe` or a new crate.

## Consequences

`docs/behaviour/output/terminal.md` describes the current behaviour.

Considered: a setting without a reason that the opt-out report does not list, an environment variable in place of or beside the guide setting, a separate switch for each of colour, progress and questions, `confirm` asking with detection off, `script(1)` for the pseudo-terminal test, a pseudo-terminal crate such as portable-pty, a public pseudo-terminal check in crumbkit-test.
