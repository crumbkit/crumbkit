# Parsing

This page covers how a tool parses its command line with crumbkit: the
`parse` call in `main`, the `Stop` it returns when it ends the run itself,
the command tree that it hands on to `finish`, command lines that crumbkit
refuses, `try_parse`, which returns a refusal unprinted, and how a tool
keeps clap's matches after parsing. [Aliases](aliases.md) and
[shell completion](completion.md) have pages of their own.

## Parsing in main

A tool's `main` parses with [`parse`](../../../crates/crumbkit/src/parse.rs)
in place of clap's `Cli::parse`. `parse` returns the exit code instead of
calling `process::exit`, so every invocation ends by returning from `main`.
Tests run the same parse in memory through `parse_to`, which writes to a
sink of in-memory writers. The rustdoc of `parse`, `parse_from` and
`parse_to` states what each prints and the help lines that crumbkit adds.

A sink asks for no styles by default, so `parse_to` writes plain help. A
sink built with `Sink::styled(true)` receives help with clap's styles when
crumbkit's `color` feature is on, so a tool or a test can render help for a
terminal through its own writer ([help](../help/README.md)). `parse` and
`parse_from` mark their standard sink as styled when standard output is a
terminal and terminal detection is on.

On success, `parse` returns the tool's parsed type and an
[`Output`](../../../crates/crumbkit/src/output.rs). The `Output` holds the
view flags, the command tree, the guide's settings with the spelling of its
codes, the words the caller
typed with any values that a [picker](../effects/pickers.md) added, whether
the caller passed `--no-input` or `--dry-run`, and the code renames of the
leaf that the words name. `Output::confirm` repeats those words with `--yes`
in its recovery.

A tool flattens [`ViewFlags`](../../../crates/crumbkit/src/output/build.rs)
into its clap types to accept `--json`, `--fields`, `--long` and `--full`.
`parse` reads them before the tool's type takes its values, so a handler
reads them through the `Output`. A tool that does not flatten `ViewFlags`
parses with every view flag off.

A tool that parses with clap alone builds its `Output` with
`Output::new(view, dry_run, command, &GUIDE, args)`, where `dry_run` is the
`DryRun` that the command parsed, or `DryRun::default()` in a tool without
the flag, and `args` is the argv with the program name first. `Output::new`
returns `Err` with the `Stop` of a refused command line when `--full` comes
without `--json` or `--fields`
comes with `--long` or `--full`, after it printed the refusal on standard
error, so a tool's `main` returns the stop's code as it does after `parse`.
`Output::new_to` prints the refusal to a sink instead. `Output::new` finds
the leaf that the words name, so the run gets that leaf's code renames, and
it keeps the words for the `--yes` recovery. It takes the spelling of codes
from the guide, as `parse` does. It reads no `--no-input`.

A tool with personal aliases passes its arguments through
`crumbkit_aliases::expand` before `parse_from`, as the
[aliases page](aliases.md) describes. A tool that reads a refused command
line before it prints calls `try_parse`
([a refusal that the tool prints](#a-refusal-that-the-tool-prints)).

Why: ADR-0015, ADR-0076, ADR-0086, ADR-0002, ADR-0102.

## The Stop of a run that parse ended

When `parse` prints help, the version or a refusal, it ends the run and
returns a [`Stop`](../../../crates/crumbkit/src/parse/stop.rs) in place of a
bare `ExitCode`. `try_parse` returns the `Stop` of help or the version
inside `Unparsed::Stopped`, and `Refused::print` returns the `Stop` of a
refusal. A tool's `main` returns `stop.code()`. A tool that prints
more before it exits reads `Stop::output`, whose `Output::is_json` says
whether the caller asked for JSON. `Stop::number` returns the same exit code
as a `u8`: 0 after help or the version, the code that the guide lists for a
refused command line, which is 2 unless an exit lists the refusal, and 1
when the page could not be written. A tool that records the exit of a
stopped run, such as in a log of its own, or passes it on reads that
number, because an `ExitCode` cannot be read back. `Stop::printed` returns
a `Printed` that says what printed before the run ended: `Help` for `-h`,
`--help` or the word `help`, `Version`, `Topic` for a topic that `help`
at the root prints, `Refusal` for a refused command line or for view flags
that cannot work together, and `Failure` for the failure of a picker's
source. A tool that records the outcome of a stopped run in a history of
its own reads it, because the number is 0 after help, the version and a
topic alike. `Printed` is `#[non_exhaustive]`. A `Stop` offers `path`,
`flags` and `value` as a `Refused` does, so a tool that prints a note
after a help page reads a global option such as `--format` from it. The
`Output`, the path and the words inside a `Stop` sit in a `Box`, which
costs one allocation only on a run that `parse` ends.

Why: ADR-0002, ADR-0089, ADR-0107, ADR-0111.

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

Those commands therefore take `&mut Output`, `finish` takes the `Output`
by value because nothing reads it after, a handler that reads the tree runs
before `finish`, and the work of a run grows with the command typed. A
`Call` that does not resolve against the tree is left out,
or prints as `Recovery::None`. crumbkit prints a hidden command or flag only
where the caller typed it. The rustdoc of
[`Output::command`](../../../crates/crumbkit/src/output.rs) states what it
builds.

Why: ADR-0015, ADR-0076, ADR-0094.

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
  one, unless the tool's hint function returns a sentence or its refusal
  function returns a call that renders and parses
  ([refused command lines](../failures/README.md#refused-command-lines)).
- In text, a refusal from clap prints clap's usage as `usage:` lines under
  the message. A command line that stops at a group with visible
  subcommands prints the group's `-h` page in place of the refusal lines.

The walk that finds the deepest command also finds `--json`, which makes a
refusal print as a JSON document. It counts the word only where it reads it
as a flag, so a value of an option that allows hyphen values is no request
for JSON. It also counts the word only where the typed command declares
`--json` or a command above it declares a global `--json`, so a refusal on a
command without the flag prints as text. A tool cannot name its own output flags for the walk to treat as
`--json`, because on the parsed path crumbkit cannot set the `Output` that
the tool's own type builds, and `Output::is_json` would answer differently
on the two paths ([help](../help/README.md#the-walk-before-parsing)).

The refusal names real names only. When clap suggests a hidden alias of a
flag or subcommand, the refusal names the real flag or subcommand that the
alias belongs to, and it leaves the suggestion out when that real item is
hidden. A tool can add its own alternatives or a hint to a refusal through
a [declaration](../adoption/declarations.md).

A command that refuses the caller after parsing, such as a handler that
rejects a value, returns an ordinary `Failure`, which `finish` prints.
crumbkit-test's checks and the golden files cover the whole command tree.

Why: ADR-0015, ADR-0001, ADR-0021, ADR-0050, ADR-0118, ADR-0123.

## A refusal that the tool prints

[`try_parse`](../../../crates/crumbkit/src/parse.rs), `try_parse_from` and
`try_parse_to` parse as `parse`, `parse_from` and `parse_to` do and take the
same arguments. They print help, the version and a topic, and return
`Unparsed::Stopped` with the `Stop`. They print no refusal. A refused
command line comes back as `Unparsed::Refused` with a
[`Refused`](../../../crates/crumbkit/src/parse/refused.rs), and `parse` is
`try_parse` followed by printing it.

A `Refused` holds what the refusal prints and what the run knew:

- `message`, `code` and `recovery`, which the refusal prints, with the code
  after the guide's renames and the recovery after the tool's declared hint
  or calls, so a tool that prints its own document prints its hint;
- `number`, the exit code that the run ends with once the refusal prints;
- `output`, the output flags as far as the walk found them;
- `typed`, the words after the program name as the caller typed them;
- `parsed`, the words that crumbkit gave clap, after it turned the word
  `help` into `--help`, replaced a dashed form or added a picked value;
- `path`, the real names of the subcommands from the root to the deepest
  command that the walk reached, which is empty at the root;
- `flags`, the clap ids of the arguments that `parsed` passes as flags, in
  the order typed, with each flag of a group such as `-qv`;
- `value`, the last value that `parsed` gives an option by its clap id,
  whether the option is the deepest command's own or a global option of a
  command above it, typed as the next word or after `=`;
- `culprit`, the word that clap refused, from clap's error context: an
  unknown subcommand, a refused value, or an unexpected word or flag, or
  the view flag that crumbkit refused. A missing argument has none.
- `error`, clap's error, whose `kind()` and `get(ContextKind)` name what
  clap refused, such as the argument that conflicted or was repeated. A
  refused view flag has none, because crumbkit refuses it after clap
  accepted the command line.

`tool help nonsense` is refused as `tool nonsense --help`, so `typed` holds
`help` and `nonsense`, `parsed` holds `nonsense` and `--help`, and
`culprit` is `nonsense`. A tool that resolves aliases of its own reads
`typed`, and a tool with its own failure document prints the refusal from
`message`, `code` and `recovery`. A tool that words a refusal in its own
sentence reads the kind and context of `error`. A tool that names the
deepest command's help, or records the refused run in a history of its
own, reads `path`, `flags` and a global option's `value`, such as the value
of `--format`, instead of reading the words again. A `Refused` has no
getter for the usage or the group page. `Refused::print` prints them, and
a tool that prints the refusal itself reads clap's usage from `error`.

`Refused::print` prints the refusal as `parse` prints it, and
`Refused::print_to` prints it to a sink. Each takes the `Refused` by value
and returns the `Stop`, so a run prints a refusal at most once. The run's
observers hear of the refusal when it prints, so a refusal that the tool
answers itself reports no `refused` event. `Unparsed` is
`#[non_exhaustive]`, and `Unparsed::print` prints whatever variant it holds.

Why: ADR-0100, ADR-0107, ADR-0109, ADR-0123.

## Reading clap's matches after parsing

`parse` and the other parse functions accept any tool type that implements
`clap::FromArgMatches`, and hand clap's `ArgMatches` to the type's
`from_arg_matches_mut`. A type that clap derives takes each value out of
the matches there. clap removes the whole entry of each argument that the
type takes, so `ArgMatches::value_source` answers `None` for that argument
afterwards.

A tool that needs each value's source, such as whether a value came from
the command line, an environment variable or a default, can keep the
matches without parsing the command line a second time. It wraps its
derived type in a type whose `FromArgMatches` implementation clones the
matches before the derived type takes the values:

```rust
struct WithMatches {
    cli: Cli,
    matches: ArgMatches,
}

impl FromArgMatches for WithMatches {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
        Self::from_arg_matches_mut(&mut matches.clone())
    }

    fn from_arg_matches_mut(matches: &mut ArgMatches) -> Result<Self, clap::Error> {
        let kept = matches.clone();
        Cli::from_arg_matches_mut(matches).map(|cli| Self { cli, matches: kept })
    }

    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), clap::Error> {
        self.update_from_arg_matches_mut(&mut matches.clone())
    }

    fn update_from_arg_matches_mut(&mut self, matches: &mut ArgMatches) -> Result<(), clap::Error> {
        let kept = matches.clone();
        self.cli.update_from_arg_matches_mut(matches)?;
        self.matches = kept;
        Ok(())
    }
}
```

The tool calls `crumbkit::parse::<WithMatches>(Cli::command(), &GUIDE)` and
reads a subcommand's sources through `ArgMatches::subcommand_matches`.
crumbkit reads `--json`, `--dry-run`, `--no-input` and the view flags from
the matches before the tool's type runs, so the wrapper changes nothing
that crumbkit reads or prints. Each parse clones the matches once. A
[picker](../effects/pickers.md) parses again after each value that it
adds. When crumbkit refuses a command line, it parses the line of each
call that the tool's refusal function returns. Those parses clone the
matches too.
[`matches.rs`](../../../crates/crumbkit/tests/matches.rs) runs the wrapper.
