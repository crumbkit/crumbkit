# Failures

This page covers how a tool reports a failure: the `Failure` trait, codes and
their spelling, recoveries and alternatives, fields, refused command lines,
codes learned at run time, unclassified errors and the `miette` feature. The
other pages in this directory cover [deriving `Failure`](derive.md),
[source excerpts and failure readers](excerpts-and-readers.md) and
[exit codes](exit-codes.md).
[`Failure`](../../../crates/crumbkit/src/failure.rs),
[`Code`](../../../crates/crumbkit/src/code.rs) and
[`crumbkit::codes`](../../../crates/crumbkit/src/codes.rs) describe the items
themselves.

## The failure model

A tool's error types implement `Failure` beside `std::error::Error`, which
adds a stable code, a recovery and an optional url. The failure's `Display`
and its `source()` chain give the message and the causes, and Debug
formatting never reaches the output.

[`Error`](../../../crates/crumbkit/src/failure.rs) holds any failure, so
`?` works across a tool's error types. It has no context methods. `finish`
accepts a `Failure` or an `Error` through the sealed `AsFailure` trait,
because `Error` cannot implement `Failure`. A tool writes each `impl Failure`
by hand or [derives it](derive.md).

A failure prints as text on standard error, or as one failure document with
`--json` ([output](../output/README.md#standard-output-and-standard-error)).

Why: ADR-0001.

## Recoveries and alternatives

Every failure states a [`Recovery`](../../../crates/crumbkit/src/failure.rs).
A `Recovery::Run` call that the tool's command tree cannot render prints as
`None` with crumbkit's reason, so crumbkit never advertises a command the tool
hid. A `Recovery::Run` call can carry concrete values with `Call::value`, so a
failure can suggest the exact command that repairs it. Rendering refuses a
value that the tool would refuse before it runs: a value for a flag, a value
outside the argument's possible values, and a positional value that clap would
read as a flag.

`Recovery::Terminal` names a command that a person must run in a terminal,
such as one that opens an editor or asks questions. Text prints it as
``run `shelf login` in a terminal``, and JSON holds
`{"kind":"terminal","call":"shelf login"}`, so a program that reads the
failure can hand the step to a person and not run it itself. A call that the
tool's command tree cannot render behaves as a `Recovery::Run` call that
cannot render.

`Recovery::None` with a reason is the exception for a failure that no step
can repair. A tool declares each such failure with one of its declared codes
through an opt-out from `check_recoveries`'s `Rule::NextStep`
([recovery chains](../checks/recovery-chains.md)).

A failure may offer alternative repairs through `Failure::alternatives`, such
as setting a token variable beside logging in. An alternative that offers no
step is left out. When the recovery offers no step, the first alternative
that offers one takes its place, so the output never says that there is no
next step and then names one. In text, each alternative prints on its own
`help:` line after the recovery's.

Why: ADR-0001.

## Fields

A failure names the values at fault through `Failure::fields`, such as the id
of a missing book, so a program reads the value without parsing the message.
A field name is one or more lowercase ASCII letters, digits and underscores.
`finish` leaves out a field whose name breaks the rule or repeats an earlier
name, so a wrong name never fails a run that has already failed. A field's
value can be a list, an object or null as well as a scalar
([schemas](../schemas/README.md)). A tool's own field shapes are a
[declaration](../adoption/declarations.md) that needs no opt-out.

Why: ADR-0001, ADR-0007, ADR-0081.

## Debug-build warnings

A build with debug assertions writes one `warning:` line on standard error
before a failure for each field that `finish` leaves out, and for each
`Recovery::Run` or `Recovery::Terminal` call of its recovery or alternatives
that does not render,
with the reason. A tool's own tests then see the mistake, and
`check_recoveries` fails on the line. A release build writes no such line.
These lines are every output that differs between a debug build and a release
build.

Why: ADR-0001.

## Codes and their spelling

A `Code` has three segments: the owning component, the area and the reason. Each
segment is one or more words joined by single hyphens, where each word is
lowercase ASCII letters, digits and underscores, such as `not_found` or
`book-club`. A hyphen never leads, trails or follows another hyphen, so a code
never starts like a flag. `Code::new` checks the segments at compile time, and
`Code::runtime` applies the same rule to a code learnt at run time, such as a
plugin's. The area is a noun from the tool's domain, such as `books`, so moving
code does not change a code (OUTPUT-2). Each error type lists its codes in a
`CODES` list, which crumbkit-test's `check_twins` and `check_recoveries` read.

Every code prints in [`Spelling::COLONS`](../../../crates/crumbkit/src/code.rs),
which joins the segments with `::`. `parse` and `Run::new` carry the spelling in
the `Run`, so `finish`, `stream` and `hand_over` print in it. `Code` has no
`Display`, so every printed code goes through the spelling. `Code::new` refuses
characters that let two different codes look the same, because callers match a
code exactly.

crumbkit's own codes are public constants in `crumbkit::codes`, which
`codes::ALL` lists, so a tool's tests can name them. A tool's failures print
these codes with the tool's name as the component, as the next section
describes. crumbkit decides when each failure happens, its message and its
recovery.

Why: ADR-0001, ADR-0086, ADR-0056, ADR-0156, ADR-0186.

## The tool's name

A tool's output names the tool wherever crumbkit's own names would appear.
crumbkit's codes print with the tool's name as their component, so a tool
named `shelf` prints `shelf::cli::usage`. Its documents print the tool's
name in their ids, such as `shelf/failure/v1`, and the output budget
variable is `SHELF_OUTPUT_BUDGET`. The pages of this tree write the name as
`<tool>` and the variable as `<TOOL>_OUTPUT_BUDGET`. Help, notes and the
command reference name neither crumbkit nor its crates.

The name is the clap name of the root command, or the last word of the
tool record's `root` when the tool runs below another program, so
`cargo crumbkit` keeps the name `crumbkit`.
[`Tool::tool_name`](../../../crates/crumbkit/src/tool.rs) derives it, and
[`ToolName`](../../../crates/crumbkit/src/tool_name.rs) spells it:

- A document id lowercases the name and has `-` in place of each character
  that a document component does not allow, so `my-tool` prints
  `my-tool/failure/v1`.
- A code component lowercases the name and has `_` in place of each
  character other than a lowercase ASCII letter, a digit and `_`, including
  each `-`, so `my-tool` prints `my_tool::cli::usage`. The component is
  also the start of the tool's environment variables, which cannot hold a
  `-`.
- A variable starts with the code component in capitals, such as
  `MY_TOOL_OUTPUT_BUDGET`. When the component starts with a digit, the
  variable starts with `_`, because a POSIX shell cannot set a variable
  whose name starts with a digit. A tool named `2fa` reads
  `_2FA_OUTPUT_BUDGET`.
- A name without an ASCII letter or digit is spelled `tool` in each place.

Only crumbkit's own codes and documents change. A tool's own codes and the
document of its failure form print as the tool declared them. The Rust
constants, such as `crumbkit::codes::CLI_USAGE`, and the checked-in schema files
keep the `crumbkit` component, and the tool's output spells them with the tool's
name. A tool's own code that equals one of crumbkit's codes spelled with the
tool's name, such as a declared `shelf::cli::usage`, is not detected by any
check.

Why: ADR-0079.

## Codes learned at run time

A tool that learns a code only at run time, such as from a plugin's answer, gets
it from [`Code::runtime`](../../../crates/crumbkit/src/code/runtime.rs), which
checks the segments by the same rule and keeps each distinct code until the
process exits. A process holds at most 256 run-time codes. A new code past that
limit is refused, and the tool reports the failure under a code of its own, so a
plugin that puts an id in its reason cannot grow memory without bound.
`Spelling::COLONS.split(text)` reads a printed code back into its segments, so a
tool passes the segments to `Code::runtime`.

A `CODES` list cannot name a run-time code, so such a failure supplies its own
recovery. `check_recoveries` does not follow the recovery of a code from a
component that no declared code has, such as a plugin's, and counts such codes
in its report. It reports a code from a declared component that is not declared.
It finds a printed code's component from the text before the area, as [recovery
chains](../checks/recovery-chains.md) describes. A run-time code cannot be
listed by an [exit](exit-codes.md), so its failure exits with 1.

A tool whose tests hold its declared codes by value, such as a `Vec<Code>`
built from `Code::runtime`, passes them to `Recoveries::from_codes` and
`Twins::from_codes`, and lists a code that no test provokes with
`Unprovoked::from_code`. These constructors take the owned codes directly.
`Recoveries::new` and `Twins::new` take a slice of lists of references, so
with them the tool first collects a `Vec<&Code>` from its codes and wraps
it in an outer slice.

Why: ADR-0001, ADR-0037, ADR-0085, ADR-0088, ADR-0089.

## Refused command lines

A command line that clap refuses, an unknown `--fields` name, and a view flag
that the command or another typed view flag rules out fail with the one
fixed code `<tool>::cli::usage`. Their fields name the refused `argument`,
its `value` or the unknown `subcommand`, as clap reports them. A refused
command line's recovery runs the `-h` of the deepest typed command that has
a help flag, even a hidden one, unless the tool's functions below return a
hint or a call that it keeps, or the line sets a `--limit` above its
[bound](../output/listings/README.md#an-upper-bound-on---limit). A command
that turned its help flag off, such as a hidden leaf that answers a shell's
completion requests, has no `-h` page, so the recovery names the nearest
command above it with a help flag. When no command on the typed path has a
help flag, the recovery offers no step, and its reason says that no command
on the line has a `-h` help page. A refused
command line exits with 2 unless the tool record lists another code
([exit codes](exit-codes.md)).

A command line that stops at a group with visible subcommands prints the
group's `-h` page on standard error in text, with no `error:`, `help:` or
`code:` line, and exits with the code of a refused command line. Its failure
document says that the group needs a subcommand and names the group's
visible subcommands in the order that the tool declares them, as in
``the command `shelf books` needs a subcommand: list, show, summarize, remove``.
A hidden subcommand is left out. A command line that stops at a leaf that
needs arguments says that the command needs arguments.

In text, a refusal from clap prints clap's usage under the `error:` line,
one `usage:` line for each form that clap prints, as in
`usage: shelf books show <ID>...`. The usage names the tool as help does.
clap leaves out of it the options that the line does not need, so it can
differ from the usage line of help. clap prints a usage only with its
`usage` feature, which clap's default features enable, and never for an
empty value, a value outside the possible values, or a value that an
argument's parser rejects. A refusal that crumbkit words
itself, for a view flag, an unknown `--fields` name or a `--limit` above
its bound, prints no usage line. The failure document holds no usage.

A command line that leaves out a required argument with possible values
names the argument's visible possible values right after the argument, in
the form that clap uses for an invalid value, as in
`the following required arguments were not provided: <SHELL> [possible values: bash, fish, zsh]`.
A missing argument without possible values is named alone. A refusal that
names several missing arguments separates them with commas, as in
`the following required arguments were not provided: <SHELL> [possible values: bash, fish, zsh], <DIR>`.

A leaf may declare a function with `Leaf::refusal_alternatives` that builds
calls from a refusal of a command line that reaches the leaf, such as the
refused line with the refused value corrected. A tool record may declare one
such function with `Failures::refusal_alternatives` for every command whose leaf
sets none, and a leaf's own function replaces it. The refusal keeps each call
that renders and whose line parses, in the order that the function returned
them. The first kept call becomes the recovery, and the `-h` call becomes the
first alternative, followed by the other kept calls. A path without a help flag
has no `-h` call, so the other kept calls follow the recovery alone. With no
kept call, the `-h` call, or the recovery without a step, stays the recovery. A
`--limit` above its bound keeps its `--all` recovery and offers the kept calls
after crumbkit's own alternatives. The refusal leaves out the calls that it does
not keep with no warning and no trace event, so a function may return a
correction that it has not checked, and a trace never holds the caller's values.

A leaf may also declare a function with `Leaf::refusal_hint` that returns a
sentence for a refusal of a command line that reaches the leaf, and a tool
record may declare one with `Failures::refusal_hint` for every command whose
leaf sets none. The function reads the leaf's command, the kind of clap's error
and the words from its `RefusalContext`, so it can tell the caller of a command
whose last positional takes the words after `--` to put an unknown flag there.
When it returns a sentence, the sentence becomes the recovery as a hint: the
first `help:` line in text, and a `hint` in the failure document. The `-h` call
becomes the first alternative, followed by the kept calls in their order. When
it returns `None`, the refusal recovers as the paragraph above says. A `--limit`
above its bound keeps its `--all` recovery and offers the sentence after
crumbkit's own alternatives and before the kept calls.

[Parsing](../parsing/README.md) covers when `parse` refuses a command line.

Why: ADR-0001, ADR-0056, ADR-0030, ADR-0078, ADR-0106, ADR-0118, ADR-0123,
ADR-0124.

## The `miette` feature

The optional `miette` feature lets a failure that implements
`miette::Diagnostic` show its labels, and crumbkit draws them itself
([source excerpts](excerpts-and-readers.md#source-excerpts)). The JSON document also holds the
diagnostic's related diagnostics and a source excerpt for each label. The
code, the recovery and the url still come from `Failure`.

With the feature, a `miette::Report` converts into `Error` with the code
`<tool>::report::unchecked`
([`report`](../../../crates/crumbkit/src/report.rs)), so `?` works on a
`miette::Result`. The code tells the caller only that the failure was not
classified, so a tool that wants its own failure back calls
`report.downcast` before converting.

A diagnostic's own `code()` does not become the failure code. miette's code is
text that the diagnostic decides at run time, and a code that `Failure::code`
returns is a declared `&'static Code` that a `CODES` list can name. The
failure's message ends with the diagnostic's code in parentheses. A tool that
wants its own code on a miette failure downcasts the report to its own
diagnostic type and returns a typed failure that declares the code.

Why: ADR-0001, ADR-0016.

## Unclassified errors

[`Error::unclassified`](../../../crates/crumbkit/src/unclassified.rs)
holds any `std::error::Error` or message with the code
`<tool>::report::unchecked` and a `Recovery::None`. A tool that uses an
error library such as anyhow therefore reaches `finish` without a
dependency of crumbkit on that library. The tool downcasts to its own
failure types first, because boxing the library's error hides the type
inside. The code is `codes::REPORT_UNCHECKED` in `codes::ALL`, so
`check_recoveries` follows a chain through it.

Why: ADR-0001.
