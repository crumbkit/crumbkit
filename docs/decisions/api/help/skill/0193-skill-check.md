# ADR-0193. Skill check

- Date: 2026-11-10
- Status: accepted

## Context

A tool's agent skill, its SKILL.md, names commands in code spans and code
blocks. A skill written by hand drifts from the command tree when a
command or flag is renamed, and nothing tells the author.

## Decision

- `cargo crumbkit skill check --bin <NAME>` builds the binary as `weigh`
  does and reads its tree from `commands list --all --json --full`, so it
  checks against what the binary serves. It reads `skill/SKILL.md` in the
  binary's package, or the file that `--skill` names.
- A command line is an inline code span, or a line of a fenced block, whose
  first word is the name of the tree's root, after an optional `$ ` prompt.
  When the root's name differs from the binary's, as cargo-crumbkit's root
  `cargo` does, the second word must also name a subcommand of the root, so
  `cargo test` is not checked. The check splits a command line into words
  as a POSIX shell does and stops at a pipe, a list operator, a comment or a
  redirection.
- Each word must name a subcommand, a flag of the command or a global flag
  of a command above it, or a value for a positional argument the command
  still has room for. A flag that takes a value must get one, and a value
  must be one of the argument's possible values when it lists them.
- A word in capitals such as `NAME`, or in angle brackets such as `<name>`,
  counts as any value. `[OPTIONS]`, an optional placeholder such as `[ID]`
  and `...` count as nothing, and the brackets of `[--limit N]` are dropped.
  `--` ends the check.
- A missing required argument passes, because prose names a command without
  its arguments.
- The result is the `crumbkit/skill-check/v1` listing, one row per command
  line with `file`, `line`, `span` and `reason`, failing rows first. The
  default text prints only failing rows as `<FILE>:<LINE>: <SPAN>: <REASON>`
  lines, and the run exits with 1 when any row fails.
- The Markdown reader and the word splitter are small functions in
  cargo-crumbkit, because a skill file writes commands in two forms of code
  and simple shell quoting, and neither needs a dependency.

## Consequences

`docs/behaviour/help/skill-check/README.md` describes the command. The tree
module sits under `skill`, so a later `skill draft` reads the same tree.

Considered: running each command line with `--help` to see whether clap accepts it, which runs the tool once per line and cannot check a value against its possible values without running the command; matching the binary's file name instead of the tree's root name, which fails for cargo-crumbkit itself, whose root is `cargo`; failing a missing required argument; pulldown-cmark and a shell-words crate as dependencies; checking only fenced blocks.
