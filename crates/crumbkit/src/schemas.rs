//! The schema commands: `<tool> schemas list`, which lists the JSON
//! documents that a tool prints, and `<tool> schemas show <ID>...`, which
//! prints the JSON Schema of each, so an agent learns the shape of a
//! document from the tool itself (ADR-0035).
//!
//! The documents come from the tool's [`Guide`]: each leaf that declares
//! the document it prints with [`Leaf::document`](crate::help::Leaf::document)
//! adds that document, and crumbkit adds `<tool>/failure/v1`, which every
//! command prints when it fails with `--json`. Each
//! [`FileFormat`](crate::help::FileFormat) that the guide declares with
//! [`Guide::documents`] adds the schema of a file that users write and no
//! command prints, after the leaves' documents.
//!
//! The tool owns the commands. It adds a `schemas` resource with a `list`
//! subcommand that takes [`SchemasList`] and a `show` subcommand that takes
//! [`SchemasShow`] to its own clap types, at those paths below the root
//! (ADR-0015), and its handlers pass the results of [`list`] and [`show`]
//! to [`finish`](crate::finish) with the documents [`SCHEMAS_LIST`] and
//! [`SCHEMAS_SHOW`]:
//!
//! ```
//! use std::process::ExitCode;
//!
//! use clap::{CommandFactory, Parser, Subcommand};
//! use crumbkit::help::{Guide, Leaf};
//! use crumbkit::schemas::{self, SCHEMAS_LIST, SCHEMAS_SHOW, SchemasList, SchemasShow};
//! use crumbkit::ViewFlags;
//!
//! /// Browse a small catalogue.
//! #[derive(Parser)]
//! #[command(name = "shelf")]
//! struct Cli {
//!     #[command(subcommand)]
//!     resource: Resource,
//!     #[command(flatten)]
//!     view: ViewFlags,
//! }
//!
//! #[derive(Subcommand)]
//! enum Resource {
//!     /// Explore the JSON documents that shelf prints.
//!     Schemas {
//!         #[command(subcommand)]
//!         command: SchemasCommand,
//!     },
//! }
//!
//! #[derive(Subcommand)]
//! enum SchemasCommand {
//!     /// List the JSON documents that shelf prints.
//!     List(SchemasList),
//!     /// Print the JSON Schema of each document.
//!     Show(SchemasShow),
//! }
//!
//! const GUIDE: Guide = Guide::new().leaves(&[
//!     Leaf::new(&["schemas", "list"]).document(SCHEMAS_LIST),
//!     Leaf::new(&["schemas", "show"]).document(SCHEMAS_SHOW),
//! ]);
//!
//! fn run(args: &[&str]) -> ExitCode {
//!     let parsed = crumbkit::parse_from::<Cli, _, _>(Cli::command(), &GUIDE, args);
//!     let (cli, mut output) = match parsed {
//!         Ok(parsed) => parsed,
//!         Err(stop) => return stop.exit_code(),
//!     };
//!     match cli.resource {
//!         Resource::Schemas { command } => match command {
//!             // `list` and `show` build the command tree in `output`,
//!             // so each runs before `finish`.
//!             SchemasCommand::List(args) => {
//!                 let result = schemas::list(&mut output, &GUIDE, &args);
//!                 crumbkit::finish(output, SCHEMAS_LIST, result)
//!             }
//!             SchemasCommand::Show(args) => {
//!                 let result = schemas::show(&mut output, &GUIDE, &args);
//!                 crumbkit::finish(output, SCHEMAS_SHOW, result)
//!             }
//!         },
//!     }
//! }
//!
//! assert_eq!(run(&["shelf", "schemas", "list"]), ExitCode::SUCCESS);
//! assert_eq!(
//!     run(&["shelf", "schemas", "show", "shelf/failure/v1"]),
//!     ExitCode::SUCCESS
//! );
//! ```

mod columns;
mod envelope;
mod row;
mod served;

use std::fmt;

use clap::Args;
use schemars::generate::SchemaSettings;

pub use self::envelope::document_schema;
#[cfg(test)]
pub(crate) use self::envelope::stream_schema;
pub(crate) use self::envelope::{envelope, file_format};
pub use self::row::{SchemaRow, SchemaShowRow};
pub use self::served::{Served, documents};
use crate::help::Guide;
use crate::{Batch, Call, Code, Document, Error, Failure, Listing, Output, Page, Recovery, codes};

#[cfg(test)]
mod tests;

/// Returns the settings of every JSON Schema that describes a document the
/// tool prints: draft 2020-12, with the contract of what serde serialises
/// (ADR-0110).
///
/// A schema built with these settings describes the fields that the
/// document holds. A field with `skip_serializing_if` is optional, because
/// the document leaves it out when it is empty, and a field without it is
/// required, whether or not it has a serde default. schemars describes what
/// deserialises unless its settings say otherwise, so a schema of a file
/// that the tool reads keeps schemars' default settings.
pub(crate) fn settings() -> SchemaSettings {
    SchemaSettings::draft2020_12().for_serialize()
}

/// The document that `schemas list` prints: one page of the documents that
/// the tool prints, and the number of them. Its schema is
/// `crates/crumbkit/schemas/schemas-list-v1.json`.
pub const SCHEMAS_LIST: Document<Listing<SchemaRow>> = Document::new(
    "crumbkit",
    "schemas-list",
    1,
    "The JSON documents that the tool prints, with a description of each.",
);

/// The document that `schemas show` prints: the JSON Schema of each id
/// given, and a failure for each id that names no document. Its schema is
/// `crates/crumbkit/schemas/schemas-show-v1.json`.
pub const SCHEMAS_SHOW: Document<Batch<SchemaShowRow, Error>> = Document::new(
    "crumbkit",
    "schemas-show",
    1,
    "The JSON Schema of each document id given.",
);

/// The path from the root to `schemas list`, which the recovery of an
/// unknown id runs.
const LIST_PATH: &[&str] = &["schemas", "list"];

/// The path from the root to `schemas show`, which the next step of
/// `schemas list` names.
const SHOW_PATH: &[&str] = &["schemas", "show"];

/// The arguments of `<tool> schemas list`: the paging flags.
///
/// A tool's `schemas list` subcommand takes it as its arguments type, as in
/// the [module example](self).
#[derive(Args, Clone, Debug, PartialEq, Eq)]
// The doc comment is rustdoc, and clap would copy it into the help of a
// command without a description of its own.
#[command(about = None, long_about = None)]
pub struct SchemasList {
    /// How many documents to print.
    #[command(flatten)]
    page: Page,
}

/// The arguments of `<tool> schemas show`: the ids of the documents whose
/// JSON Schemas to print, from 1 to 100 of them, as every `show` command
/// takes (ADR-0016).
///
/// A tool's `schemas show` subcommand takes it as its arguments type, as in
/// the [module example](self).
#[derive(Args, Clone, Debug, PartialEq, Eq)]
// The doc comment is rustdoc, and clap would copy it into the help of a
// command without a description of its own.
#[command(about = None, long_about = None)]
pub struct SchemasShow {
    /// The ids of the documents that `schemas list` lists, separated by
    /// spaces
    #[arg(required = true, value_name = "ID", num_args = 1..=100)]
    ids: Vec<String>,
}

/// Returns the documents that the tool prints, one page of them as `args`
/// asks, for the tool to pass to [`finish`](crate::finish) with
/// [`SCHEMAS_LIST`]. [`documents`] says which documents those are and in
/// which order.
///
/// It builds the whole command tree through [`Output::command`], so it sees
/// each command that `Command::defer` adds, and it builds no JSON Schema.
///
/// # Errors
///
/// It returns no failure. It returns a `Result` so that a tool passes it to
/// `finish` as it passes any handler's result.
pub fn list(
    output: &mut Output,
    guide: &Guide,
    args: &SchemasList,
) -> Result<Listing<SchemaRow>, Error> {
    let root = output.command();
    let rows = documents(root, guide).iter().map(SchemaRow::new).collect();
    Ok(args.page.first_of(rows).described("documents").next(
        Call::new(SHOW_PATH).arg("ids"),
        "prints the JSON Schema of each document.",
    ))
}

/// Returns the JSON Schema of each document that `args` names, in the order
/// given, for the tool to pass to [`finish`](crate::finish) with
/// [`SCHEMAS_SHOW`].
///
/// An id that names no document of [`documents`] adds a failure with the
/// code `<tool>::schemas::not_found` to the batch. Its recovery runs
/// `schemas list`. It builds the whole command tree through
/// [`Output::command`], and the JSON Schema of each id given once.
///
/// # Errors
///
/// It returns no failure of its own. It returns a `Result` so that a tool
/// passes it to `finish` as it passes any handler's result.
pub fn show(
    output: &mut Output,
    guide: &Guide,
    args: &SchemasShow,
) -> Result<Batch<SchemaShowRow, Error>, Error> {
    let root = output.command();
    let served = documents(root, guide);
    let mut batch = Batch::new(SHOW_PATH, "ids", &args.ids);
    for id in &args.ids {
        let result = match served.iter().find(|document| document.id() == *id) {
            Some(document) => Ok(SchemaShowRow::new(document)),
            None => Err(SchemasError::NotFound {
                tool: root.get_name().to_owned(),
                id: id.clone(),
            }
            .into()),
        };
        batch.push(result);
    }
    Ok(batch)
}

/// A failure of `schemas list` or `schemas show`.
#[derive(Debug)]
enum SchemasError {
    /// An id names no document that the tool prints.
    NotFound {
        /// The tool's name, such as `shelf`.
        tool: String,
        /// The id as the caller gave it.
        id: String,
    },
}

/// Prints the message, such as `shelf has no document shelf/books-lst/v1`.
impl fmt::Display for SchemasError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { tool, id } => write!(f, "{tool} has no document {id}"),
        }
    }
}

impl std::error::Error for SchemasError {}

impl Failure for SchemasError {
    fn code(&self) -> &'static Code {
        match self {
            Self::NotFound { .. } => &codes::SCHEMAS_NOT_FOUND,
        }
    }

    fn recovery(&self) -> Recovery {
        match self {
            Self::NotFound { .. } => Recovery::Run(Call::new(LIST_PATH)),
        }
    }
}
