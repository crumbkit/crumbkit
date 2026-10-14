# ADR-0021. Help pages

- Date: 2026-10-09
- Status: accepted

## Context

Agents learn a tool from its help, so a help page must show what to run
next, what a command changes and which commands belong together. Tools
already have clap help pages with their own text and layout, which
adoption should not replace, and every run pays for whatever crumbkit
does to the tree before parsing.

## Decision

- `parse` kept clap's template and added crumbkit's lines after the
  command's own help text, so a tool keeps clap's layout and its text. It
  changes only the commands that the typed words pass through, so the
  extra work grows with the depth of the command typed and the tree is
  never cloned.
- crumbkit's lines named the global flags a leaf offers, its effects and
  write guards, and a `Next:` line that opens a subcommand.
- One walk of the typed words found the deepest command reached and
  `--json`, reading each option's values from clap's `Arg`, so an option
  value that equals a subcommand name or `--json` is not mistaken for
  one. A second parse with clap's `ignore_errors` was rejected, because it
  costs a full parse on every refusal and relies on loosely specified
  behaviour. `command_path` made the walk public for tools and checks.
- A tool could not name its own output flags for the walk, because
  `Output::is_json` would then answer differently on the parsed path.
- A `const` `Guide` could list a group's subcommands under goal headings,
  written into clap's default template so the tool's own `before_help`
  still prints. Commands the guide does not name keep clap's list.
- `RuntimeLeaf`s gave commands added at run time their examples, joined
  into a guide that every reader takes as it takes the `const` one, so no
  function needs a second guide parameter.
- `parse` never fails on a guide, and `check_guide` catches a guide that
  does not match the tree.
- A refusal's message is clap's error on one line, and shelf's golden
  files fail when clap's wording or layout changes.

## Consequences

`docs/behaviour/help/README.md` describes the current help pages.

Considered: a scan for the literal word `--json`, output flags that the tool names, help templates for crumbkit's own lines, crumbkit rendering whole help pages, plain text always, a second parse with errors ignored, rebuilding the message from clap's error parts, applying the whole guide before every parse, hiding a group's children, the list in `before_help` on every guided command, a check that fails on a brace in a listed description, a runtime builder holding `Vec`s as the only guide, a `Guide` with a lifetime that borrows a runtime guide, a second guide parameter on every function.
