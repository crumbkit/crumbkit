# ADR-0079. The tool's name in place of crumbkit's

- Date: 2026-10-17
- Status: accepted
- Supersedes: ADR-0001, ADR-0007, ADR-0046 and ADR-0058, which it amends
- Missed: ADR-0001, ADR-0046 and ADR-0058 gave crumbkit's own codes, the failure document and the output budget variable the name `crumbkit` in every tool's output, so a tool's users had to learn the name of a library that they never chose.

## Context

A tool printed `crumbkit/failure/v1` as the id of its failure document,
`crumbkit::cli::usage` and the other codes of `crumbkit::codes` as its
failure codes, and read `CRUMBKIT_OUTPUT_BUDGET` as its budget variable.
Its help notes, opt-out texts and command reference also named crumbkit.
A tool's user has no use for the name of the library that the tool is
built on. crumbkit enforces behaviour and lets a tool declare its own
names, and its own name in a tool's output was the exception to that.
Before this date, a tool could hide crumbkit's names only by renaming
each code with `Guide::codes` and by replacing the failure document with
a failure form.

## Decision

- With no setting, every name that crumbkit puts in a tool's output takes
  the tool's name in place of `crumbkit`. A tool named `shelf` prints
  `shelf/failure/v1`, `shelf::cli::usage` and `shelf::output::over_budget`,
  serves `shelf/schemas-list/v1` from `schemas list`, and reads
  `SHELF_OUTPUT_BUDGET`. `CRUMBKIT_OUTPUT_BUDGET` has no effect.
- The name is the clap name of the tool's root command. A guide whose
  `root` puts the tool below another program, such as a cargo subcommand,
  takes the last word of that root, so `cargo crumbkit` keeps the name
  `crumbkit`. `Guide::tool_name` derives it, and every entry point of
  crumbkit and crumbkit-test uses that one method.
- A document id keeps the name, lowercased, with `-` in place of each
  character that a document component does not allow. A code component
  has `_` in place of `-` and of each other character that a code segment
  does not allow, so `my-tool` prints `my-tool/failure/v1` and
  `my_tool::cli::usage`. A variable starts with the code component in
  capitals, `MY_TOOL_OUTPUT_BUDGET`, with `_` before a leading digit,
  `_2FA_OUTPUT_BUDGET`, because a shell variable cannot start with a
  digit. A name without an ASCII letter or digit is spelled `tool`.
- Only codes and documents whose component is `crumbkit` change. A tool's
  own codes, its renames of crumbkit's codes and the documents of its
  failure forms print as the tool declared them, so a per-code rename
  still wins.
- The Rust constants, such as `crumbkit::codes::CLI_USAGE`, and the
  checked-in schema files keep the `crumbkit` component, because only
  Rust code and crumbkit's own tests read them. `ToolName` spells them at
  the moment they reach a tool's output.
  The checked-in schema of the usage log's record is the exception: a
  reader of the log validates its lines against that file, so the file
  gives the record's id the pattern `<component>/usage/v1` in place of
  the const `crumbkit/usage/v1`.
- crumbkit-markdown, cargo-crumbkit and crumbkit-usage's report read the
  `commands-list/v1`, `self-figures-list/v1` and `usage/v1` documents
  under any component, because each tool now prints its own.
- crumbkit-test's checks expect the tool's names. The recovery check
  follows an undeclared code in the tool's component only when it is one
  of crumbkit's own codes or of its add-on crates' codes, crumbkit-child's
  and crumbkit-aliases's, spelled with the tool's name, such as
  `shelf::child::failed`. Any other undeclared code in the tool's
  component, such as `shelf::cache::stale`, stays a problem. crumbkit-test
  keeps its own list of the add-on crates' codes, and a test compares it
  with the codes that those crates declare. The example and recovery
  checks fail a run on the warnings that crumbkit prints, which start with
  `warning: <tool> left out `, and leave a tool's own warnings alone.
- crumbkit adds no option. A tool that wants other names already renames
  codes with `Guide::codes` and replaces the failure document with a
  failure form (API-1).

## Consequences

- A tool's goldens, scripts and agents that read `crumbkit/...` ids or
  `crumbkit::` codes read the tool's names instead. crumbkit has not been
  released, so no published tool breaks.
- A crumbkit code and a tool's own code can now share a component. A
  tool that declares a code such as `shelf::cli::usage` itself collides
  with crumbkit's usage code, and `check_codes` does not detect it,
  because it compares the Rust values, whose components differ.
- crumbkit's tracing spans keep the target `crumbkit`, because the
  tracing macros need their names when the crate compiles. Only a
  subscriber that the tool's developer installs sees them.
- Panic messages of crumbkit-test's checks and the const checks of a
  guide still name crumbkit, because only the tool's developer reads
  them.

Considered: an option that keeps crumbkit's names, a prefix that the
guide sets in place of the root command's name, reading both the old and
the new variable, serving each document under both ids, changing the
component of the Rust constants, keeping the first word of a guide's root
as the name, accepting every undeclared code in the tool's component whose
area the tool does not declare.
