//! Step 2 of the tutorial: the pantry tool fails with a stable code and a
//! recovery when no jar stands on the shelf the caller named.

use std::fmt;
use std::process::ExitCode;

use clap::{Args, CommandFactory, Parser, Subcommand};
use crumbkit::help::Guide;
use crumbkit::{Call, Code, Document, Failure, Listing, Page, Recovery, Row, ViewFlags};
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
    view: ViewFlags,
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

/// The code of a shelf that holds no jar.
const SHELF_NOT_FOUND: Code = Code::new("pantry", "shelves", "not_found");

/// No jar stands on the shelf that the caller named.
#[derive(Debug)]
struct ShelfNotFound(String);

impl fmt::Display for ShelfNotFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "no jar stands on the shelf {}", self.0)
    }
}

impl std::error::Error for ShelfNotFound {}

impl Failure for ShelfNotFound {
    fn code(&self) -> &'static Code {
        &SHELF_NOT_FOUND
    }

    fn recovery(&self) -> Recovery {
        Recovery::Run(Call::new(&["jars", "list"]))
    }
}

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
fn list(args: &JarsList) -> Result<Listing<Jar>, ShelfNotFound> {
    let jars: Vec<Jar> = jars()
        .into_iter()
        .filter(|jar| args.shelf.as_deref().is_none_or(|shelf| jar.shelf == shelf))
        .collect();
    if let (Some(shelf), true) = (&args.shelf, jars.is_empty()) {
        return Err(ShelfNotFound(shelf.clone()));
    }
    Ok(args.page.first_of(jars).described("jars"))
}

fn main() -> ExitCode {
    let parsed = crumbkit::parse::<Cli>(Cli::command(), &Guide::new());
    let (cli, output) = match parsed {
        Ok(parsed) => parsed,
        Err(stop) => return stop.exit_code(),
    };
    match cli.resource {
        Resource::Jars {
            command: JarsCommand::List(args),
        } => crumbkit::finish(output, JARS_LIST, list(&args)),
    }
}
