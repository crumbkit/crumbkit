# Golden files

A golden file holds the exact bytes that a tool printed, and a check compares
a fresh run with it. This page covers how crumbkit-test's checks keep and
update golden files, and which files a tool keeps.

## Comparison and updates

snapbox compares each output with its golden file byte for byte. A check that
keeps golden files writes each missing file, rewrites each file that differs
and deletes each stale file when the environment variable
`SNAPSHOTS=overwrite` is set. Without it, a check fails on a missing,
differing or stale file and shows snapbox's report of both texts. A person
reviews the rewritten files with `git diff`.

Why: ADR-0004.

## Placeholders in example golden files

An example golden file holds the exact bytes of the run except the path of
each of the run's XDG directories, which the check replaces with a named
placeholder, such as `[XDG_CONFIG_HOME]`. snapbox's wildcards, such as
`[..]`, are plain text in an example golden file, so they match only
themselves. A tool whose output holds a value that changes on every run
opts that example out with `Allow::example`.

Why: ADR-0004, ADR-0005, ADR-0085.

## The golden files a tool keeps

A tool keeps its golden files under `tests/golden/` in the package that runs
the test, in the directories it passes to each check. shelf keeps these:

| Files | Check | Holds |
| --- | --- | --- |
| `tests/golden/help/*.txt` | `check_help_goldens` | One file for each `-h` and each `--help` page. |
| `tests/golden/examples/*.txt` | `check_examples` | The standard output of each example in a leaf's help. |
| `tests/golden/examples/*.stderr` | `check_examples` | The standard error of an example that writes there. |
| `tests/golden/examples/*.stdin` | `check_examples` | The standard input of an example, which a person writes and no check rewrites. |
| `tests/golden/schemas/*.json` | `check_schema_goldens` | The JSON Schema of each document. |
| `tests/golden/opt-outs.txt`, `opt-outs.json` | `check_opt_outs` | The report of opt-outs, budgets and customisations. |
| `tests/golden/recoveries.txt` | `check_recoveries` | The recovery chains, codes and outcomes. |
| `docs/commands.md` | `check_command_reference` | The Markdown command reference. |

A check that deletes stale files counts every file of its kind in its
directory, so each of those directories holds only that check's files.
`check_opt_outs` and `check_recoveries` leave other files alone, so their
files can sit in a directory of other golden files.
[Help checks](../help/checks.md) describes the help golden files.

Why: ADR-0004, ADR-0005, ADR-0025.
