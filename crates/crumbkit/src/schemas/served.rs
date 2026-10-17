//! The documents that a tool prints, as the schema commands serve them.

use clap::Command;
use schemars::Schema;

use crate::help::Guide;
use crate::output::failure;
use crate::{Document, ToolName};

/// One document that the schema commands serve: its id, its description,
/// its kind, the command lines that print it, and how to build its JSON
/// Schema.
#[derive(Clone, Debug)]
pub struct Served {
    /// The document's id and description.
    document: Document<()>,
    /// Whether a command prints the document in crumbkit's envelope, a
    /// command prints it without the envelope, or no command prints it.
    kind: Kind,
    /// The command lines of the visible leaves that print the document,
    /// such as `shelf books list`, in guide order.
    commands: Vec<String>,
    /// Whether the leaves that print the document print it as a stream,
    /// one line for each row.
    stream: bool,
    /// Builds the document's JSON Schema from its id and description: the
    /// failure document's schema, a file format's schema, or the schema of
    /// the first leaf in guide order that declares the document.
    schema: fn(Document<()>) -> Schema,
    /// The tool whose name the ids in the document's schema take in place
    /// of `crumbkit` (ADR-0079).
    tool: ToolName,
}

impl Served {
    /// Returns the document's id, such as `shelf/books-list/v1`, with the
    /// tool's name in place of `crumbkit` in a document that crumbkit
    /// declares, such as `shelf/failure/v1` (ADR-0079).
    #[must_use]
    pub fn id(&self) -> String {
        self.tool.document(self.document).to_string()
    }

    /// Returns the document's id and description as the tool or crumbkit
    /// declares it, so the id of a document that crumbkit declares has the
    /// component `crumbkit`. [`Served::id`] gives the id that the tool
    /// prints.
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
    /// because every command prints it when it fails with `--json`, and a
    /// file format has none, because no command prints it.
    #[must_use]
    pub fn commands(&self) -> &[String] {
        &self.commands
    }

    /// Returns the document's kind as `schemas list` prints it: `output`,
    /// `unwrapped` or `file`.
    pub(crate) const fn kind(&self) -> &'static str {
        match self.kind {
            Kind::Output => "output",
            Kind::Unwrapped => "unwrapped",
            Kind::File => "file",
        }
    }

    /// Returns the document's form as `schemas list` prints it: `stream`
    /// when a command prints it as a stream of lines, and `document`
    /// otherwise (ADR-0041).
    pub(crate) const fn form(&self) -> &'static str {
        if self.stream { "stream" } else { "document" }
    }

    /// Returns the document's 2020-12 JSON Schema, with the tool's name in
    /// place of `crumbkit` in each id that it names. It builds the schema on
    /// each call.
    #[must_use]
    pub fn schema(&self) -> Schema {
        let mut schema = (self.schema)(self.tool.document(self.document));
        if self.tool != ToolName::CRUMBKIT
            && let Some(value) = schema.as_object_mut()
        {
            name_ids(value, self.tool);
        }
        schema
    }
}

/// Returns the documents that the tool whose built command tree is
/// `command` serves: the failure document first, then the document of
/// each leaf of `guide` in guide order, and then each file format of
/// `guide` in the order that `Guide::documents` set (ADR-0035).
///
/// The failure document is `<tool>/failure/v1`, and it names no command,
/// because every command prints it when it fails.
///
/// A leaf counts when it declares a document and its path names a visible
/// command by real names. A leaf whose command, or a command above it, is
/// hidden or missing at runtime is left out, as `commands list` leaves it
/// out (ADR-0026). A document that several leaves declare is served once,
/// with each leaf's command line, and its schema comes from the first of
/// them. crumbkit-test's `check_schema_goldens` fails when two leaves with
/// one id have different schemas.
///
/// A file format is always served, because no command can hide it.
/// crumbkit-test's `check_schema_goldens` fails on a file format whose id
/// another document also has.
///
/// It builds no JSON Schema, and its work grows with the number of leaves
/// and file formats in `guide` and the depth of the leaves' paths.
#[must_use]
pub fn documents(command: &Command, guide: &Guide) -> Vec<Served> {
    let tool = guide.tool_name(command);
    let mut served = vec![Served {
        document: failure::DOCUMENT,
        kind: Kind::Output,
        commands: Vec::new(),
        stream: false,
        schema: |_| failure::document_schema(),
        tool,
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
        let id = tool.document(document).to_string();
        match served.iter_mut().find(|entry| entry.id() == id) {
            Some(entry) => entry.commands.push(line),
            None => served.push(Served {
                document,
                kind: if document.get_envelope().wraps() {
                    Kind::Output
                } else {
                    Kind::Unwrapped
                },
                commands: vec![line],
                stream: leaf.is_stream(),
                schema,
                tool,
            }),
        }
    }
    served.extend(guide.get_documents().iter().map(|format| Served {
        document: format.document(),
        kind: Kind::File,
        commands: Vec::new(),
        stream: false,
        schema: format.schema_fn(),
        tool,
    }));
    served
}

/// Gives `tool`'s name to each document id that `schema` names as a
/// `title` or a `const`, such as `crumbkit/failure/v1`, at any depth. An id
/// that does not start with `crumbkit/` stays as it is.
fn name_ids(schema: &mut serde_json::Map<String, serde_json::Value>, tool: ToolName) {
    for (key, value) in schema.iter_mut() {
        match value {
            serde_json::Value::String(id) if key == "title" || key == "const" => {
                *id = tool.schema_id(id);
            }
            serde_json::Value::Object(object) => name_ids(object, tool),
            serde_json::Value::Array(items) => {
                for item in items
                    .iter_mut()
                    .filter_map(serde_json::Value::as_object_mut)
                {
                    name_ids(item, tool);
                }
            }
            _ => {}
        }
    }
}

/// What a served document is, which `schemas list` prints as its `kind`.
#[derive(Clone, Copy, Debug)]
enum Kind {
    /// A command prints the document in crumbkit's envelope, or every
    /// command prints it on failure, as the failure document.
    Output,
    /// A command prints the document without crumbkit's envelope.
    Unwrapped,
    /// A file format that users write and no command prints.
    File,
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
