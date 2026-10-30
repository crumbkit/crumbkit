# ADR-0163. Example preconditions

- Date: 2026-10-30
- Status: accepted
- Supersedes: ADR-0005, which it amends

## Context

`check_examples` ran every example of every leaf in a cleared environment.
Some examples need something that environment lacks, such as a running
container daemon for a command that acts on containers, or the files that
an earlier command writes for a command that reads them back. A tool
skipped each such example with `Allow::example` and a reason. The reason
then lived in the opt-out list, away from the example it explains, and the
opt-out report showed a correct example as a departure from the
conventions. No test run ever ran the example, even a CI job that had the
container daemon.

One adopter asked for this. Other tools whose commands drive a daemon, a
server or files from an earlier step are likely to need the same.

## Decision

- An `Example` can name the preconditions that its run needs with the
  const builder `Example::needs`, such as
  `Example::new(&["--all"], "Restart every container").needs(&["docker"])`.
  A precondition is a name alone, with no setup step.
- `Examples::provides` lists the names that a test run provides. The tool's
  own test decides them, such as from a variable of its own that a CI job
  sets. crumbkit-test reads no environment variable for this.
- `check_examples` and `check_output_budget` run an example only when the
  run provides every name that it needs. `check_examples` writes one line
  for each example that it skips, with the names that the run lacks, to
  the test's captured standard error.
- A skipped example keeps its golden files, and a run without its
  preconditions never reads, rewrites or deletes them, because the run
  that provides the preconditions owns them.
- `check_guide` still parses every example, skipped or not.
- `check_opt_outs` lists each example that needs a precondition among the
  customisations, as an `example-precondition`, because the example
  follows the conventions and needs no reason.
- Help and `commands list` do not show preconditions. A reader of help
  learns what a command needs from its about text, and showing the names
  would change the `commands list` document for a fact that only the
  tests use.

## Consequences

A tool moves an example that it skipped with `Allow::example` for a missing
resource to `Example::needs`, and its opt-out report moves the entry from the
opt-outs to the customisations. A CI job that provides the resource runs the
example and owns its golden files, and a local run without it skips the
example and leaves those files alone. A local run therefore never notices
that such an example's golden file is missing or out of date. A skipped
example's line shows only with `--nocapture` or when the test fails,
because a passing test's captured output is hidden.

Considered: a setup step that creates the precondition, such as running an
earlier command first, which would let the example run everywhere and was
left for a later need; reading the provided names from an environment
variable that crumbkit-test names; failing the check when a run provides a
name that no example needs; showing preconditions in `--help` or in the
`commands list` document.
