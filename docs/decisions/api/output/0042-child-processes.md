# ADR-0042. Child processes

- Date: 2026-10-12
- Status: amended by ADR-0058, ADR-0095

## Context

Some commands run another program as their whole job, such as an editor
or a pager. The child needs the terminal and must receive Ctrl-C itself,
while the tool must still report its result, join the run's trace and
record its usage line. The child's exit codes have their own meanings,
and a caller asking for `--json` expects a document.

## Decision

- `child::hand_over`, behind the off-by-default `hand-over` feature, ran
  the child with the tool's standard streams and the trace variables, so
  the child joins the run's trace.
- The tool returned the child's exit code, and the leaf declares this
  with `Leaf::passes_on_exit`, which help prints, so a caller knows where
  the code comes from. The checks let such a leaf's unlisted failures end
  with any nonzero code, but never with a code that the guide gives a
  listed failure.
- On Unix the tool ignored SIGINT and SIGQUIT from the first child's
  start until it ends, so Ctrl-C reaches only the child. It uses a
  signal-hook handler, because an ignored disposition would stay ignored
  in the child and resetting it needs unsafe code. It keeps ignoring the
  signals after the child exits, because a late Ctrl-C could reach a
  thread then. A command hands over as its last step, so only the result
  and the exit remain.
- On Windows the tool ignored SIGINT and SIGBREAK only while the child
  runs, through signal-hook's conditional shutdown, and a child ended by
  Ctrl-C counts as ended by SIGINT.
- A child with a JSON form went through `hand_over_json`, which adds the
  child's JSON arguments under `--json` and prints the child's output as
  a crumbkit document, so `--fields`, `--long` and `--full` work.
  `--json` on a child without a JSON form fails before the child runs.
- A missing or refused program, a failed child under `--json` and bad
  JSON each failed with their own `crumbkit::child` code.

## Consequences

Closing the console, logging off and shutting down still end a Windows
tool beside its child. `docs/behaviour/output/child-processes.md`
describes the current behaviour.

Considered: a `--json` refused by clap through a per-command flag, which changes help, a child's JSON passed through without parsing, a fixed exit code for a child that a signal ended, `SIG_IGN` for SIGINT with a reset in the child before it starts its program, which needs unsafe code, restoring the default actions on Unix once the child exits, a handler that forwards SIGINT to the child, the `hand-over` code in the default build, a pager API with its own environment variables, a Windows console control handler through `windows-sys`, Windows exit codes passed through as they are, an opt-out with a reason for a command that passes on its child's code, which no single run check could report as stale.
