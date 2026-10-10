# crumbkit-aliases

crumbkit-aliases lets a person define their own aliases for the commands of a
tool built with crumbkit. An agent never sees an alias: help, `commands list`,
`Next:` lines, recoveries and JSON output all print the real command, because
the tool expands an alias before clap parses the command line.

crumbkit is in design and is not published yet. This crate has three
functions, the arguments and row types of one command, and its document.

- `file(tool)` returns the path of the tool's aliases file:
  `$XDG_CONFIG_HOME/<tool>/aliases.toml`, or
  `$HOME/.config/<tool>/aliases.toml` when `XDG_CONFIG_HOME` is unset, empty
  or relative. `<tool>` is the name of the root command. It returns `None`
  when neither variable is an absolute path.
- `expand(&command, args, file)` returns the process arguments with a
  personal alias in the first word replaced by its words, for the tool to
  pass to `crumbkit::parse_from`. Only the first word after the tool's name
  can be an alias, and it expands once. A first word that is `help`, a
  subcommand of the root or one of its aliases never expands, and `expand`
  does not read the file for it, or for a flag or an empty command line.
  `expand` never prints and never fails. A missing or broken file, an unknown
  word or a bad entry leaves the arguments unchanged.
- `list(&output, file, &args)` returns the rows of `<tool> aliases list` for
  `crumbkit::finish` with the document `ALIASES`, whose id is
  `crumbkit/aliases/v1`. The rows are the built-in aliases first, which are
  the clap aliases of the visible commands named by their full path such as
  `books ls`, and then the file's entries in name order. Each row has the
  columns `NAME`, `COMMAND`, `PROBLEM` and `FROM`, and an entry that does not
  expand says why under `PROBLEM`. In text output, a note after the table
  names the aliases file and says when it does not exist yet, or says why
  the tool has no aliases file. A file that exists but cannot be read or
  parsed fails with the code `crumbkit::aliases::unreadable`. The schema of
  the JSON document is `schemas/aliases-v1.json`.
- `AliasesList` is the arguments type of `aliases list`: crumbkit's paging
  flags.

The aliases file is TOML. Each key is an alias, and each value is a list of
one or more words:

```toml
mine = ["books", "list", "--author", "4"]
```

With this file, `shelf mine --limit 1` runs
`shelf books list --author 4 --limit 1`. A real command always wins, so an
entry named `books` never expands.

A tool gives a command a built-in alias with clap's hidden `alias`, such as
`#[command(alias = "ls")]`, which crumbkit-test's `check_names` accepts. A
tool adopts personal aliases in the function that its `main` calls, and adds
a hidden `aliases` group with a `list` command:

```rust
use std::path::Path;
use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand};
use crumbkit::help::Guide;
use crumbkit::{Output, Spelling};
use crumbkit_aliases::AliasesList;

/// Browse a small catalogue.
#[derive(Parser)]
#[command(name = "shelf")]
struct Cli {
    #[command(subcommand)]
    resource: Resource,
    #[command(flatten)]
    output: Output,
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
    let (mut cli, command) =
        match crumbkit::parse_from::<Cli, _, _>(command, &GUIDE, SPELLING, args) {
            Ok(parsed) => parsed,
            Err(code) => return code,
        };
    cli.output.set_command(command);
    match cli.resource {
        Resource::Aliases { command: AliasesCommand::List(args) } => crumbkit::finish(
            &cli.output,
            SPELLING,
            crumbkit_aliases::ALIASES,
            crumbkit_aliases::list(&cli.output, file, &args),
        ),
    }
}

assert_eq!(run(&["shelf", "aliases", "list"], None), ExitCode::SUCCESS);
```

The crate is separate from crumbkit because only a tool with personal aliases
needs a TOML parser, and crumbkit's default build depends only on clap,
serde, `serde_json` and schemars. ADR-0027 records the design.
