# ADR-0005. End-to-end tests

- Date: 2026-10-07
- Status: amended by ADR-0085 and ADR-0163

## Context

A tool's help shows command lines as examples, and an agent copies them.
A refusal test can pass for the wrong reason when the command fails for
some cause other than the one under test. crumbkit needed end-to-end
checks that run the built binary, that do not depend on the developer's
environment, and that a run which hangs cannot stall.

## Decision

- End-to-end tests ran the built binary with a cleared environment and
  fresh XDG directories, so a run depends only on what the test gives it.
  The checks run in an integration test of the binary's own package,
  because cargo gives the binary's path only there.
- `check_examples` ran every example of every leaf in the guide, so each
  command line that help shows is a run that works. A run that exits with
  a code other than 0, or prints crumbkit's debug warning, fails the
  check. The examples run with no shell, so a run depends only on what the
  example and the check give it.
- The examples got settings for a working directory, a fixture copied
  fresh for each run, an XDG fixture, a standard-input file and
  redactions, so a tool whose commands read files, configuration or input
  can still show stable, isolated examples. A redaction replaces a value
  with a named placeholder, because a test cannot know a temporary path.
- The checks ran the binary through `std::process` with their own
  timeout, because snapbox's `cmd` timeout does not say whether it killed
  the run and could report a stopped run as a failed one. The check stops
  waiting for the output at the timeout, so a process that the run
  started and that keeps a pipe open makes a reported problem instead of
  a hang.
- A refusal test used a twin: a command line that differs from the
  refused one by one word and succeeds, so the changed word is shown to be
  the reason (TEST-1). `check_twin` refuses twins that prove nothing, such
  as an admitted line that only asks for help or runs another command. It
  reads a line's command from the command tree with `parse`'s own walk,
  because the guide's leaves or the leading words alone miss global
  options and aliases. `check_twins` fails on a declared code that no twin
  refuses, unless an opt-out explains it.
- A code that no single word causes got an opt-out that holds the failing
  line, and a code that an environment variable causes got a variable
  twin, so these codes are still provoked by a test.

## Consequences

Every adopter of crumbkit-test needs a library target that exports its
command tree and guide. `docs/behaviour/checks/end-to-end.md` describes
the current settings and rules.

Considered: snapbox's wildcards such as `[..]` in example golden files, regular expressions as redaction values, a `.stdin` file named after the example's position in its leaf, deleting a stale `.stdin` file with `SNAPSHOTS=overwrite`, an opt-out that names an example by its position, trycmd, assert_cmd, running the examples through a shell, one golden file that holds standard output and standard error together, a golden file of the exit code, a `.stderr` golden file for every example even when it is empty, failing every run that writes to standard error, the inherited environment for example runs, a tool variable that wins over its fresh XDG directory, a check that fails when both `current_dir` and `fresh_dir` are set, a fresh directory inside `current_dir`, `[CWD]` for the fresh directory, snapbox's `cmd` timeout for the checks' runs, the wait-timeout crate as a direct dependency, finding the binary from a unit test, including the tool's source files in the test with `#[path]`, twins that differ by any number of words, `check_twin` adding `--json`, a setting that turns `--json` on for `check_twins`, an opt-out that names the code alone and leaves `check_recoveries` to provoke it, an opt-out that names the command and requires `check_twins` to take the clap tree, a twins-only list of untwinned codes outside the opt-out report, refusing `--dry-run` as the changed word, resolving a twin's command from the guide's leaves, skipping the leading option words of a line without the tree, a setting that lists the global options that take a value, a variable twin in `check_twin`, a variable twin that names the variable's admitted value, a required twin for a declared code that a rename prints in place of crumbkit's, waiting for the output threads without a limit, killing the run's process group at the timeout.
