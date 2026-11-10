# Skill check

This page covers `cargo crumbkit skill check`, which checks every command
line in a tool's agent skill against the tool's commands.
[cargo-crumbkit](../../../../crates/cargo-crumbkit/README.md) describes the
binary.

Why: ADR-0193.

## Running it

`cargo crumbkit skill check --bin <NAME>` builds the binary and reads its
command tree as [`weigh`](../weight.md) does, so it shares weigh's
`crumbkit::tool::*` failures. It reads `skill/SKILL.md` in the binary's
package, or the Markdown file that `--skill <FILE>` names. A file that
cannot be read fails with `crumbkit::skill::unreadable`.

## What counts as a command line

The check reads every inline code span and every non-empty line of every
fenced code block. A piece of code is a command line when its first word is
the name of the tool's root command, such as `shelf`, after an optional
`$ ` prompt. Other code, such as `git status`, is not checked. When the
root's name differs from the binary's name, the tool runs under another
program, as cargo-crumbkit's root is `cargo`. A command line then also needs
a second word that names a subcommand of the root, so `cargo crumbkit weigh`
is checked and `cargo test` is not.

The check splits a command line into words as a POSIX shell does: single
and double quotes and backslashes join words, and an unquoted `|`, `&`,
`;`, a `#` that starts a word, or a redirection such as `> out.txt` or
`2>/dev/null` ends the command. A word that starts with `<` and a letter
runs to the next `>`, so `<file name>` is one word, and so does a `<`
after `=`, as in `--format=<FORMAT>`. An unclosed quote
fails the command line.

The reader skips indented code blocks and code spans that run over a line
break.

## What the check accepts

The words walk down the tree from the root:

- A word that names a subcommand of the current command moves to it.
- A flag must be one that the current command declares, by its long or
  short name, or a global flag of a command above it. `-h`, `--help`, `-V`
  and `--version` are always accepted. Short flags may be grouped, as in
  `-qn 5`, and a value may follow `=`, as in `--limit=5`.
- A flag that takes a value must get one. The next word is its value
  unless it is `--` or another long flag.
- A value must be one of the argument's possible values when the argument
  lists them. For an argument that takes many values, each part between
  commas must be one.
- Any other word fills the next positional argument. A word left over when
  the command has no positional argument for it fails.
- `--` ends the check, and the words after it are not checked.

Placeholders and help notation follow these rules:

- A word in capitals, such as `NAME` or `TASK_ID`, or in angle brackets,
  such as `<name>`, counts as any value.
- `...`, `[OPTIONS]` and an optional placeholder such as `[ID]` count as
  nothing. `ID...` counts as `ID`, and the brackets of `[--limit N]` are
  dropped, so `--limit N` is checked.
- A placeholder or the word `help` where a subcommand belongs, as in
  `shelf COMMAND -h` or `shelf help books`, ends the check.

A missing required argument passes, because a skill names a command in
prose without its arguments. The check also accepts a line that clap would
refuse for a rule across arguments, such as two conflicting flags, or for
a flag given twice. It refuses a command alias, a bare flag whose value is
optional, and several words after a flag that takes many values, because
`commands list` does not say which commands have aliases or how many values
an argument takes.

## Output

The result is the `crumbkit/skill-check/v1` document, which
[the schema](../../../../crates/cargo-crumbkit/schemas/skill-check-v1.json)
describes. It holds one row for each command line, with the file, the line
number, the command line and the reason it fails, or `null` when it
passes. Failing rows come first, each group in file order. The summary
counts the command lines and the failures.

The default text prints the summary and one line for each failing row:

```text
Checked 5 shelf command lines in skill/SKILL.md, and 3 failed.
skill/SKILL.md:6: shelf books lsit: `lsit` is not a command of `shelf books`
skill/SKILL.md:7: shelf books list --limit: `--limit` needs a value
skill/SKILL.md:8: shelf books list --author Austen: `shelf books list` has no flag `--author`
```

`--long`, `--fields` and `--json` list every row. The run exits with 1 when
any row fails, so CI can run the check.
