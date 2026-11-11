# ADR-0201. A tool ships several agent skills

- Date: 2026-11-11
- Status: accepted
- Supersedes: ADR-0199, which it amends
- Missed: ADR-0199 kept one skill per tool, named as the tool, although a tool can need skills that an agent loads at different moments.

## Context

crumbkit-skills embedded one skill per tool, and the skill took the tool's
name. A tool that adopts crumbkit-skills can have two jobs that an agent
should load at different moments, such as an everyday skill and a long
one for an occasional job. An agent chooses a skill by its `description`,
so each job needs a skill with its own name and description.

`install`, `check` and `print` took one `Skill`, and their rows had no
column that names a skill. A second `Skill` constant would have recorded
the second skill's name as `metadata.tool`, so `install` would not have
recognised the copies of one tool as its own.

## Decision

- A skill's name is the `name` of the frontmatter of its `SKILL.md`.
  `Skill::new(tool, version, text)` keeps its three arguments, and its
  first argument is the tool's name. It reads the name from the `name:`
  line at compile time, so the folder that `install` writes always has the
  name that an agent reads in the file. The value may be plain or in
  double quotes. A text without a `name:` line, or with a name that is not
  lowercase ASCII letters, digits and dashes starting with a letter, is
  refused at compile time.
- `Skill::tool` returns the tool's name and `Skill::name` the skill's.
  `install` writes `<dir>/<skill name>/SKILL.md`. The `metadata:` lines of
  an installed copy stay `tool` and `version`, and `tool` is the tool's
  name whatever the skill is called, so `install` recognises every skill
  of its own tool. Reasons such as `written by shelf 0.2.0` name the tool.
- `install` and `check` take a slice of skills and return one listing with
  one row for each skill in each skills directory. Directories that
  resolve to one place still share a row, for each skill. The rows have a
  `skill` field, which is a compact column after `state`. Rows that report
  a problem still come first, and the other rows keep the order of the
  slice.
- Each skill's folder is planned and written on its own, as one skill's
  folder was before. A skill that is skipped or fails leaves the tool's
  other skills as they are, and the next `install` finishes it.
- A `--dir` that is not a directory gives one row for each skill.
- `print` takes one skill, and its document gains `skill` beside `tool`. A
  tool with several skills gives its `skills print` command an argument
  of its own that chooses the skill.
- The names in a slice differ. crumbkit-skills does not check this, and
  two skills with one name would write one folder twice.

## Consequences

A tool with one skill passes a slice of one, and its documents gain the
`skill` field, which holds the name that the tool's `SKILL.md` already
gave. The reader of `name:` is no YAML parser: it reads one `name:` line
at the start of a line, and a name written any other way is refused when
the tool is built. The installed folder is named by the frontmatter
`name:`, so a text without a `name:` line no longer compiles, and a tool
whose frontmatter name differs from its tool name installs into
`<dir>/<name>` and leaves the old `<dir>/<tool>` folder for the person to
remove. Each crate's rustdoc describes the current behaviour.

Considered: a `named` builder method that sets the skill's name beside the tool's, which lets the folder's name differ from the name in the file; a `Skills` set type that holds the tool's name once, which adds a type for what a slice already says; a name lookup inside `print`, which adds a failure to crumbkit-skills for a choice that the tool's own argument makes.
