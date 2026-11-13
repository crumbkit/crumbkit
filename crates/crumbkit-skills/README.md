# crumbkit-skills

crumbkit-skills gives a tool built with crumbkit the commands that install
its agent skills. The tool embeds each `SKILL.md`, and `skills install`
writes those files into the skills directories that coding agents read, so
the skills an agent reads always match the installed binary.

## Adding it

A tool adds crumbkit-skills beside crumbkit:

```toml
[dependencies]
crumbkit-skills = "0.4"
```

## Example

A tool describes each skill with [`Skill`], embedding the text with
`include_str!`. The first argument is the tool's name, and the skill's name
is the `name` of its frontmatter:

```rust
use crumbkit_skills::Skill;

const SKILLS: &[Skill] = &[
    Skill::new(
        "shelf",
        env!("CARGO_PKG_VERSION"),
        "---\nname: shelf\ndescription: Browse the catalogue.\n---\n\nRun `shelf books list`.\n",
    ),
    Skill::new(
        "shelf",
        env!("CARGO_PKG_VERSION"),
        "---\nname: shelf-lending\ndescription: Lend a book.\n---\n\nRun `shelf loans add`.\n",
    ),
];

assert_eq!(SKILLS[1].name(), "shelf-lending");
let printed = crumbkit_skills::print(&SKILLS[1]);
assert!(printed.value().to_string().contains("shelf loans add"));
```

An agent chooses a skill by its `description`, so a tool gives each job
that an agent loads at a different moment a skill of its own.

The tool mounts a `skills` group with one field, one match arm and three
leaves, and [`run`] does the rest:

```rust,ignore
/// Give coding agents shelf's skill.
Skills {
    #[command(subcommand)]
    command: crumbkit_skills::SkillsCommand,
},
// ...
Resource::Skills { command } => crumbkit_skills::run(run, SKILLS, &command),
// ...
crumbkit_skills::install_leaf(&["skills", "install"]),
crumbkit_skills::check_leaf(&["skills", "check"]),
crumbkit_skills::print_leaf(&["skills", "print"]),
```

[`SkillsCommand`] has three commands, and `skills print` prints the first
skill of the slice, so the tool lists its main skill first. [`run_to`]
prints to a `Sink` in place of the standard streams, for a tool's tests.

A tool that mounts the commands itself uses the functions of each:

- `skills install` writes `<dir>/<skill>/SKILL.md` for each skill into each
  of `~/.claude/skills`, `~/.agents/skills` and `$CODEX_HOME/skills` that
  exists, or into the directory that `--dir` names. It takes
  [`SkillsInstall`], and [`install`] returns one [`InstallRow`] for each
  skill in each directory, for the document [`SKILLS_INSTALL`].
- `skills check` takes [`SkillsCheck`], and [`check`] returns one
  [`CheckRow`] for each skill in each directory, for [`SKILLS_CHECK`]. The
  command exits with 1 when a copy is missing or differs.
- `skills print` prints one skill. [`print`] returns a [`SkillText`] for
  [`SKILLS_PRINT`]. A tool whose skills print by name gives its own
  `skills print` command an argument, such as the skill's name, and passes
  the skill that it names.

A skill that reads other files beside its `SKILL.md` names them with
[`Skill::files`]. `install` writes them into the skill's folder before
`SKILL.md`, `check` compares each one, and `print` names them.

shelf's [`cli.rs`] and [`lib.rs`] show the mount.

## Where to read more

- [`install`] describes the directories, symbolic links and the
  frontmatter lines that mark the tool's own copy, and [`check`] what
  counts as current.
- The [schemas] describe the `<tool>/skills-install/v1`,
  `<tool>/skills-check/v1` and `<tool>/skills-print/v1` documents.
- [ADR-0192], [ADR-0199], [ADR-0201] and [ADR-0216] record why they work this way.

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.
