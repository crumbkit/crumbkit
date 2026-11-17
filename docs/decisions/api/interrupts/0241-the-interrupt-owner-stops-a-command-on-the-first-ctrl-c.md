# ADR-0241. The interrupt owner stops a command on the first Ctrl-C

- Date: 2026-11-17
- Status: accepted

## Context

An adopter's batch commands each registered their own SIGINT handler. The
first Ctrl-C stopped the command before its next item with a declared
failure, and the second ended the process with 130. No crumbkit crate offers
that. crumbkit-usage's `start` ends the tool on the first SIGINT or SIGTERM,
crumbkit-child installs a SIGINT handler only while a child holds the
terminal, and crumbkit-process's `forward` feature ends the tool on SIGINT,
SIGTERM and SIGHUP. None keeps the tool running past one Ctrl-C.

The request comes from one adopter, and the need is ordinary: any command
over many items wants to finish its current item and report where it
stopped. This is the ground that API-1 asks for.

## Decision

- The crumbkit crate gains an off-by-default `interrupt` feature and an
  `interrupt` module. `install` takes SIGINT and SIGTERM for the life of the
  process on one thread, and returns a token that opens windows.
  signal-hook becomes an optional dependency of crumbkit. The lock file
  already holds it through three other crates, so the edge adds no crate.
- The owner stays installed until the process ends, because signal-hook does
  not restore a signal's default action when its last handler goes, so a
  scoped handler would leave Ctrl-C ignored.
- The first SIGINT inside a window sets a flag, and `check` returns
  `Interrupted`. A second SIGINT inside the window, a SIGINT outside a
  window and every SIGTERM run the clean-ups and take the signal's default
  action. On Windows the process exits with `STATUS_CONTROL_C_EXIT`.
- `Interrupted` has the code `crumbkit::run::interrupted`, the recovery
  `Retry`, and the default exit 130, which a tool record can map to another
  exit like any code.
- The owner is a run observer. `handles_interrupt` is true once it is
  installed, so crumbkit-child installs no SIGINT handler, and
  `handed_over` makes the owner drop SIGINT until `taken_back`.
- A tool with the usage log calls `start_under(interrupt::clean_up)`, as
  ADR-0179 decided for `forward`, so the line is written once.

## Consequences

The owner and `forward` each end the tool on SIGINT, so a tool uses one of
them. The rustdoc of the module says so, and neither can detect the other.
A process that started with SIGINT ignored now ends on it, a shortcut that
the code marks with a RUST-10 comment, as crumbkit-usage's thread does.
A Ctrl-C that arrives after the last `check` is lost, and the run ends with
its own result.

Considered: a window that closes itself when the command returns, which hides
the point where the flag is read; and a shared signals crate for the four
owners, which no tool needs yet.
