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
- **idle**: `install` took SIGINT and SIGTERM, and no guard is open.
- **open(n)**: n guards are open (`n >= 1`), and no Ctrl-C has arrived.
- **requested(n)**: n guards are open, and one Ctrl-C arrived inside the
  window.
- **held**: a child holds the terminal.
- **ending**: the clean-ups run, then the signal's default action.

| From | Event | To | Effect |
| --- | --- | --- | --- |
| not installed | `install`, with `handles_interrupt` false | idle | one thread takes SIGINT and SIGTERM; `handles_interrupt` is true |
| not installed | `install`, with `handles_interrupt` already true | not installed | returns `InstallError::Taken`; nothing is installed |
| idle | SIGINT or SIGTERM | ending | clean-ups, then the signal's default action |
| idle | `cooperate` | open(1) | request cleared, then count 1 |
| open(n) | `cooperate` | open(n+1) | request untouched |
| requested(n) | `cooperate` | requested(n+1) | request kept |
| open(n) | SIGINT | requested(n) | flag set; the process keeps running |
| requested(n) | `check` on any guard | requested(n) | returns `Err(Interrupted)` |
| requested(n) | SIGINT | ending | a shell reports 130 |
| open(n), requested(n), n > 1 | a guard dropped | open(n-1), requested(n-1) | request kept |
| open(1), requested(1) | the last guard dropped | idle | request cleared |
| open, requested | SIGTERM | ending | as from idle |
| any but ending | `handed_over` | held | SIGINT is dropped |
| held | `taken_back`, Windows only | the earlier state | a Ctrl-C not yet read is discarded |
| held | SIGTERM | ending | as from idle |

The usage log's `start` reads `handles_interrupt` after it installs its own
observer. When the answer is true, `start` skips its signal thread, so the
owner stays the only handler.

Crash points:

- SIGKILL in any state ends the process with no clean-up, and the usage line
  is missing.
- A clean-up that panics while ending does not stop the others, and the
  default action follows.
- A clean-up that blocks delays the end. The usage line writer waits at most
  `LINE_WAIT`.
- A guard that unwinds in a panic still runs its drop, so the count stays
  correct.
- A panic while the window lock is held poisons the lock. Readers take the
  data with `PoisonError::into_inner`.
- A request that arrives after the last `check` is lost, and the run ends
  with its own result.
- `install` that cannot take a signal returns `InstallError::Os`, and the tool
  keeps the default signals. A thread that cannot start leaves SIGINT and
  SIGTERM ignored, because signal-hook cannot restore a default action.
- A run that answers a shell's completion request takes no signal, and
  `install` still returns `Ok`.

Invariants:

1. While the owner is installed, it is the only handler from crumbkit or
   crumbkit-usage that ends the process on SIGINT or SIGTERM. Code enforces
   this in both orders of `install` and `crumbkit_usage::start`: `install`
   after `start` returns `InstallError::Taken`, and `start` after `install`
   starts no signal thread. A tool with the usage log starts it with
   `start_under(interrupt::clean_up)`. crumbkit-process's `forward` stays a
   rule of this page: a tool with `forward` does not install the owner.
2. The first SIGINT while any guard is open never ends the process.
3. A request is cleared only while no guard is open.
4. A SIGINT outside a window, the second SIGINT inside one and every SIGTERM
   end the process only after every clean-up has run.
5. The usage line is written at most once.
6. A SIGINT while a child holds the terminal never reaches the ending path.

Why: ADR-0241.
