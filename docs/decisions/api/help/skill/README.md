# ADR-0194. Skill draft

- Date: 2026-11-10
- Status: amended by ADR-0199

## Context

A tool's SKILL.md mixes judgement that only its author can write with
facts that the tool already knows: its about text, the jobs that its help's
goal headings name, and the commands that callers use most. The facts drift
when the tool changes, and nothing tells the author which commands callers
need.

## Decision

- `cargo crumbkit skill draft --bin <NAME>` builds the binary as `weigh`
  does, reads the tree that `skill check` reads, and rewrites only the
  lines between a pair of markers, leaving every other byte of the file
  alone. A section named `jobs` starts with a line that holds only
  `<!-- crumbkit:generated jobs -->` and ends with
  `<!-- /crumbkit:generated jobs -->`.
- In YAML frontmatter the markers are YAML comments, such as
  `# crumbkit:generated description`, because an HTML comment there would
  be a value. The YAML form counts only inside the frontmatter and the HTML
  form only outside it, because `# ...` in the body is a heading. A marker
  inside a fenced code block is an example and does not count.
- The generated body takes the line ending of its start marker, so a CRLF
  file stays CRLF.
- The draft writes three sections: `description`, the frontmatter line
  `description: "<about>"` from the root's about text as a JSON string;
  `jobs`, a table of each goal heading's commands with their about text;
  and `usage`, the most-used commands.
- A section without markers is left out and reported as `unmarked`, so an
  author keeps a hand-written description by deleting its two marker
  lines. A missing file gets a skeleton with frontmatter, a comment where
  the hand-written text goes, and the markers of all three sections.
- `--usage <DIR>` reads every `*.jsonl` file under the directory and its
  subdirectories as `self runs export --json` lines, each span id once,
  because exports from several machines may repeat a run. A line that is
  not an exported record is counted as unreadable, and a directory that
  does not exist fails with `crumbkit::usage::unreadable`.
- A command's score is runs × (1 + share of wasted runs), which equals runs
  plus wasted runs. A run is wasted when it exited non-zero, or when it is
  a retry: the run before it in its session, ordered by `at`, ran the same
  command and failed.
- Help pages and runs of the root are not ranked and do not count as the
  run before another, and only leaf commands are ranked. Runs that would
  be ranked but name another tool or a command that the tree no longer has
  are ignored and counted. `--top` caps the section at 10 commands by
  default.
- Each most-used entry is one code span with the command and up to three of
  its most common flags in help notation, such as
  `shelf books list [--limit N] [--all]`, so `skill check` checks it, and
  the command's about text.
- Without usage, or with a usage directory that holds no ranked command,
  the usage section keeps its text and is reported as `kept`. A draft in CI
  without the exports then does not erase the section that a draft with
  them wrote.
- `--check` writes nothing and reports each section that a rerun would
  change as `would-change`, a problem row, so the run exits with 1. The
  result is the `crumbkit/skill-draft/v1` listing of one row per section.

## Consequences

`docs/behaviour/help/skill-draft/README.md` describes the command. Chains
of commands, which phase 2 mines from the same exports, add a section with
its own markers. This directory holds the records of the commands that
draft and check a tool's agent skill.

Considered: treating a missing usage directory as no usage, which hides a mistyped path; a hand-written description kept by a frontmatter flag, which adds a second rule beside the markers; markers inside the YAML value; dropping the usage section without `--usage`, which makes `--check` in CI fail after every draft with usage; ranking by runs alone, which hides the commands that callers fight with; counting a retry as any later run of the same command in the session; ranking group commands and the root, whose runs are mostly refusals and help.

## Children

- [ADR-0193. Skill check](0193-skill-check.md)
- [ADR-0199. A skill's other files, and cargo crumbkit's own skill](0199-skill-files.md)
