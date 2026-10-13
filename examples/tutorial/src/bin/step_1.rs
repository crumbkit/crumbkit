//! Step 1 of the tutorial: the pantry tool parses with `crumbkit::parse` and
//! prints its jars with `crumbkit::finish`.

use std::process::ExitCode;

use clap::{Args, CommandFactory, Parser, Subcommand};
use crumbkit::help::Guide;
use crumbkit::{Document, Listing, Output, Page, Row, Spelling};
use schemars::JsonSchema;
use serde::Serialize;

/// Keep track of the jars in a pantry.
#[derive(Parser)]
#[command(name = "pantry")]
struct Cli {
    /// The resource the command acts on.
    #[command(subcommand)]
    resource: Resource,

    /// The flags that choose how the result prints.
    #[command(flatten)]
    output: Output,
}

/// The resources that pantry serves.
#[derive(Subcommand)]
enum Resource {
    /// Work with the jars in the pantry.
    Jars {
        /// What to do with the jars.
        #[command(subcommand)]
        command: JarsCommand,
    },
}

/// The commands for jars.
#[derive(Subcommand)]
enum JarsCommand {
    /// List the jars, in id order.
    #[command(visible_alias = "ls")]
    List(JarsList),
}

/// The arguments of `pantry jars list`.
#[derive(Args)]
struct JarsList {
    /// List only the jars on this shelf
    #[arg(long)]
    shelf: Option<String>,

    /// How many jars to print.
    #[command(flatten)]
    page: Page<20>,
}

/// One jar in the pantry.
#[derive(Serialize, JsonSchema)]
struct Jar {
    /// The jar's id.
    id: u32,
    /// What the jar holds.
    name: &'static str,
    /// The shelf the jar stands on.
    shelf: &'static str,
}

impl Row for Jar {
    const COLUMNS: &'static [&'static str] = &["id", "name", "shelf"];
}

/// The document that `pantry jars list` prints.
const JARS_LIST: Document<Listing<Jar>> = Document::new(
    "pantry",
    "jars-list",
    1,
    "One page of the jars in the pantry.",
);

/// How pantry spells a failure code.
const SPELLING: Spelling = Spelling::COLONS;

/// The jars in the pantry, in id order.
fn jars() -> Vec<Jar> {
    vec![
        Jar {
            id: 1,
            name: "rice",
            shelf: "top",
        },
        Jar {
            id: 2,
            name: "lentils",
            shelf: "top",
        },
        Jar {
            id: 3,
            name: "oats",
            shelf: "bottom",
        },
    ]
}

/// Returns one page of the jars on the shelf that `args` names.
fn list(args: &JarsList) -> Listing<Jar> {
    let jars = jars()
        .into_iter()
        .filter(|jar| args.shelf.as_deref().is_none_or(|shelf| jar.shelf == shelf))
        .collect();
    args.page.first_of(jars).described("jars")
}

fn main() -> ExitCode {
    let parsed = crumbkit::parse::<Cli>(Cli::command(), &Guide::new(), SPELLING);
    let (mut cli, command) = match parsed {
        Ok(parsed) => parsed,
        Err(stop) => return stop.code(),
    };
    cli.output.set_command(command);
    match cli.resource {
        Resource::Jars {
            command: JarsCommand::List(args),
        } => {
            let result: Result<_, crumbkit::Error> = Ok(list(&args));
            crumbkit::finish(&mut cli.output, SPELLING, JARS_LIST, result)
        }
    }
}
