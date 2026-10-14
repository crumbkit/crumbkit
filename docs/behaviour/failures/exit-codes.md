# Exit codes

This page covers the exit codes that a tool returns: crumbkit's 0, 1 and 2,
the codes that a tool declares, which failure decides a run's exit, how help
lists the codes, and what crumbkit-test checks.
[`Exit`](../../../crates/crumbkit/src/help/exit.rs) and
[`check_exit_codes`](../../../crates/crumbkit-test/src/exits.rs) describe the
items themselves.

## crumbkit's codes

A run that succeeds exits with 0. A failure exits with 1, and a refused
command line exits with 2. A run whose rows report a problem and that has no
failure exits with 1
([rows that report a problem](../output/listings/README.md#rows-that-report-a-problem)).
A command that hands the terminal to a child returns the child's exit code
([child processes](../output/child-processes.md)).

Why: ADR-0037, ADR-0050, ADR-0042.

## Declaring codes

A tool declares the exit codes it returns besides crumbkit's 0, 1 and 2 in
its `Guide`, each an `Exit` with a one-sentence meaning and, optionally, the
command paths that return it. A code without paths covers every command, and
a path covers its command and every command below it. The declaration lives
on the `Guide`, because most codes apply to many commands.

An `Exit` lists failure codes with `Exit::failures`. A run whose first
failure has a listed code exits with that code in place of 1, in `finish`, a
batch, a prose result and a stream alike. A listed code may be one of
crumbkit's, such as the code of a refused command line, or the tool's rename
of it. A rename that a leaf declares with `Leaf::codes` counts only in runs
of that leaf's command, so an exit that lists a leaf's code covers the
failure only there. The first failure decides, because the usage log and the
`crumbkit.finish` span record its code, so a recorded `code` explains its
`exit`. The guide holds the mapping beside the meaning, so crumbkit-test can
predict each run's exit.

`Output::set_command` copies the exits from the `Parsed` of `parse` or
`Parsed::guided`, so a bare `Command` maps no failure. A code from
`Code::runtime` cannot be listed, so its failure exits with 1.

Why: ADR-0037, ADR-0056, ADR-0060.

## Listing the codes

`<tool> help exit-codes` lists crumbkit's codes, then the tool's, and says
that a shell reports 126 and above for its own reasons. It leaves out
crumbkit's 2 when a refusal exits with another code. It names each command
that passes on a child's exit code, and says that 1 also means a row reported
a problem. `commands list` gives each row the codes that cover its command,
and its text table ends with a note that names the codes and the topic
([commands list](../help/commands-list.md)).

A tool that another program runs with fixed words sets `Guide::root` to the
path of the command that callers run first. cargo runs `cargo crumbkit help`
as `cargo-crumbkit crumbkit help`, so cargo-crumbkit's clap root is `cargo`
and its guide's root is `crumbkit`. That command answers `help` with the
topics, and its help has the `Topics:` line. The topic lines and the
`commands list` note start with the clap root's name and the root's path,
such as `cargo crumbkit help exit-codes`, so every printed command runs.
`check_guide` reports a root that names no command, and `parse` then uses the
clap root.

Why: ADR-0037, ADR-0023, ADR-0042, ADR-0050.

## Checks

crumbkit-test's `check_exit_codes` reports each declaration that would give a
code two meanings or leave a caller unable to read it, as its rustdoc lists.

- `check_twin` and `check_recoveries` fail a run that exits otherwise than
  `Guide::failure_exit` decides for the run's command path, unless its leaf passes on a program's exit
  code.
- A code from 126 to 255 needs an opt-out of `exit-codes/shell-code`, and 1
  or 2 an opt-out of `exit-codes/shared-exit`, that covers every command
  returning it ([opt-outs](../adoption/opt-outs.md)).
- The code 0 has no opt-out, because a caller reads 0 as success.

crumbkit never checks the declarations at run time, so a wrong declaration
cannot end a run.

`self usage` and `self figures list` count each command's runs that exited
with each code from 3, with the meaning that the guide declares for that
command, or null ([usage report](../usage-log/report.md)).

Why: ADR-0037, ADR-0036, ADR-0034.
