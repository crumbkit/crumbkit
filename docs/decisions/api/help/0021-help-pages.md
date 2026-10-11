# ADR-0021. Help pages

- [`parse`](../../../../crates/crumbkit/src/parse.rs) keeps clap's help template for each command and adds crumbkit's lines after the command's own `after_help` and `after_long_help`, so a tool keeps clap's layout and its own text. It changes only the commands that the typed words pass through, so crumbkit's extra work on every run grows with the depth of the command typed, and the command tree is never cloned.
- `-h` names the global flags on one line and `--help` lists them in full.
- The help of a command with a visible subcommand ends with a `Next:` line that opens one of them.
- Help and the version keep clap's colours only when standard output is a terminal.
- crumbkit finds the deepest command the caller reached by walking the typed words once before parsing. The walk misreads an option's value or a positional that equals a subcommand name, and a walk that knows how many values each option takes replaces it when that matters.
- A tool may list a group's subcommands under goal headings that it declares in a `const` [`Guide`](../../../../crates/crumbkit/src/help.rs), in place of clap's single `Commands:` list. `parse` applies the headings to the groups that the typed words pass through, and [`help::extend`](../../../../crates/crumbkit/src/help.rs) applies them to the whole tree for a tool that parses with plain clap and for tests, through the same per-group function.
- A guided command gets clap's default template with the headed list in place of clap's argument sections. A guided page therefore has no `Arguments:` section and loses the tool's own `before_help`, and shelf's golden files fail when clap's default layout changes.
- `parse` and `extend` never fail on a guide. crumbkit-test's [`check_guide`](../../../../crates/crumbkit-test/src/guide.rs) catches a guide that does not match the tree.
- A command that the guide does not name keeps clap's `Commands:` list. crumbkit's `after_help` lines appear on guided and unguided pages alike.
- A refusal's message is clap's error on one line. The message depends on clap's wording, and shelf's golden files fail when it changes.

Considered: help templates for crumbkit's own lines, crumbkit rendering whole help pages, plain text always, a second parse with errors ignored, rebuilding the message from clap's error parts, applying the whole guide before every parse, hiding a group's children, the list as literal text in the template, a runtime builder holding `Vec`s.
