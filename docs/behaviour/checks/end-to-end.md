# End-to-end checks

Several crumbkit-test checks run a tool's built binary. This page covers what
those runs share: where the binary comes from, how each run is isolated and
timed, and how twins prove a refusal.

## The binary and the settings

cargo gives the path of a built binary, `CARGO_BIN_EXE_<name>`, only to the
integration tests of the binary's own package. A tool therefore runs these
checks from such a test. An integration test imports only from a library, so
that package has a library target that exports the clap command and the
guide, and its binary calls the library, as shelf's does.

One [`Examples`](../../../crates/crumbkit-test/src/examples/settings.rs) value
holds the settings that every such check shares: the binary, the golden
directory, the environment, the working directory or fresh fixture, the XDG
fixture, the redactions, the opt-outs and the failure reader.
`check_examples`, `check_output_budget`, `check_twin`, `check_twins`,
`check_recoveries` and `Pages::binary` all read it.

Why: ADR-0005.

## Isolation of each run

Each run has no shell, a cleared environment and fresh XDG directories, so it
depends only on what the example and the check give it. The table shows how
often each check makes a fresh copy of the directories that the settings name.

| Setting | `check_examples`, `check_output_budget`, `Pages::binary` | `check_twin` | `check_recoveries` |
| --- | --- | --- | --- |
| Fresh XDG directories, from `Examples::xdg` or `Failing::xdg` | Each run | Each line | Once for each chain |
| Copy of `Examples::fresh_dir` | Each run | Each line | Once for each chain |
| `Examples::current_dir` | Every run starts there | A `Failing` without its own directory starts there | A `Failing` without its own directory starts there |

A step of a recovery chain therefore sees what the earlier runs of its chain
wrote. `current_dir` and `fresh_dir` are one setting, and the later call wins.
A check fails without running anything when the directory or fixture does not
exist, and a run fails when a fixture holds a symbolic link.

Each example reads the bytes of `<name>.stdin` in the golden directory on
standard input, and a run without that file reads an empty standard input.
`check_recoveries` runs each start and step with the working directory,
standard input and XDG fixture described here.

The check replaces the path of a fresh copy and its canonical path with
`[ROOT]`, and the path of each fresh XDG directory with `[XDG_CONFIG_HOME]`,
`[XDG_DATA_HOME]`, `[XDG_STATE_HOME]` or `[XDG_CACHE_HOME]`. An XDG fixture
holds any of `config`, `data`, `state` and `cache`, and a fixture with any
other entry fails the check before it runs anything. `Failing::xdg` names
the fixture of one chain and wins over the fixture of `Examples`. The
variables of a `Failing` win over `Examples::env` and the check's XDG
directories.

Why: ADR-0005, ADR-0053.

## Example runs

[`check_examples`](../../../crates/crumbkit-test/src/examples.rs) runs
every example of every leaf in the guide, so each command line that help
shows is a run that works. It runs the examples of hidden commands, because
their own help lists them. The rustdoc of `check_examples` lists every
problem it reports. These rules matter most to a tool's author:

- A run that exits with a code other than 0 fails, and the problem names
  the failure's code and steps, read through the guide's failure form or
  the `FailureReader` of `Examples`.
- A run whose standard error holds a warning that crumbkit prints in a
  debug build fails, because such a warning marks a mistake in the tool.
  crumbkit's warnings start with `warning: <tool> left out `, such as
  `warning: shelf left out a recovery of the failure ...`. A tool's own
  warning, such as `warning: shelf cache is stale`, does not fail the run.
- `Allow::example` opts out of one example by its words, for an example
  whose output no redaction can make stable. It is stale when its command
  has no example with those words.
- The output weight read from a golden file counts the bytes of a
  placeholder in place of the value it replaced.

[Golden files](golden-files.md) lists the files the check keeps.

Why: ADR-0005, ADR-0036, ADR-0045, ADR-0050.

## Timeouts and pipes

The checks run the binary through `std::process`. Each run has a timeout of
10 seconds, which starts when the binary starts.

- The check waits for the run itself and kills it when the timeout ends, so
  it reports a stopped run as stopped, and never as failed.
- The kill ends the run's own process. A process that the run started
  survives it.
- The check pipes standard input, standard output and standard error. One
  thread writes the input and two threads read the output, so a tool that
  never reads an input larger than a pipe holds is stopped at the timeout and
  does not block the check.
- After the run exits, the check waits for the reading threads only until the
  timeout ends. A process that the run started and that still holds a pipe
  open makes the run a problem, and the check does not hang.
- When the check cannot learn whether the run exited, it kills the run before
  it reports the error.

These rules live in a private module, so this page states them. snapbox's
`cmd` timeout does not say whether it killed the run, and the wait-timeout
crate under it can return up to 1 ms before its timeout, so the checks keep
their own timer.

Why: ADR-0005.

## Twins

A test of a refusal also runs a twin: a command line that differs from the
refused one by one word added, removed or replaced, and that the tool accepts
(TEST-1). The twin shows that the changed word causes the refusal.

- [`check_twin`](../../../crates/crumbkit-test/src/twins.rs) runs one twin as
  written and returns the refused line's output for the test to compare.
- [`check_twins`](../../../crates/crumbkit-test/src/twins/coverage.rs) runs
  every twin of a tool with `--json` added to both lines when neither holds
  it, and fails on a declared code that no twin refuses.
- A code that no single word causes has an opt-out made with
  `Allow::untwinned`, which holds the `Failing` that provokes it and a reason.
- A code that an environment variable provokes has a variable twin made with
  `Twin::variable`, whose admitted line runs without the refused line's entry
  for that variable.
- A code that no test can provoke goes in the `Unprovoked` list, which a tool
  passes to both `check_twins` and `check_recoveries`.
- A code in the `crumbkit` component needs no twin and no declaration,
  because crumbkit or one of its add-on crates, such as crumbkit-child,
  raises that failure. A declared code that a `Rename` of the guide or of a
  leaf prints in place of one of them needs no twin either.
- Neither check keeps a golden file, because every twin and every listed code
  is in the test's source.

`check_twin` refuses two kinds of twin that prove nothing. In the first
kind, the admitted line adds `-h`, `--help`, `-V`, `--version` or a first
word `help`, or puts one of them in place of a word, so clap or `parse`
answers without running the command. In the second kind, the admitted line
runs another command than the refused line, or none. A refused line that
runs no command, such as the misspelt `books lst`, needs an admitted line
that runs one, such as `books list`.

`check_twins` runs the line of an `Allow::untwinned` opt-out with `--json`
added, and fails when the line exits with 0 or fails with another code. The
opt-out is stale when a twin refuses its code, when `Unprovoked` lists the
code, or when the code is one of crumbkit's own. A problem names a line
with its variables, such as `` `tool books list` (with DEPTH=1) ``.

`check_twin` and `check_twins` take the tool's unbuilt command tree and read
the command of each line with `crumbkit::command_path`, the walk that `parse`
makes. A test passes the tree that the binary builds for the twin's run, such
as the tree of an empty directory for a tool that adds commands at run time.

Why: ADR-0005, ADR-0060.
