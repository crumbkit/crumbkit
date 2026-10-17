# Parsing

This page covers how a tool parses its command line with crumbkit: the
`parse` call in `main`, the `Stop` it returns when it ends the run itself,
the command tree that it hands on to `finish`, and command lines that
crumbkit refuses. [Aliases](aliases.md) and
[shell completion](completion.md) have pages of their own.

## Parsing in main

A tool's `main` parses with [`parse`](../../../crates/crumbkit/src/parse.rs)
in place of clap's `Cli::parse`. `parse` returns the exit code instead of
calling `process::exit`, so every invocation ends by returning from `main`.
Tests run the same parse in memory through `parse_to`, which writes to a
sink of in-memory writers. The rustdoc of `parse`, `parse_from` and
`parse_to` states what each prints and the help lines that crumbkit adds.

On success, `parse` returns the tool's parsed type and a
[`Parsed`](../../../crates/crumbkit/src/parse/parsed.rs). `Parsed` keeps the
command tree, the words the caller typed with any values that a
[picker](../effects/pickers.md) added, and whether the caller passed
`--no-input`. `Output::confirm` repeats those words with `--yes` in its
recovery.

A tool with personal aliases passes its arguments through
`crumbkit_aliases::expand` before `parse_from`, as the
[aliases page](aliases.md) describes.

Why: ADR-0015, ADR-0002.

## The Stop of a run that parse ended

When `parse` prints help, the version or a refusal, it ends the run and
returns a [`Stop`](../../../crates/crumbkit/src/parse/stop.rs) in place of a
bare `ExitCode`. A tool's `main` returns `stop.code()`. A tool that prints
more before it exits reads `Stop::output`, whose `Output::is_json` says
whether the caller asked for JSON. The `Output` inside a `Stop` sits in a
`Box`, which costs one allocation only on a run that `parse` ends.
`Stop::number` gives the same exit code as a `u8`, for a tool that records
or passes on the code before it exits.

Why: ADR-0002.

## The command tree after parsing

Help pages, the `Next:` lines of listings and the calls in recoveries are
rendered from the `clap::Command` that the tool builds at run time, which
may hide or remove commands. A tool's `main` passes the `Parsed` that
`parse` returned to `Output::set_command` once.

- `set_command` keeps the tree as parsing left it, with the root and each
  command on the typed path built.
- `finish` builds only the branch below the root that a call names, or the
  whole tree for a call to the root.
- The commands that read the whole tree, such as `commands list` and the
  [schema commands](../schemas/commands.md), build it through
  `Output::command`.

`finish` and those commands therefore take `&mut Output`, a handler that
reads the tree runs before `finish`, and the work of a run grows with the
command typed. A `Call` that does not resolve against the tree is left out,
or prints as `Recovery::None`. crumbkit prints a hidden command or flag only
where the caller typed it. The rustdoc of
[`Output::set_command`](../../../crates/crumbkit/src/output.rs) and
`Output::command` states what each builds.

Why: ADR-0015.

## Dashed forms of command names

When the tool's guide declares `Guide::dashed_forms`, `parse` replaces each
typed dashed form, such as `reading-lists`, with the command's own name,
such as `reading_lists`, before anything else reads the words. clap, help,
refusals, the usage log and the parsed words then name the command as the
tool does. Completion and `command_path` find the command through the same
lookup. The rewrite walks the typed words once. The
[declarations page](../adoption/declarations.md) covers the declaration.

Why: ADR-0015, ADR-0056.

## Refused command lines

A command line that clap refuses prints as a failure with the code
`crumbkit::cli::usage` and exits with 2, unless the tool's guide lists
another [exit code](../failures/exit-codes.md) for it. With `--json`, the
refusal prints as a `crumbkit/failure/v1` document on standard output. The
rustdoc of `parse_to` states the message and the recovery.

- The refusal's message is clap's error on one line, except a view-flag
  refusal, which crumbkit words itself
  ([view flags](../output/README.md#view-flags)).
- The refusal's fields name what clap reports: `argument`, the refused
  argument; `value`, its refused value; and `subcommand`, an unknown
  subcommand, in that order. A refused `--limit` above its bound adds
  `most` ([listings](../output/listings/README.md#an-upper-bound-on---limit)).
- The recovery runs the `-h` of the deepest typed command, even a hidden one.

The walk that finds the deepest command also finds `--json`, which makes a
refusal print as a JSON document. It counts the word only where it reads it
as a flag, so a value of an option that allows hyphen values is no request
for JSON. A tool cannot name its own output flags for the walk to treat as
`--json`, because on the parsed path crumbkit cannot set the `Output` that
the tool's own type builds, and `Output::is_json` would answer differently
on the two paths ([help](../help/README.md#the-walk-before-parsing)).

The refusal names real names only. When clap suggests a hidden alias of a
flag or subcommand, the refusal names the real flag or subcommand that the
alias belongs to, and it leaves the suggestion out when that real item is
hidden. A tool can add its own alternatives to a refusal through a
[declaration](../adoption/declarations.md).

A command that refuses the caller after parsing, such as a handler that
rejects a value, returns an ordinary `Failure`, which `finish` prints.
crumbkit-test's checks and the golden files cover the whole command tree.

Why: ADR-0015, ADR-0001, ADR-0021, ADR-0050.
