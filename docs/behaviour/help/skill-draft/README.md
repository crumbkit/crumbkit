# Skill draft

This page covers `cargo crumbkit skill draft`, which writes the generated
sections of a tool's agent skill from the tool's commands and, when given,
its exported usage. [cargo-crumbkit](../../../../crates/cargo-crumbkit/README.md)
describes the binary, and the [skill check](../skill-check/README.md)
checks the command lines that the draft writes.

Why: ADR-0194.

## Running it

`cargo crumbkit skill draft --bin <NAME>` builds the binary and reads its
command tree as [`weigh`](../weight.md) does, so it shares weigh's
`crumbkit::tool::*` failures. It drafts `skill/SKILL.md` in the binary's
package, or the Markdown file that `--skill <FILE>` names. A file that
exists but cannot be read fails with `crumbkit::skill::unreadable`, and a
file that cannot be written fails with `crumbkit::skill::write_failed`.

## Generated sections

The draft rewrites only the lines between the two markers of a section and
leaves every other byte of the file alone, line endings included. A section
named `jobs` looks like this:

```markdown
<!-- crumbkit:generated jobs -->
## Jobs
...
<!-- /crumbkit:generated jobs -->
```

Each marker is a line of its own, and spaces around it are allowed. In the
YAML frontmatter, where an HTML comment would be a value, the markers are
YAML comments: `# crumbkit:generated description` and
`# /crumbkit:generated description`. A start marker without its end marker
fails with `crumbkit::skill::unclosed_section`, which names the line.

The draft writes three sections:

- `description` is the frontmatter line `description: "<about>"`, with the
  about text of the tool's root command as a JSON string, which YAML reads
  as a double-quoted string.
- `jobs` is a table with one row for each command under each goal heading
  of the tool's help pages, with the command's about text.
- `usage` is the most-used commands, described below.

A section without markers is left out and reported as `unmarked`. An author
who wants to write the description by hand deletes its two marker lines.

When the file is missing, the draft writes a skeleton: frontmatter with the
tool's name and the `description` section, a heading, an HTML comment where
the author writes the hand-written text, and the `jobs` and `usage`
sections.

## Most-used commands

`--usage <DIR>` names a directory of exported usage, the JSON lines that
`<tool> self runs export --json` prints (see
[usage export](../../usage-log/export/README.md)). The draft reads every
`*.jsonl` file under the directory and its subdirectories, so a directory
laid out as `<host>/<day>.jsonl` holds the exports of several machines. A
run whose span id it has already read is skipped, and a non-empty line that
is not an exported record is counted as unreadable.

The draft ranks each command by its runs × (1 + the share of its runs that
were wasted), which equals its runs plus its wasted runs. A run is wasted
when it exited with a code other than 0, or when it is a retry: the run
before it in the same session, ordered by `at`, ran the same command and
failed. Runs that print a help page and runs of the root command are not
ranked and are not counted as the run before another, so a failure, a look
at `-h` and a second try count as a retry. Only commands without
subcommands are ranked. Equal scores rank by command path.

Runs of another tool, and runs of a command that the tool no longer has,
are ignored, and the summary counts them. `--top <N>` caps the section at
`N` commands, 10 by default.

Each entry is one code span with the command and up to three of its most
common flags in help notation, followed by the command's about text:

```markdown
- `shelf books list [--limit N] [--all] [--json]`: List the books in the catalogue, in id order
```

A flag that the command no longer accepts is left out. The skill check
reads the span, so each entry names real commands and flags.

Without `--usage`, or when the directory is missing or holds no run of a
ranked command, the `usage` section keeps its text and is reported as
`kept`. A draft in CI without the exports then leaves the section that a
draft with them wrote.

## Output

The result is the `crumbkit/skill-draft/v1` document, which
[the schema](../../../../crates/cargo-crumbkit/schemas/skill-draft-v1.json)
describes. It holds one row for each section, with its name and its status:
`written`, `unchanged`, `would-change`, `kept` or `unmarked`. The summary
names the file and the tool, and counts the runs that the usage holds, the
runs it ignored and the unreadable lines.

```text
Drafted skill/SKILL.md for shelf from 18 runs, and ignored 2 that name a command shelf no longer has or another tool.
SECTION      STATUS
description  written
jobs         written
usage        written
```

`--check` writes nothing. Each section that a draft would change is
reported as `would-change` and listed first, and the run exits with 1, so
CI can run `cargo crumbkit skill draft --check` beside
`cargo crumbkit skill check`.

## Tests

Unit tests rank hand-built runs, rewrite sections between markers, and
draft a file from a fixed command tree. End-to-end tests build shelf, draft
a skill file from the exported usage in
`crates/cargo-crumbkit/tests/fixtures/shelf-usage-export`, compare it with
`shelf-skill-drafted.md`, keep the hand-written text of a stale file, and
run `skill check` on the drafted file.

Why: ADR-0194.
