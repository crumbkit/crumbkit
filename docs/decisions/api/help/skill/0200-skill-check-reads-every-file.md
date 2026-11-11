# ADR-0200. Skill check reads every Markdown file of a skill

- Date: 2026-11-11
- Status: accepted
- Supersedes: ADR-0193, which it amends
- Missed: ADR-0193 had the check read one file, `skill/SKILL.md` or the file that `--skill` names. A skill can hold other Markdown files beside its `SKILL.md` (ADR-0199), and a tool can ship several skills, so the command lines in those files went unchecked unless CI ran the check once per file.

## Context

`Skill::files` lets a skill ship files beside its `SKILL.md`, such as a
reference table, and those files name commands in code as `SKILL.md` does.
A tool with several skills keeps each in a folder of its own. The check
read one file per run, so a renamed command stayed wrong in every file that
CI did not name.

## Decision

- With no `--skill`, `cargo crumbkit tool-skill check` reads every Markdown
  file under `skill` in the binary's package, subfolders included, in path
  order. A Markdown file is a file whose extension is `md` in any ASCII
  case. The walk does not follow a symbolic link to a folder, so a link
  that loops cannot hang the check. It reads a link to a Markdown file.
- `--skill <PATH>` names a folder, which is read the same way, or one file,
  which is checked alone as before.
- A folder that holds no Markdown file fails with
  `crumbkit::skill::unreadable` naming `SKILL.md` in that folder. A file or
  folder that cannot be read fails with the same code and names that path,
  and no row prints. A path that does not exist fails this way, so a
  package with no `skill` folder fails naming `skill` in the package.
- The document stays `crumbkit/tool-skill-check/v1` with the same fields.
  Each row already names its file. Failing rows come first, and each group
  is in path order and then line order. The summary's `file` holds the path
  that was checked, which is the file or the folder.

## Consequences

One run in CI covers every file of every skill in the folder, and a new
file is checked without a change to CI. A Markdown file under `skill` that
is not part of a skill is checked too, and its code that starts with the
tool's name must name real commands.

Considered: reading the files that the tool's `Skill` record names, which
needs the tool to print its skills and misses a file that the record
forgot; repeating `--skill`, which CI must extend for each new file; a
`files` count in the summary, which adds a field for a figure that the rows
already show; a new document version, which the unchanged fields do not
need.
