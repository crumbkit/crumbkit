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

On success, `parse` returns the tool's parsed type and an
[`Output`](../../../crates/crumbkit/src/output.rs). The `Output` holds the
view flags, the command tree, the guide's settings, the words the caller
typed with any values that a [picker](../effects/pickers.md) added, whether
the caller passed `--no-input`, and the code renames of the leaf that the
words name. `Output::confirm` repeats those words with `--yes` in its
recovery.

A tool flattens [`ViewFlags`](../../../crates/crumbkit/src/output/build.rs)
into its clap types to accept `--json`, `--fields`, `--long` and `--full`.
`parse` reads them before the tool's type takes its values, so a handler
reads them through the `Output`. A tool that does not flatten `ViewFlags`
parses with every view flag off.

A tool that parses with clap alone builds its `Output` with
`Output::new(view, command, &GUIDE, args)`, where `args` is the argv with
the program name first. `Output::new` finds the leaf that the words name,
so the run gets that leaf's code renames, and it keeps the words for the
`--yes` recovery. It reads no `--no-input`.

A tool with personal aliases passes its arguments through
`crumbkit_aliases::expand` before `parse_from`, as the
[aliases page](aliases.md) describes.

Why: ADR-0015, ADR-0076, ADR-0002.

## The Stop of a run that parse ended

When `parse` prints help, the version or a refusal, it ends the run and
returns a [`Stop`](../../../crates/crumbkit/src/parse/stop.rs) in place of a
bare `ExitCode`. A tool's `main` returns `stop.code()`. A tool that prints
more before it exits reads `Stop::output`, whose `Output::is_json` says
whether the caller asked for JSON. The `Output` inside a `Stop` sits in a
`Box`, which costs one allocation only on a run that `parse` ends.

Why: ADR-0002.

## The command tree after parsing

Help pages, the `Next:` lines of listings and the calls in recoveries are
rendered from the `clap::Command` that the tool builds at run time, which
may hide or remove commands. The `Output` that `parse` returns holds that
tree.

- The `Output` keeps the tree as parsing left it, with the root and each
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
[`Output::command`](../../../crates/crumbkit/src/output.rs) states what it
builds.

Why: ADR-0015, ADR-0076.

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
`<tool>::cli::usage` and exits with 2, unless the tool's guide lists
another [exit code](../failures/exit-codes.md) for it. With `--json`, the
refusal prints as a `<tool>/failure/v1` document on standard output. The
rustdoc of `parse_to` states the message and the recovery.

- The refusal's message is clap's error on one line, except a view-flag
  refusal, which crumbkit words itself
  ([view flags](../output/README.md#view-flags)).
- The refusal's fields name what clap reports: `argument`, the refused
  argument; `value`, its refused value; and `subcommand`, an unknown
  subcommand, in that order. A refused `--limit` above its bound adds
  `most` ([listings](../output/listings/README.md#an-upper-bound-on---limit)).
- The recovery runs the `-h` of the deepest typed command, even a hidden
  one, unless the tool's refusal function returns a call that renders and
  parses ([refused command lines](../failures/README.md#refused-command-lines)).

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
