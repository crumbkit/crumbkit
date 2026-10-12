//! The schema commands: `<tool> schemas list`, which lists the JSON
//! documents that a tool prints, and `<tool> schemas show <ID>...`, which
//! prints the JSON Schema of each, so an agent learns the shape of a
//! document from the tool itself (ADR-0035).
//!
//! The documents come from the tool's [`Guide`]: each leaf that declares
//! the document it prints with [`Leaf::document`](crate::help::Leaf::document)
//! adds that document, and crumbkit adds `crumbkit/failure/v1`, which every
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
//! use crumbkit::{Output, Spelling};
//!
//! /// Browse a small catalogue.
//! #[derive(Parser)]
//! #[command(name = "shelf")]
//! struct Cli {
//!     #[command(subcommand)]
//!     resource: Resource,
//!     #[command(flatten)]
//!     output: Output,
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
//! const SPELLING: Spelling = Spelling::COLONS;
//!
//! fn run(args: &[&str]) -> ExitCode {
//!     let (mut cli, parsed) =
//!         match crumbkit::parse_from::<Cli, _, _>(Cli::command(), &GUIDE, SPELLING, args) {
//!             Ok(parsed) => parsed,
//!             Err(stop) => return stop.code(),
//!         };
//!     cli.output.set_command(parsed);
//!     match cli.resource {
//!         Resource::Schemas { command } => match command {
//!             // `list` and `show` build the command tree in `cli.output`,
//!             // so each runs before `finish`.
//!             SchemasCommand::List(args) => {
//!                 let result = schemas::list(&mut cli.output, &GUIDE, &args);
//!                 crumbkit::finish(&mut cli.output, SPELLING, SCHEMAS_LIST, result)
//!             }
//!             SchemasCommand::Show(args) => {
//!                 let result = schemas::show(&mut cli.output, &GUIDE, &args);
//!                 crumbkit::finish(&mut cli.output, SPELLING, SCHEMAS_SHOW, result)
//!             }
//!         },
//!     }
//! }
//!
//! assert_eq!(run(&["shelf", "schemas", "list"]), ExitCode::SUCCESS);
//! assert_eq!(
//!     run(&["shelf", "schemas", "show", "crumbkit/failure/v1"]),
//!     ExitCode::SUCCESS
//! );
//! ```

mod envelope;
mod row;
mod served;

use std::fmt;

use clap::Args;

pub use self::envelope::{document_schema, stream_schema};
pub(crate) use self::envelope::{envelope, file_format};
pub use self::row::{SchemaRow, SchemaShowRow};
pub use self::served::{Served, documents};
use crate::help::Guide;
use crate::{Batch, Call, Code, Document, Error, Failure, Listing, Output, Page, Recovery};

#[cfg(test)]
mod tests;

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

/// The code of an id that names no document the tool prints, in the tool's
/// spelling, such as `crumbkit::schemas::not_found` (ADR-0035).
const NOT_FOUND: Code = Code::new("crumbkit", "schemas", "not_found");

/// The code of a schema command run in a tool that never passed its
/// command to `Output::set_command`, in the tool's spelling, such as
/// `crumbkit::schemas::unavailable` (ADR-0035).
const UNAVAILABLE: Code = Code::new("crumbkit", "schemas", "unavailable");

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
pub struct SchemasShow {
    /// The ids of the documents, such as `crumbkit/failure/v1`, separated
    /// by spaces
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
/// Returns a failure with the code `crumbkit::schemas::unavailable` and no
/// next step when the tool has not passed its command to
/// [`Output::set_command`].
pub fn list(
    output: &mut Output,
    guide: &Guide,
    args: &SchemasList,
) -> Result<Listing<SchemaRow>, Error> {
    let Some(root) = output.command() else {
        return Err(SchemasError::Unavailable.into());
    };
    let rows = documents(root, guide).iter().map(SchemaRow::new).collect();
    Ok(Listing::from_all(rows, args.page.limit())
        .described("documents")
        .next(
            Call::new(SHOW_PATH).arg("ids"),
            "prints the JSON Schema of each document.",
        ))
}

/// Returns the JSON Schema of each document that `args` names, in the order
/// given, for the tool to pass to [`finish`](crate::finish) with
/// [`SCHEMAS_SHOW`].
///
/// An id that names no document of [`documents`] adds a failure with the
/// code `crumbkit::schemas::not_found` to the batch. Its recovery runs
/// `schemas list`. It builds the whole command tree through
/// [`Output::command`], and the JSON Schema of each id given once.
///
/// # Errors
///
/// Returns a failure with the code `crumbkit::schemas::unavailable` and no
/// next step when the tool has not passed its command to
/// [`Output::set_command`].
pub fn show(
    output: &mut Output,
    guide: &Guide,
    args: &SchemasShow,
) -> Result<Batch<SchemaShowRow, Error>, Error> {
    let Some(root) = output.command() else {
        return Err(SchemasError::Unavailable.into());
    };
    let served = documents(root, guide);
    let mut rows = Vec::new();
    let mut failures = Vec::new();
    for id in &args.ids {
        match served.iter().find(|document| document.id() == *id) {
            Some(document) => rows.push(SchemaShowRow::new(document)),
            None => failures.push(
                SchemasError::NotFound {
                    tool: root.get_name().to_owned(),
                    id: id.clone(),
                }
                .into(),
            ),
        }
    }
    Ok(Batch::new(rows, failures))
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
    /// The tool has not passed its command to `Output::set_command`.
    Unavailable,
}

/// Prints the message, such as `shelf has no document shelf/books-lst/v1`.
impl fmt::Display for SchemasError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { tool, id } => write!(f, "{tool} has no document {id}"),
            Self::Unavailable => f.write_str(
                "the tool has not kept its command tree, so it cannot list its documents",
            ),
        }
    }
}

impl std::error::Error for SchemasError {}

impl Failure for SchemasError {
    fn code(&self) -> &'static Code {
        match self {
            Self::NotFound { .. } => &NOT_FOUND,
            Self::Unavailable => &UNAVAILABLE,
        }
    }

    fn recovery(&self) -> Recovery {
        match self {
            Self::NotFound { .. } => Recovery::Run(Call::new(LIST_PATH)),
            Self::Unavailable => Recovery::None {
                reason: "the tool's main does not pass its command to Output::set_command"
                    .to_owned(),
            },
        }
    }
}
