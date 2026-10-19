# ADR-0103. Exit ranges and signal codes

- Date: 2026-10-20
- Status: accepted
- Supersedes: ADR-0002 and ADR-0037, which it amends

## Context

ADR-0002 let a tool give a failure its own exit code from 3 to 125 and
left the codes from 126 to the shell. ADR-0037 made each declared `Exit`
one code, and `check_exit_codes` required an opt-out for every code from
126 to 255.

An adopting tool reported two cases that these rules could not describe.
Some of its commands pass on the exit code of another program, such as a
child process or a test suite, and that program can exit with any code
from 1 to 255. The tool could declare those codes only one at a time, and
each code from 126 needed its own opt-out. A run of the tool that Ctrl-C
ended exits with 130, the code that a shell reports for a process that
SIGINT ended, and declaring that code also needed an opt-out with a
reason, although 130 means what a caller already expects.

A shell reports a process that a signal ended as 128 plus the signal's
number, so 130 means SIGINT and 143 means SIGTERM. A tool that catches a
signal, cleans up and exits with the same number gives a caller the code
that the caller would have seen without the handler. The codes 126, 127
and 128 have other meanings in a shell: a command that cannot run, a
command that was not found, and an invalid argument to `exit`.

## Decision

- `Exit::range(first, last, meaning)` declares the codes from `first` to
  `last`, both included, with one meaning. `Exit::codes` returns the range,
  `Exit::code` returns the first code, and a single code is a range of one
  code. `Exit` stays a `Copy` type that a `const` builds.
- A range may run from 3 to 255 without an opt-out, because its codes come
  from another program and the tool cannot choose them. `check_exit_codes`
  fails on a range that starts below 3, ends before it starts, or holds a
  code that another declaration holds.
- A range names no single code for a failure to exit with, so the failures
  that a range lists never decide a run's exit, and `check_exit_codes`
  fails on a range that lists any.
- A single code may be 3 to 125 or 129 to 159 without an opt-out. The codes
  from 129 to 159 are 128 plus the signal numbers 1 to 31. The codes 126 to
  128 and 160 to 255 still need an opt-out of `exit-codes/shell-code`.
- `help exit-codes`, the `commands list` note and the command reference
  print a range as its first and last codes joined by a dash, such as
  `124-125`. Each exit of a `commands list` row holds `last` beside `exit`,
  and `last` equals `exit` for a single code.
- The usage report gives a recorded exit the meaning of the declared range
  that holds it.

## Consequences

A command that passes on another program's code still declares
`Leaf::passes_on_exit`, which lets its failed runs end with any code. The
range adds the meaning that help prints. A tool that declared 130 or
another signal code with an opt-out finds that opt-out stale, and the check
asks it to remove it. `docs/behaviour/failures/exit-codes.md` describes the
current behaviour.

Considered: a range type separate from `Exit`, which would need a second
list on the guide and a second reader in every printer; an opt-out of
`exit-codes/shell-code` for a range, which would hide nothing that the
range does not already say; accepting every code from 126 for a single
code, which would let a tool give 127 a meaning that a shell also gives it;
a range that lists failures and exits with its first code, which would
print a code that the range's meaning does not describe; a `const fn`
assertion on the bounds, which would fail a build where crumbkit otherwise
leaves the declarations to the check.
