# Interrupts

This page covers how a command that works through many items stops on the
first Ctrl-C, under crumbkit's off-by-default `interrupt` feature. The
[`interrupt`](../../../crates/crumbkit/src/interrupt.rs) module describes the
items themselves.

## What a person sees

The first Ctrl-C does not end the tool. The command checks the flag before
its next item, stops, and prints the failure `crumbkit::run::interrupted`,
in text or JSON, with the recovery to run the command again. The run exits
with 130, unless the tool record lists another exit for the code
([exit codes](../failures/exit-codes.md)). A second Ctrl-C while the command
still runs ends the tool at once, and a shell reports 130.

A Ctrl-C when no command has a window open, and every SIGTERM, end the tool
as they do without crumbkit. A Ctrl-C that arrives after the command's last
check is lost, and the run ends with its own result.

Why: ADR-0240, ADR-0241.

## Who owns the signals

`install` starts one thread that owns SIGINT and SIGTERM for the life of the
process. Running a child process with crumbkit-child installs no handler of
its own, and the owner drops SIGINT while a child holds the terminal
([child processes](../output/child-processes.md)). The owner and crumbkit-process's
`forward` feature each end the tool on SIGINT, so a tool uses one of them
([processes](../processes.md#forwarding-the-tools-signals)). A tool with the
usage log starts it with `start_under(crumbkit::interrupt::clean_up)`, so the
line is written once, before the tool ends.

Why: ADR-0241, ADR-0179, ADR-0042.

## Lifecycle

The owner moves between these states:

- **not installed**: crumbkit takes no signal.
- **idle**: `install` took SIGINT and SIGTERM, and no window is open.
- **open**: a command holds an `Interrupt`, and no Ctrl-C has arrived.
- **requested**: one Ctrl-C arrived inside the window.
- **held**: a child holds the terminal.
- **ending**: the clean-ups run, then the signal's default action.

| From | Event | To | Effect |
| --- | --- | --- | --- |
| not installed | `install` | idle | one thread takes SIGINT and SIGTERM; `handles_interrupt` is true |
| idle | SIGINT or SIGTERM | ending | clean-ups, then the signal's default action |
| idle | `cooperate` | open | flag clear |
| open | SIGINT | requested | flag set; the process keeps running |
| open, requested | SIGTERM | ending | as from idle |
| requested | `check` | requested | returns `Err(Interrupted)` |
| requested | SIGINT | ending | a shell reports 130 |
| open, requested | guard dropped | idle | flag cleared |
| any but ending | `handed_over` | held | SIGINT is dropped |
| held | `taken_back`, Windows only | the earlier state | |
| held | SIGTERM | ending | as from idle |

Crash points:

- SIGKILL in any state ends the process with no clean-up, and the usage line
  is missing.
- A clean-up that panics while ending does not stop the others, and the
  default action follows.
- A clean-up that blocks delays the end. The usage line writer waits at most
  `LINE_WAIT`.
- `install` that cannot take a signal returns `InstallError`, and the tool
  keeps the default signals.

Invariants:

1. While the owner is installed, it is the only handler that ends the
   process on SIGINT or SIGTERM.
2. The first SIGINT inside an open window never ends the process.
3. A SIGINT outside a window, the second SIGINT inside one and every SIGTERM
   end the process only after every clean-up has run.
4. The usage line is written at most once.
5. A SIGINT while a child holds the terminal never reaches the ending path.

Why: ADR-0241.
