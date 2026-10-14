# Child processes

This page covers a command that runs another program as its whole job, such
as an editor or a pager: how it declares that the exit code is the child's,
what crumbkit-test's checks allow for it, how it shares signals with the
usage log, and how a child with a JSON form prints. The rustdoc of
[`hand_over`](../../../crates/crumbkit-child/src/hand_over.rs) describes
`hand_over`, `hand_over_json`, `Json`, the signal handling on Unix and
Windows and the exit code of a child that a signal ended, and
[`codes`](../../../crates/crumbkit-child/src/codes.rs) lists the failure
codes.

## The crumbkit-child crate

The published crumbkit-child crate holds the hand-over. It depends on
crumbkit and on signal-hook, so a tool that hands nothing over builds no
signal-hook unless it keeps the usage log. A command hands the terminal over
as its last step, in place of `finish`, so only the result, the usage log's
line and the exit remain after the child exits.

crumbkit-child prints its failures and refusals through crumbkit's public
`Output` methods `print_failure`, `refuse_view`, `refuse_fields_for_text`,
`failure_exit` and `listed_exit`, so they look and exit as `finish` would
print them. It reports the hand-over to the
[run observer](../run-observer/README.md) through the public `events`
functions.

The failure codes keep the `crumbkit` component, as in
`crumbkit::child::not_found`, and crumbkit-child lists them in its own
`codes::ALL`, which `crumbkit::codes::ALL` does not include. crumbkit-test
treats every code in the `crumbkit` component as one of crumbkit's codes,
so these codes need no twin and no declaration, and `check_recoveries`
follows their recoveries ([end-to-end checks](../checks/end-to-end.md)).

Why: ADR-0058, ADR-0042, ADR-0003.

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

When the [run observer](../run-observer/README.md) says that it handles
SIGINT, as the [usage log](../usage-log/README.md)'s signal thread does,
crumbkit installs no SIGINT handler of its own for the child. crumbkit tells
the observer that a child took the terminal before the child starts, and on
Windows that the child gave it back once it exits. With `Signals::Crumbkit`, the log's thread skips SIGINT once a
child has held the terminal. With `Signals::Tool`, the tool's own handler
decides, whether or not a child runs.

On Windows, closing the console, logging off and shutting down still end the
tool beside the child, because the C runtime raises no signal for them. A
Ctrl-C that reaches the handler after the child exits may end the tool, and
nobody has tested that case on Windows.

Why: ADR-0042, ADR-0029, ADR-0057.

## A child with a JSON form

Under `--json`, `hand_over_json` prints the child's document as `finish`
prints a result, so `--fields`, `--long` and `--full` work as on any
command. The refusal of `--json` for a child without a JSON form happens when the
command runs, so help still lists the global `--json`. That refusal, with
`crumbkit::child::no_json`, exits with the code that the guide gives a
refused command line, which is 2 unless the guide lists another, or with
the exit that lists `crumbkit::child::no_json`. A guide's rename of each
`crumbkit::child` code applies as it does to every other failure
([failures](../failures/README.md)).

Why: ADR-0042, ADR-0058.
