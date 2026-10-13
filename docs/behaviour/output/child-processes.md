# Child processes

This page covers a command that runs another program as its whole job, such
as an editor or a pager: how it declares that the exit code is the child's,
what crumbkit-test's checks allow for it, how it shares signals with the
usage log, and how a child with a JSON form prints. The
[`child`](../../../crates/crumbkit/src/child.rs) module's rustdoc describes
`hand_over`, `hand_over_json`, `child::Json`, the failure codes, the signal
handling on Unix and Windows and the exit code of a child that a signal
ended.

## The `hand-over` feature

The `child` module sits behind the `hand-over` cargo feature, which is off by
default and adds signal-hook. A command hands the terminal over as its last
step, in place of `finish`, so only the result, the usage log's line and the
exit remain after the child exits.

Why: ADR-0042, ADR-0003.

## Declaring the child's exit code

The tool returns the child's exit code, so on such a command 1 and 2 carry
the child's meanings. The command's leaf declares this with
`Leaf::passes_on_exit`, which names the program, such as `the editor`. Its
`-h` and `--help` then print
`Exit code: the command passes on the exit code of the editor.` after the
`Output:` line, and `help exit-codes` names the command
([help](../help/README.md), [exit codes](../failures/exit-codes.md)). The
declaration needs no reason, because help tells a caller where the code
comes from. Such a leaf needs no document
([view flags](README.md#view-flags)).

`check_twin`, `check_twins` and `check_recoveries` let a failure of such a
leaf end with any code but 0 when no exit lists the failure. A listed
failure and a refused command line still exit as `Guide::failure_exit`
decides. An unlisted failure never exits with a code that an exit gives
listed failures, because a caller would read it as one of them. A failure
with `crumbkit::child::failed` on a leaf without the declaration must exit
with 1, so a test that provokes it shows the missing declaration.

Why: ADR-0042, ADR-0056.

## Signals and the usage log

The child receives the variables of `trace::child_env`, so it joins the run's
[trace](../usage-log/trace-context.md). The usage log records a child's exit
code, and `"interrupted": true` when a signal ended the child.

When a [usage log](../usage-log/README.md) runs, its signal thread has
already taken SIGINT, so crumbkit installs no SIGINT handler of its own for
the child. With `Signals::Crumbkit`, the log's thread skips SIGINT once a
child has held the terminal. With `Signals::Tool`, the tool's own handler
decides, whether or not a child runs.

On Windows, closing the console, logging off and shutting down still end the
tool beside the child, because the C runtime raises no signal for them. A
Ctrl-C that reaches the handler after the child exits may end the tool, and
nobody has tested that case on Windows.

Why: ADR-0042, ADR-0029.

## A child with a JSON form

Under `--json`, `hand_over_json` prints the child's document as `finish`
prints a result, so `--fields`, `--long` and `--full` work as on any
command. The refusal of `--json` for a child without a JSON form happens when the
command runs, so help still lists the global `--json`. A guide's rename of
each `crumbkit::child` code applies as it does to every other failure
([failures](../failures/README.md)).

Why: ADR-0042.
