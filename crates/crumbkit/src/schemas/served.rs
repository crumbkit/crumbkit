//! The documents that a tool prints, as the schema commands serve them.

use clap::Command;
use schemars::Schema;

use crate::Document;
use crate::help::Guide;
use crate::output::failure;

/// One document that a tool prints: its id, its description, the command
/// lines that print it, and how to build its JSON Schema.
#[derive(Clone, Debug)]
pub struct Served {
    /// The document's id and description.
    document: Document<()>,
    /// The command lines of the visible leaves that print the document,
    /// such as `shelf books list`, in guide order.
    commands: Vec<String>,
    /// Builds the document's JSON Schema from its id and description: the
    /// envelope of the first leaf in guide order that declares the
    /// document, or the failure document's schema.
    schema: fn(Document<()>) -> Schema,
}

impl Served {
    /// Returns the document's id, such as `shelf/books-list/v1`.
    #[must_use]
    pub fn id(&self) -> String {
        self.document.to_string()
    }

    /// Returns the document's id and description.
    #[must_use]
    pub const fn document(&self) -> Document<()> {
        self.document
    }

    /// Returns the document's one-line description.
    #[must_use]
    pub const fn description(&self) -> &'static str {
        self.document.description()
    }

    /// Returns the command lines that print the document, such as
    /// `shelf books list`, in guide order. The failure document has none,
    /// because every command prints it when it fails with `--json`.
    #[must_use]
    pub fn commands(&self) -> &[String] {
        &self.commands
    }

    /// Returns the document's 2020-12 JSON Schema. It builds the schema on
    /// each call.
    #[must_use]
    pub fn schema(&self) -> Schema {
        (self.schema)(self.document)
    }
}

/// Returns the documents that the tool whose built command tree is
/// `command` prints: `crumbkit/failure/v1` first, and then the document of
/// each leaf of `guide` in guide order.
///
/// A leaf counts when it declares a document and its path names a visible
/// command by real names. A leaf whose command, or a command above it, is
/// hidden or missing at runtime is left out, as `commands list` leaves it
/// out (ADR-0026). A document that several leaves declare is served once,
/// with each leaf's command line, and its schema comes from the first of
/// them. crumbkit-test's `check_schema_goldens` fails when two leaves with
/// one id have different schemas.
///
/// It builds no JSON Schema, and its work grows with the number of leaves
/// in `guide` and the depth of their paths.
#[must_use]
pub fn documents(command: &Command, guide: &Guide) -> Vec<Served> {
    let mut served = vec![Served {
        document: failure::DOCUMENT,
        commands: Vec::new(),
        schema: |_| failure::document_schema(),
    }];
    for leaf in guide.get_leaves() {
        let Some((document, schema)) = leaf.get_document().zip(leaf.envelope()) else {
            continue;
        };
        if !is_visible(command, leaf.path()) {
            continue;
        }
        let line = std::iter::once(command.get_name())
            .chain(leaf.path().iter().copied())
            .collect::<Vec<_>>()
            .join(" ");
        let id = document.to_string();
        match served.iter_mut().find(|entry| entry.id() == id) {
            Some(entry) => entry.commands.push(line),
            None => served.push(Served {
                document,
                commands: vec![line],
                schema,
            }),
        }
    }
    served
}

/// Reports whether `path`, the real names of subcommands below `command`,
/// names a command that is visible, as is every command above it.
fn is_visible(command: &Command, path: &[&str]) -> bool {
    let mut node = command;
    for word in path {
        match node
            .get_subcommands()
            .find(|subcommand| subcommand.get_name() == *word)
        {
            Some(subcommand) if !subcommand.is_hide_set() => node = subcommand,
            _ => return false,
        }
    }
    true
}
