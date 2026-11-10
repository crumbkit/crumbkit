# crumbkit-skills

crumbkit-skills gives a tool built with crumbkit the commands that install
its agent skill. The tool embeds its `SKILL.md`, and `skills install` writes
that file into the skills directories that coding agents read, so the skill
an agent reads always matches the installed binary.

## Adding it

A tool adds crumbkit-skills beside crumbkit:

```toml
[dependencies]
crumbkit-skills = "0.3"
```

## Example

A tool describes its skill with [`Skill`], embedding the text with
`include_str!`:

```rust
use crumbkit_skills::Skill;

const SKILL: Skill = Skill::new(
    "shelf",
    env!("CARGO_PKG_VERSION"),
    "---\nname: shelf\ndescription: Browse the catalogue.\n---\n\nRun `shelf books list`.\n",
);

let printed = crumbkit_skills::print(&SKILL);
assert!(printed.value().to_string().contains("shelf books list"));
```

The tool adds a `skills` group with three commands:

- `skills install` writes `<dir>/<tool>/SKILL.md` into each of
  `~/.claude/skills`, `~/.agents/skills` and `$CODEX_HOME/skills` that
  exists, or into the directory that `--dir` names. It takes
  [`SkillsInstall`], and [`install`] returns one [`InstallRow`] for each
  directory, for the document [`SKILLS_INSTALL`].
- `skills check` takes [`SkillsCheck`], and [`check`] returns one
  [`CheckRow`] for each directory, for [`SKILLS_CHECK`]. The command exits
  with 1 when a copy is missing or differs.
- `skills print` prints the skill. [`print`] returns a [`SkillText`] for
  [`SKILLS_PRINT`].

The tool passes each result to crumbkit's `finish` with its document.
shelf's [`cli.rs`] and [`lib.rs`] show all three.

## Where to read more

- [`install`] describes the directories, symbolic links and the
  frontmatter lines that mark the tool's own copy, and [`check`] what
  counts as current.
- The [schemas] describe the `<tool>/skills-install/v1`,
  `<tool>/skills-check/v1` and `<tool>/skills-print/v1` documents.
- [ADR-0192] records why they work this way.

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.
