# ADR-0037. Exit codes

- Date: 2026-10-12
- Status: amended by ADR-0103

## Context

Many tools return exit codes beyond 0, 1 and 2, and scripts already
depend on them. An agent needs to learn what each code means for each
command, and the code a run returns must agree with the failure that the
run recorded.

## Decision

- A tool declared its extra codes in the `Guide` as `Exit`s with a
  meaning and optional command paths, because most codes apply to many
  commands.
- An `Exit` listed failure codes, and a run exits with that code when its
  first failure has a listed code. The first failure decides, because the
  usage log and the span record its code, so a recorded code explains its
  exit. A listed code may be one of crumbkit's.
- `help exit-codes` and `commands list` printed the codes, so an agent
  learns them with the help or the branch.
- `Guide::root` let a tool that another program runs with fixed words,
  such as a cargo subcommand, print commands that run as written.
- `check_exit_codes` reported declarations that give a code two meanings
  or that a caller cannot read, and the run checks fail a run that exits
  otherwise than the guide decides. Codes from 126 and 1 or 2 need an
  opt-out, and 0 has none, because a caller reads 0 as success. crumbkit
  never checks declarations at run time, so a wrong declaration cannot
  end a run.

## Consequences

A run-time code cannot be listed, so its failure exits with 1.
`docs/behaviour/failures/exit-codes.md` describes the current behaviour.

Considered: a `Failure::exit` method, an exit code passed to `end` and `finish`, the last failure choosing the exit, listed failure codes in `help exit-codes`, renaming cargo-crumbkit's clap root `cargo crumbkit`, which clap would print as one word, a full invocation string in place of a path, exit codes declared on each leaf, a declared code that must name its commands, a runtime check of the declarations, an opt-out for the code 0.
