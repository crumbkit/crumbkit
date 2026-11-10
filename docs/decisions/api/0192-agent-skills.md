# ADR-0192. Agent skills

- Date: 2026-11-10
- Status: accepted

## Context

A coding agent learns a tool from a `SKILL.md` in its skills directory.
A skill copied there by hand drifts from the installed binary, and no
crumbkit tool had a shared way to install one or to tell that a copy is
stale.

## Decision

- The crumbkit-skills crate gives a tool `skills install`, `skills check`
  and `skills print`. The tool passes a `Skill` that holds its name, its
  version and its `SKILL.md` text, which it embeds with `include_str!`, so
  the skill always ships with the binary.
- Each command follows the shape of `self runs list`: a clap arguments
  type, and a function whose result the tool passes to `finish` with the
  command's document. The functions take the `Skill` in place of `Run`
  and the tool record, because they read neither. The arguments types
  derive no `Serialize` or `JsonSchema`, because nothing reads them.
- The verb is `print`. crumbkit-test's listing check requires every
  `show` to take ids, and printing the skill takes none.
- `install` writes `<dir>/<tool>/SKILL.md` into each of `~/.claude/skills`,
  `~/.agents/skills` and `$CODEX_HOME/skills` that exists, with
  `CODEX_HOME` defaulting to `~/.codex`, or into the one directory that
  `--dir` names. The home directory comes only from `HOME`, or
  `USERPROFILE` on Windows, never from `std::env::home_dir`, which reads
  the password database and would install into the real home during a
  test or an example run that clears the environment (ADR-0027). With no
  home directory, the commands act on nothing and their note says so.
- Directories that resolve to one real path share one write and one row
  that names each path. When `<dir>/<tool>` is a symbolic link, `install`
  follows it, updates the folder it leads to when that folder holds a copy
  that this tool's `install` wrote, and otherwise skips it with the reason
  `linked to <path>`.
- `install` marks its copy with `tool` and `version` keys under
  `metadata:` in the frontmatter, with quoted values, so the YAML stays
  valid for any version, including build metadata, and agents still read
  `name` and `description`. `Skill::new` refuses at compile time a text
  without frontmatter or with its own `metadata:` key. `install` never
  replaces a file, a folder link or a `SKILL.md` that it did not write
  unless `--force` is given, so its effects are additive and idempotent,
  and `--force` is the confirmation that a `--yes` would otherwise give.
- `install` writes through a temporary file in the same folder and a
  rename, so an agent never reads half a file.
- An I/O error in one directory becomes a `failed` row, the only row that
  makes `install` exit with 1, so the crate declares no failure codes. A
  skipped row does not fail the run, because its owner chose that file.
- `check` lists only the rows whose copy is missing, differs or cannot be
  read, first, and exits with 1 when there is one. A copy is current only
  when it is exactly what this binary's `install` writes, so a copy from
  another version differs.
- `print` prints the text as the tool embeds it, without the metadata
  lines, so its output is the same in every build of one version, and its
  document always prints the first answer whole.

## Consequences

The plan named a `SkillsCommand` type and a `show` verb; the crate has
three functions that the tool mounts in its own subcommand enum, as
crumbkit-usage's commands are mounted, and the verb `print`. Each
crate's rustdoc describes the current behaviour.

Considered: one `SkillsCommand` enum with one function that runs each of its commands, the verb `show` with an opt-out from the listing check, `std::env::home_dir` or etcetera for the home directory, writing to every agent directory whether it exists or not, a sidecar file or a hash to mark the tool's copy, a YAML parser, a `--yes` flag in place of `--force`, failure codes for I/O errors, `print` with the metadata lines.
