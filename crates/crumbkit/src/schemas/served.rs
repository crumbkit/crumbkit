//! The documents that a tool prints, as the schema commands serve them.

use clap::Command;
use schemars::Schema;

use crate::Document;
use crate::form::FailureForm;
use crate::help::Guide;
use crate::output::failure;

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
    /// Builds the document's JSON Schema.
    schema: Build,
}

/// How a served document builds its JSON Schema.
#[derive(Clone, Copy, Debug)]
enum Build {
    /// A function of the document's id and description: the failure
    /// document's schema or a file format's schema.
    Function(fn(Document<()>) -> Schema),
    /// The schema of the first leaf in guide order that declares the
    /// document, from a function of the document's id and description, with
    /// the entry schema of the guide's failure form, when it has one, as
    /// the `items` of `failures`.
    Leaf(fn(Document<()>) -> Schema, Option<&'static dyn FailureForm>),
    /// The schema of a tool's failure form, with the document's id as its
    /// `title`.
    Form(&'static dyn FailureForm),
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
    /// `shelf books list`, in guide order. The failure document and a
    /// failure form's document have none, because every command prints them
    /// when it fails with `--json`, and a file format has none, because no
    /// command prints it.
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

    /// Returns the document's 2020-12 JSON Schema. It builds the schema on
    /// each call.
    #[must_use]
    pub fn schema(&self) -> Schema {
        match self.schema {
            Build::Function(build) => build(self.document),
            Build::Leaf(build, form) => {
                let mut schema = build(self.document);
                if let Some(form) = form {
                    set_entry_schema(&mut schema, form.entry_schema());
                }
                schema
            }
            Build::Form(form) => {
                let mut schema = form.schema();
                schema.insert("title".to_owned(), self.id().into());
                schema
            }
        }
    }
}

/// Returns the documents that the tool whose built command tree is
/// `command` serves: the failure document first, then the document of
/// each leaf of `guide` in guide order, and then each file format of
/// `guide` in the order that `Guide::documents` set (ADR-0035).
///
/// The failure document is `crumbkit/failure/v1`, or the document of the
/// guide's failure form when it has one. A failure form's document is
/// `unwrapped`, because crumbkit's envelope does not hold it, and it names
/// no command, because every command prints it when it fails (ADR-0045).
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
    let mut served = vec![match guide.get_failure_form() {
        Some(form) => Served {
            document: form.document(),
            kind: Kind::Unwrapped,
            commands: Vec::new(),
            stream: false,
            schema: Build::Form(form),
        },
        None => Served {
            document: failure::DOCUMENT,
            kind: Kind::Output,
            commands: Vec::new(),
            stream: false,
            schema: Build::Function(|_| failure::document_schema()),
        },
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
                kind: if document.has_envelope() {
                    Kind::Output
                } else {
                    Kind::Unwrapped
                },
                commands: vec![line],
                stream: leaf.is_stream(),
                schema: Build::Leaf(schema, guide.get_failure_form()),
            }),
        }
    }
    served.extend(guide.get_documents().iter().map(|format| Served {
        document: format.document(),
        kind: Kind::File,
        commands: Vec::new(),
        stream: false,
        schema: Build::Function(format.schema_fn()),
    }));
    served
}

/// Puts `entry` as the `items` of the `failures` property of `schema`, a
/// document's schema. A schema without `failures`, such as that of a
/// listing or of a document without crumbkit's envelope, stays as it is.
fn set_entry_schema(schema: &mut Schema, entry: Schema) {
    let failures = schema
        .get_mut("properties")
        .and_then(|properties| properties.get_mut("failures"))
        .and_then(serde_json::Value::as_object_mut);
    if let Some(failures) = failures {
        failures.insert("items".to_owned(), entry.to_value());
    }
}

/// What a served document is, which `schemas list` prints as its `kind`.
#[derive(Clone, Copy, Debug)]
enum Kind {
    /// A command prints the document in crumbkit's envelope, or every
    /// command prints it on failure, as the failure document.
    Output,
    /// A command prints the document without crumbkit's envelope, or every
    /// command prints it on failure, as a failure form's document.
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
