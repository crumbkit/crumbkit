# Failures

This page covers how a tool reports a failure: the `Failure` trait, codes and
their spelling, recoveries and alternatives, fields, refused command lines,
codes learned at run time, unclassified errors and the `miette` feature. The
other pages in this directory cover [deriving `Failure`](derive.md),
[failure forms](forms.md), which replace crumbkit's failure
document, and [exit codes](exit-codes.md).
[`Failure`](../../../crates/crumbkit-core/src/failure.rs),
[`Code`](../../../crates/crumbkit-core/src/code.rs) and
[`crumbkit::codes`](../../../crates/crumbkit/src/codes.rs) describe the items
themselves.

## The failure model

A tool's error types implement `Failure` beside `std::error::Error`, which
adds a stable code, a recovery and an optional url. The failure's `Display`
and its `source()` chain give the message and the causes, and Debug
formatting never reaches the output.

[`Error`](../../../crates/crumbkit-core/src/failure.rs) holds any failure, so
`?` works across a tool's error types. It has no context methods. `finish`
accepts a `Failure` or an `Error` through the sealed `AsFailure` trait,
because `Error` cannot implement `Failure`. A tool writes each `impl Failure`
by hand or [derives it](derive.md).

A failure prints as text on standard error, or as one failure document with
`--json` ([output](../output/README.md#standard-output-and-standard-error)). A
tool that publishes its own failure document prints every failure in that
form ([failure forms](forms.md)).

Why: ADR-0001.

## Recoveries and alternatives

Every failure states a [`Recovery`](../../../crates/crumbkit-core/src/failure.rs).
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
fails a run that has already failed. A tool whose field names come from
another system, such as `requestId` from an API or `Content-Type` from a
header, lists those names with `Guide::field_names`. `finish` and `stream`
then keep a field with a listed name, and the served failure schema accepts
the listed names beside the rule ([schemas](../schemas/README.md)). The
checked-in schema keeps the rule alone, because it describes crumbkit's
document for every tool. `check_guide` reports a listed name that is empty,
holds whitespace or a control character, is listed twice, or already
follows the rule. A field's value can be a list, an object
or null as well as a scalar ([schemas](../schemas/README.md)). A tool's own
field shapes are a [declaration](../adoption/declarations.md) that needs no
opt-out.

Why: ADR-0001, ADR-0007, ADR-0061.

## Debug-build warnings

A build with debug assertions writes one `warning:` line on standard error
before a failure for each field that `finish` leaves out, and for each `Run`
call of its recovery or alternatives that does not render, with the reason.
When the tool's `main` never called `Output::set_command`, `finish` and
`stream` write one `warning:` line in its place, which names `set_command`. A
tool's own tests then see the mistake, and `check_recoveries` fails on the
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
[`Spelling`](../../../crates/crumbkit-core/src/code.rs), and passes the same
spelling to `finish` and to `check_codes`. `Code` has no `Display`, so every
printed code goes through the tool's spelling. `check_codes` refuses
characters that let two different codes look the same, because callers match
a code exactly.

crumbkit's own codes are public constants in `crumbkit::codes`, which
`codes::ALL` lists, so a tool's tests can name them. A tool may print its own
code in place of any of crumbkit's, such as its own usage code for
`crumbkit::cli::usage`, with a `Rename` in `Guide::codes`. One command can
print its own code for one of crumbkit's failures with a `Rename` in
`Leaf::codes`, which its runs read before the guide's. crumbkit still
decides when the failure happens, its message and its recovery
([declarations](../adoption/declarations.md#codes-per-command)).

Why: ADR-0001, ADR-0056, ADR-0060.

## Codes learned at run time

A tool that learns a code only at run time, such as from a plugin's answer,
gets it from [`Code::runtime`](../../../crates/crumbkit-core/src/code/runtime.rs),
which checks the segments by the same rule and keeps each distinct code until
the process exits. A process holds at most 256 run-time codes. A new code
past that limit is refused, and the tool reports the failure under a code of
its own, so a plugin that puts an id in its reason cannot grow memory without
bound. `Spelling::split` reads a spelled code back into its segments.

`check_codes` cannot list a run-time code, so such a failure supplies its own
recovery. `check_recoveries` does not follow the recovery of a code from a
component that no declared code has, such as a plugin's, and counts such
codes in its report. It reports a code from a declared component that no
`CODES` list holds. A run-time code cannot be listed by an
[exit](exit-codes.md), so its failure exits with 1.

Why: ADR-0001, ADR-0037.

## Refused command lines

A command line that clap refuses, an unknown `--fields` name, and a view flag
that the command or another typed view flag rules out fail with the one
fixed code `crumbkit::cli::usage`. Their fields name the refused `argument`,
its `value` or the unknown `subcommand`, as clap reports them. A refused
command line's recovery runs the `-h` of the deepest typed command, even a
hidden one, except for a `--limit` above its
[bound](../output/listings/README.md#an-upper-bound-on---limit). A refused
command line exits with 2 unless the guide lists another code
([exit codes](exit-codes.md)).

A leaf may declare a function with `Leaf::refusal_alternatives` that builds
calls from a refusal of a command line that reaches the leaf, such as the
refused line with the refused value corrected. A guide may declare one such
function with `Guide::refusal_alternatives` for every command whose leaf sets
none, and a leaf's own function replaces it. The refusal offers each call
that renders and whose line parses, after crumbkit's own alternatives. It
leaves out the others with no warning and no trace event, so a function may
return a correction that it has not checked, and a trace never holds the
caller's values.

[Parsing](../parsing/README.md) covers when `parse` refuses a command line.

Why: ADR-0001, ADR-0056, ADR-0030.

## The `miette` feature

The optional `miette` feature lets a failure that implements
`miette::Diagnostic` show its labels, and crumbkit draws them itself
([failure forms](forms.md#source-excerpts)). The JSON document also holds the
diagnostic's related diagnostics and a source excerpt for each label. The
code, the recovery and the url still come from `Failure`.

With the feature, a `miette::Report` converts into `Error` with the code
`crumbkit::report::unchecked`
([`report`](../../../crates/crumbkit-core/src/report.rs)), so `?` works on a
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

[`Error::unclassified`](../../../crates/crumbkit-core/src/unclassified.rs)
holds any `std::error::Error` or message with the code
`crumbkit::report::unchecked` and a `Recovery::None`. A tool that uses an
error library such as anyhow therefore reaches `finish` without a
dependency of crumbkit on that library. The tool downcasts to its own
failure types first, because boxing the library's error hides the type
inside. The code is `codes::REPORT_UNCHECKED` in `codes::ALL`, so
`check_recoveries` follows a chain through it. crumbkit-core holds the
method and the code, because neither needs clap, and crumbkit re-exports the
code in its own `codes` module.

Why: ADR-0001, ADR-0032.
