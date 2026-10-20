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
A `Run` call that the tool's command tree cannot render prints as `None` with
crumbkit's reason, so crumbkit never advertises a command the tool hid. A
`Run` call can carry concrete values with `Call::value`, so a failure can
suggest the exact command that repairs it. Rendering refuses a value that the
tool would refuse before it runs: a value for a flag, a value outside the
argument's possible values, and a positional value that clap would read as a
flag.

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
A field name follows the rule of a code segment. `finish` leaves out a field
whose name breaks the rule or repeats an earlier name, so a wrong name never
fails a run that has already failed. A field's value can be a list, an object
or null as well as a scalar ([schemas](../schemas/README.md)). A tool's own
field shapes are a [declaration](../adoption/declarations.md) that needs no
opt-out.

Why: ADR-0001, ADR-0007, ADR-0081.

## Debug-build warnings

A build with debug assertions writes one `warning:` line on standard error
before a failure for each field that `finish` leaves out, and for each `Run`
call of its recovery or alternatives that does not render, with the reason.
A tool's own tests then see the mistake, and `check_recoveries` fails on the
line. A release build writes no such line. These lines are every output that
differs between a debug build and a release build.

Why: ADR-0001.

## Codes and their spelling

A `Code` has three segments: the owning component, the area and the reason.
Each segment is one or more lowercase ASCII letters, digits and underscores,
and `Code::new` checks them at compile time. The area is a noun from the
tool's domain, such as `books`, so moving code does not change a code
(OUTPUT-2). Each error type lists its codes in a `CODES` list for
crumbkit-test's `check_codes`.

Each tool chooses how its codes are spelled with one
[`Spelling`](../../../crates/crumbkit/src/code.rs), which it sets with
`Guide::spelling`. A guide that sets none spells codes with
`Spelling::COLONS`. `parse` and `Output::new` carry the guide's spelling in
the `Output`, so `finish`, `stream` and `hand_over` print in it, and
`check_codes` reads it from the same guide. `Code` has no `Display`, so every
printed code goes through the tool's spelling. `check_codes` refuses
characters that let two different codes look the same, because callers match
a code exactly.

crumbkit's own codes are public constants in `crumbkit::codes`, which
`codes::ALL` lists, so a tool's tests can name them. A tool may print its own
code in place of any of crumbkit's, such as its own usage code for
`<tool>::cli::usage`, with a `Rename` in `Guide::codes`. One command can
print its own code for one of crumbkit's failures with a `Rename` in
`Leaf::codes`, which its runs read before the guide's. crumbkit still
decides when the failure happens, its message and its recovery
([declarations](../adoption/declarations.md#codes-per-command)).

Why: ADR-0001, ADR-0086, ADR-0056, ADR-0060.

## The tool's name

A tool's output names the tool wherever crumbkit's own names would appear.
crumbkit's codes print with the tool's name as their component, so a tool
named `shelf` prints `shelf::cli::usage`. Its documents print the tool's
name in their ids, such as `shelf/failure/v1`, and the output budget
variable is `SHELF_OUTPUT_BUDGET`. The pages of this tree write the name as
`<tool>` and the variable as `<TOOL>_OUTPUT_BUDGET`. Help, notes and the
command reference name neither crumbkit nor its crates.

The name is the clap name of the root command, or the last word of the
guide's `root` when the tool runs below another program, so
`cargo crumbkit` keeps the name `crumbkit`.
[`Guide::tool_name`](../../../crates/crumbkit/src/help.rs) derives it, and
[`ToolName`](../../../crates/crumbkit/src/tool_name.rs) spells it:

- A document id lowercases the name and has `-` in place of each character
  that a document component does not allow, so `my-tool` prints
  `my-tool/failure/v1`.
- A code component lowercases the name and has `_` in place of `-` and of
  each other character that a code segment does not allow, so `my-tool`
  prints `my_tool::cli::usage`.
- A variable starts with the code component in capitals, such as
  `MY_TOOL_OUTPUT_BUDGET`. When the component starts with a digit, the
  variable starts with `_`, because a POSIX shell cannot set a variable
  whose name starts with a digit. A tool named `2fa` reads
  `_2FA_OUTPUT_BUDGET`.
- A name without an ASCII letter or digit is spelled `tool` in each place.

Only crumbkit's own codes and documents change. A tool's own codes, its
renames in `Guide::codes` and `Leaf::codes`, and the document of its failure
form print as the tool declared them. The Rust constants, such as
`crumbkit::codes::CLI_USAGE`, and the checked-in schema files keep the
`crumbkit` component, and the tool's output spells them with the tool's
name. A tool's own code that equals one of crumbkit's codes spelled with
the tool's name, such as a declared `shelf::cli::usage`, is not detected
by `check_codes`.

Why: ADR-0079.

## Codes learned at run time

A tool that learns a code only at run time, such as from a plugin's answer,
gets it from [`Code::runtime`](../../../crates/crumbkit/src/code/runtime.rs),
which checks the segments by the same rule and keeps each distinct code until
the process exits. A process holds at most 256 run-time codes. A new code
past that limit is refused, and the tool reports the failure under a code of
its own, so a plugin that puts an id in its reason cannot grow memory without
bound. `Spelling::split` reads a code in `Spelling::COLONS` back into its
segments, and returns `None` for a custom spelling, so a tool calls
`guide.get_spelling().split(text)` and passes the segments to
`Code::runtime`. A tool with a custom spelling gets `None` for every code
from that call, so it splits the text itself before it calls
`Code::runtime`.

`check_codes` cannot list a run-time code, so such a failure supplies its own
recovery. `check_recoveries` does not follow the recovery of a code from a
component that no declared code has, such as a plugin's, and counts such
codes in its report. It reports a code from a declared component that is
not declared. It finds a printed code's component from the text that
the spelling writes before the area, as
[recovery chains](../checks/recovery-chains.md) describes. A run-time code
cannot be listed by an [exit](exit-codes.md), so its failure exits with 1.

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
command line's recovery runs the `-h` of the deepest typed command, even a
hidden one, unless the tool's function below returns a call that it keeps or
the line sets a `--limit` above its
[bound](../output/listings/README.md#an-upper-bound-on---limit). A refused
command line exits with 2 unless the guide lists another code
([exit codes](exit-codes.md)).

A command line that stops at a group says that the group needs a
subcommand and names the group's visible subcommands in the order that the
tool declares them, as in
``the command `shelf books` needs a subcommand: list, show, summarize, remove``.
A hidden subcommand is left out. A command line that stops at a leaf that
needs arguments says that the command needs arguments.

A command line that leaves out a required argument with possible values
names the argument's visible possible values right after the argument, in
the form that clap uses for an invalid value, as in
`the following required arguments were not provided: <SHELL> [possible values: bash, fish, zsh]`.
A missing argument without possible values is named alone.

A leaf may declare a function with `Leaf::refusal_alternatives` that builds
calls from a refusal of a command line that reaches the leaf, such as the
refused line with the refused value corrected. A guide may declare one such
function with `Guide::refusal_alternatives` for every command whose leaf sets
none, and a leaf's own function replaces it. The refusal keeps each call
that renders and whose line parses, in the order that the function returned
them. The first kept call becomes the recovery, and the `-h` call becomes
the first alternative, followed by the other kept calls. With no kept call,
the `-h` call stays the recovery. A `--limit` above its bound keeps its
`--all` recovery and offers the kept calls after crumbkit's own
alternatives. The refusal leaves out the calls that it does not keep with no
warning and no trace event, so a function may return a correction that it
has not checked, and a trace never holds the caller's values.

[Parsing](../parsing/README.md) covers when `parse` refuses a command line.

Why: ADR-0001, ADR-0056, ADR-0030, ADR-0078, ADR-0106.

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

A diagnostic's own `code()` does not become the failure code. miette's
code is text that the diagnostic decides at run time, and a code that
`Failure::code` returns is a declared `&'static Code` that `check_codes`
can see. The failure's message ends with the diagnostic's code in
parentheses. A tool that wants its own code on a miette failure downcasts
the report to its own diagnostic type and returns a typed failure that
declares the code.

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
