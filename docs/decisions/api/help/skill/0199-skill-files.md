# ADR-0199. A skill's other files, and cargo crumbkit's own skill

- Date: 2026-11-11
- Status: accepted
- Supersedes: ADR-0192, ADR-0193 and ADR-0194, which it amends
- Missed: ADR-0192 let a skill hold only its `SKILL.md`, although a skill can read other files beside it, such as a reference table. ADR-0193 and ADR-0194 named cargo crumbkit's group `skill`, which reads as the same group as the `skills` group that crumbkit-skills gives a tool.

## Context

The audit that finds places where a Rust tool should use crumbkit, and
does not, existed only as an agent skill outside the repository. It read a
table of crumbkit's affordances, which drifted from each release, and it
filed its findings in one adopter's own task records, so no other tool
author could run it.

The audit needs that table beside its `SKILL.md`. crumbkit-skills embedded
one `SKILL.md` per tool, so a skill with a second file could not ship with
the binary. Another tool that adopts crumbkit-skills ships a `README.md`
beside each of its skills and needs the same.

## Decision

- `Skill::files` names a skill's other files, each as a file name and the
  text that the tool embeds with `include_str!`. `Skill::new` keeps its
  signature, so a tool without other files does not change, and
  `Skill::get_files` returns them.
- A file name is ASCII letters, digits, dots, dashes and underscores that
  does not start with a dot, and is never `SKILL.md`, so every file sits in
  the skill's own folder. A name given twice is refused at compile time.
  Names are compared ignoring ASCII case, because a file system that
  ignores case, as macOS and Windows do by default, holds `skill.md` and
  `SKILL.md` as one file.
- The other files belong to `install` when the folder's `SKILL.md` does,
  because they carry no frontmatter that could mark them. A folder without
  a `SKILL.md` that holds a file at one of their names with other text is
  skipped before anything is written there, and `--force` replaces that
  file. A file that already holds the embedded text counts as install's
  own, so the next install finishes a copy that an interrupted one left.
- `install` writes the other files first and `SKILL.md` last, so an agent
  never loads a skill whose files are missing. A row is `unchanged` only
  when every file already matches.
- `check` reports a current `SKILL.md` whose folder lacks one of the other
  files as `missing`, and one whose other file holds other text as
  `differs`, with the reason naming the file.
- `print` prints `SKILL.md` alone, names the other files in a note, and
  lists them under `files` in its JSON document.
- cargo crumbkit mounts crumbkit-skills as `cargo crumbkit skills install`,
  `check` and `print`. Its skill, `cargo-crumbkit`, says what each command
  is for and holds the audit as its "Audit a tool" job, with the
  affordance table and the JSON schema of the audit's report as its other
  files. The audit writes that report in place of any one tracker's
  records, and its last step tells the agent to write the findings with
  the same fields wherever its own instructions name.
- The skill ships with the cargo subcommand, because cargo crumbkit is
  already the one binary a crumbkit tool's author installs, and its
  `tool-skill check` checks the skill's command lines in a test.
- cargo crumbkit runs a binary named `cargo-<name>` with `<name>` as its
  first word, as cargo does, so `weigh`, `docs` and `tool-skill check` read
  the commands of a cargo subcommand, cargo crumbkit's own included.
- cargo crumbkit's `skill draft` and `skill check`, which act on another
  tool's skill, become `tool-skill draft` and `tool-skill check`, so
  `skills` always means cargo crumbkit's own skill. The old names have no
  alias, because crumbkit has no outside users yet. Failure codes keep the
  area `skill`, because a code's area is a domain noun and never changes
  with a command's name (ADR-0001).

## Consequences

`install` leaves a file that an earlier version of a skill wrote and the
current version no longer names, because nothing records which files a
copy holds. Each crate's rustdoc describes the current behaviour.

Considered: a `crumbkit` binary of its own for the audit, a folder tree of
files with subfolders, a marker in each other file, a list of files in the
frontmatter, failing `check` on an unknown file in the folder, keeping the
audit's findings in one tracker's records, an alias from `skill` to
`tool-skill`, renaming crumbkit-skills' `skills` group in cargo crumbkit.
