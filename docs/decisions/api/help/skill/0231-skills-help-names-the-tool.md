# ADR-0231. Skills help names the tool

- Date: 2026-11-16
- Status: accepted
- Supersedes: ADR-0216, which it amends
- Missed: ADR-0216 chose generic about lines and example descriptions for the shared `skills` commands, and did not weigh that a reader of `shelf skills -h` then cannot tell from the text whose skills the commands manage.

## Context

Before ADR-0216 each tool wrote its own `skills` enum and leaves, so its help read "Install shelf's skill into each agent skills directory that exists" and "Print cargo crumbkit's skill". ADR-0216 moved the enum and the leaves into crumbkit-skills, where one `SkillsCommand` enum and three constant example lists serve every tool. Their text said "this tool's skills" and "the skill", and the goldens of shelf and cargo-crumbkit show that text.

Clap reads an about line from the enum when the derive runs, and the leaf examples are `&'static str` constants, so a function that takes the tool's name at run time cannot reach either. A `const fn` cannot join a name to a sentence.

## Decision

- This record replaces the clause of ADR-0216 that said the about lines say "this tool's skills" because the enum is shared. Every other clause of ADR-0216 stands.
- crumbkit-skills exports the macro `skills_command!(<vis> <Name>, "<tool>")`. The tool gives its name once, as its usage line writes it, such as `"shelf"` or `"cargo crumbkit"`. The macro defines a clap `Subcommand` enum called `<Name>` with `Install`, `Check` and `Print` variants, whose about lines name the tool, and the associated `const fn`s `install_leaf`, `check_leaf` and `print_leaf`, whose example descriptions name it. It joins the name to the text with `concat!`, which works in constants.
- `SkillsCommand` becomes `SkillsAction`, an enum that has no clap derive. The enum the macro defines converts into it, and `run` and `run_to` take `impl Into<SkillsAction>` by value.
- A tool still mounts `skills` with one field, one match arm and three leaves. It adds one line, the macro call, and replaces `crumbkit_skills::install_leaf(path)` with `<Name>::install_leaf(path)` and the same for the other two leaves.
- The about line of `skills print` reads "Print <tool>'s skill", because the command prints the first skill. The other two lines say "<tool>'s skills".

## Consequences

The help pages, the `commands list` output and `docs/commands.md` of a tool name the tool again. A tool that passes a name that differs from its usage line gets help that differs from it, and the goldens of that tool show the text. crumbkit 0.5.0 shipped `SkillsCommand` and the free leaf functions, so a tool that upgrades from 0.5.0 replaces the field's type with the macro's enum, calls the leaf functions on that enum, drops the `&` in its call to `run`, and regenerates its help goldens once. This breaks those tools by design, because crumbkit's only users are its author's projects and the better API wins over compatibility.

Considered: a `tool` argument on each leaf function, which cannot reach the clap about lines and cannot join the name to an example description in a constant; a `{tool}` placeholder that the help layer replaces, which leaves the placeholder in `commands list`, in shell completion and in any command tree that is not extended; a trait that a tool implements to give its name, which costs a type and an impl for one string; an `about` that `help::extend` writes from the leaf, which changes crumbkit for one crate's text.
