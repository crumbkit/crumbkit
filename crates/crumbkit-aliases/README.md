# crumbkit-aliases

crumbkit-aliases lets a person define their own aliases for the commands of a
tool built with crumbkit. The tool expands an alias before clap parses the
command line, so help and every output print only real commands. crumbkit is
in design and is not published yet.

## Adding it

A tool adds crumbkit-aliases beside crumbkit, from this repository until the
first release:

```toml
[dependencies]
crumbkit-aliases = { git = "https://github.com/crumbkit/crumbkit" }
```

The `main` branch can change without notice before the first release.

## Example

A person writes aliases in the TOML file that [`file`] names. With this
file, `shelf mine --limit 1` runs `shelf books list --long --limit 1`:

```toml
mine = ["books", "list", "--long"]
```

A tool passes its arguments through [`expand`] before it parses them, and
adds a hidden `aliases list` command that [`list`] answers:

```rust
use std::path::Path;
use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand};
use crumbkit::help::Guide;
use crumbkit::{Spelling, ViewFlags};
use crumbkit_aliases::AliasesList;

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

const GUIDE: Guide = Guide::new();
const SPELLING: Spelling = Spelling::COLONS;

/// Runs shelf with `args` and the aliases file at `file`. shelf's `main`
/// passes `std::env::args_os()` and
/// `crumbkit_aliases::file(Cli::command().get_name()).as_deref()`.
fn run(args: &[&str], file: Option<&Path>) -> ExitCode {
    let command = Cli::command();
    let args = crumbkit_aliases::expand(&command, args, file);
    let (cli, mut output) =
        match crumbkit::parse_from::<Cli, _, _>(command, &GUIDE, SPELLING, args) {
            Ok(parsed) => parsed,
            Err(stop) => return stop.code(),
        };
    match cli.resource {
        Resource::Aliases { command: AliasesCommand::List(args) } => {
            // `list` builds the command tree in `output`, so it runs
            // before `finish`.
            let result = crumbkit_aliases::list(&mut output, file, &args);
            crumbkit::finish(&mut output, SPELLING, crumbkit_aliases::ALIASES_LIST, result)
        }
    }
}

assert_eq!(run(&["shelf", "aliases", "list"], None), ExitCode::SUCCESS);
```

A tool that wants a person's aliases on its root's help passes [`names`] to
`Guide::personal_aliases`, and its tests opt out of `Rule::PersonalAliases`,
because the root's help then differs between machines ([ADR-0027]).

## Where to read more

- [`file`], [`expand`], [`list`], [`names`], [`AliasesList`] and
  [`ALIASES_LIST`] describe each item.
- The [schema] describes the `crumbkit/aliases-list/v1` document.
- The [aliases docs] describe the design, and [ADR-0027] records why it is
  a separate crate.

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.

[`file`]: src/file.rs
[`expand`]: src/expand.rs
[`list`]: src/list.rs
[`names`]: src/names.rs
[`AliasesList`]: src/list.rs
[`ALIASES_LIST`]: src/list.rs
[schema]: schemas/aliases-list-v1.json
[ADR-0027]: ../../docs/decisions/api/0027-aliases.md
[aliases docs]: ../../docs/behaviour/parsing/aliases.md
