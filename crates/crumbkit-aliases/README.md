# crumbkit-aliases

crumbkit-aliases gives a tool built with crumbkit aliases for its commands,
from a person's aliases file and from sources of its own. An alias can stand
for several words and can have several words as its key. The tool expands an
alias before clap parses the command line, so help and every output print
only real commands.

## Adding it

A tool adds crumbkit-aliases beside crumbkit:

```toml
[dependencies]
crumbkit-aliases = "0.4"
```

## Example

A person writes aliases in the TOML file that [`file`] names. With this
file, `shelf mine --limit 1` runs `shelf books list --long --limit 1`, and
`shelf wt ls` runs `shelf books list`:

```toml
mine = ["books", "list", "--long"]
"wt ls" = ["books", "list"]
```

A tool builds one [`Aliases`] value from [`Source`]s in precedence order.
The first source with a matching entry wins, and the longest key in a source
wins over a shorter one. [`Source::file`] reads the person's file,
[`Source::table`] holds entries in the tool's code, and [`Source::lazy`]
loads entries only when a run types a first word that is no command, so a
lazy key of several words cannot start with a command.

A tool passes its arguments through [`Aliases::expand`] before it parses
them, and adds a hidden `aliases list` command that [`Aliases::list`]
answers:

```rust
use std::process::ExitCode;
use std::sync::LazyLock;

use clap::{CommandFactory, Parser, Subcommand};
use crumbkit::Tool;
use crumbkit::ViewFlags;
use crumbkit_aliases::{Aliases, AliasesList, Source};

/// Browse a small catalogue.
#[derive(Parser)]
#[command(name = "shelf")]
struct Cli {
    #[command(subcommand)]
    resource: Resource,
    #[command(flatten)]
    view: ViewFlags,
}

#[derive(Subcommand)]
enum Resource {
    /// Work with shelf's aliases.
    #[command(hide = true)]
    Aliases {
        #[command(subcommand)]
        command: AliasesCommand,
    },
}

#[derive(Subcommand)]
enum AliasesCommand {
    /// List the built-in aliases and the aliases in shelf's aliases file.
    List(AliasesList),
}

const TOOL: Tool = Tool::new();

/// shelf's aliases: the person's file, then one shortcut of its own.
static ALIASES: LazyLock<Aliases> = LazyLock::new(|| {
    Aliases::new()
        .source(Source::file(crumbkit_aliases::file("shelf")))
        .source(Source::table("shelf", &[("wt ls", &["books", "list"])]))
});

/// Runs shelf with `args`. shelf's `main` passes `std::env::args_os()`.
fn run(args: &[&str]) -> ExitCode {
    let command = Cli::command();
    let args = ALIASES.expand(&command, args);
    // `expand` needed the tree, so the parse takes it and the run builds it once.
    let parsed = crumbkit::try_parse_to::<Cli>(&mut crumbkit::Sink::standard(), command, &TOOL, args);
    let (cli, mut run) = match parsed.map_err(crumbkit::Unparsed::print) {
        Ok(parsed) => parsed,
        Err(stop) => return stop.exit_code(),
    };
    match cli.resource {
        Resource::Aliases { command: AliasesCommand::List(args) } => {
            // `list` builds the command tree in `run`, so it runs
            // before `finish`.
            let result = Ok::<_, crumbkit::Error>(ALIASES.list(&mut run, &args));
            crumbkit::finish(run, crumbkit_aliases::ALIASES_LIST, result)
        }
    }
}

assert_eq!(run(&["shelf", "aliases", "list"]), ExitCode::SUCCESS);
```

A tool that wants aliases on its root's help passes a function that calls
`Aliases::names` to `Help::aliases` with `Scope::All`, and its tests opt out of
`Rule::PersonalAliases`, because the root's help then differs between
machines ([ADR-0027]). `Aliases::shipped_names` returns only the names of
shipped sources, which are the same on every machine. `Source::unlisted`
keeps a source out of both.

## Where to read more

- [`Aliases`], [`Source`], [`Kind`], [`Entry`], [`file`], [`AliasesList`],
  [`ALIASES_LIST`] and [`AliasRow`], the row that `aliases list` prints,
  describe each item.
- The [schema] describes the `<tool>/aliases-list/v1` document, which a
  tool prints with its own name in place of `<tool>`.
- The [aliases docs] describe the design, [ADR-0027] records why it is a
  separate crate, and [ADR-0228] records the sources.

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.

[`Aliases`]: src/aliases.rs
[`Aliases::expand`]: src/expand.rs
[`Aliases::list`]: src/list.rs
[`Source`]: src/source.rs
[`Source::file`]: src/source.rs
[`Source::table`]: src/source.rs
[`Source::lazy`]: src/source.rs
[`Kind`]: src/source.rs
[`Entry`]: src/source.rs
[`file`]: src/file.rs
[`AliasesList`]: src/list.rs
[`ALIASES_LIST`]: src/list.rs
[`AliasRow`]: src/list.rs
[schema]: schemas/aliases-list-v1.json
[ADR-0027]: ../../docs/decisions/api/0027-aliases.md
[ADR-0228]: ../../docs/decisions/api/aliases/0228-aliases-come-from-ordered-sources.md
[aliases docs]: ../../docs/behaviour/parsing/aliases.md
