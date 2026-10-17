//! The documents that a tool prints, as the schema commands serve them.

use clap::Command;
use schemars::Schema;

use crate::form::FailureForm;
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
    /// Builds the document's JSON Schema.
    schema: Build,
    /// The failure field names that the guide lists with
    /// `Guide::field_names`, which the schema of a failure in crumbkit's
    /// form accepts beside the name rule (ADR-0061). It is empty for a
    /// failure form's document and a file format.
    field_names: &'static [&'static str],
    /// The tool whose name the ids in the document's schema take in place
    /// of `crumbkit` (ADR-0079).
    tool: ToolName,
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
            Kind::OwnEnvelope => "own-envelope",
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
    /// each call. When the guide lists field names with
    /// `Guide::field_names`, each failure in crumbkit's form that the
    /// schema describes accepts those names beside the name rule.
    #[must_use]
    pub fn schema(&self) -> Schema {
        let mut schema = self.build();
        if self.tool != ToolName::CRUMBKIT
            && let Some(value) = schema.as_object_mut()
        {
            name_ids(value, self.tool);
        }
        if !self.field_names.is_empty() {
            let value = schema.as_object_mut();
            if let Some(value) = value {
                accept_field_names(value, self.field_names);
                let entry = pointer(value, &["properties", "failures", "items"]);
                if let Some(entry) = entry {
                    accept_field_names(entry, self.field_names);
                }
                let in_order = in_order_failure(value, self.document.get_envelope().data_key())
                    .and_then(serde_json::Value::as_object_mut);
                if let Some(in_order) = in_order {
                    accept_field_names(in_order, self.field_names);
                }
            }
        }
        schema
    }

    /// Builds the document's JSON Schema as [`Served::schema`] describes,
    /// before it accepts the guide's field names.
    fn build(&self) -> Schema {
        match self.schema {
            Build::Function(build) => build(self.tool.document(self.document)),
            Build::Leaf(build, form) => {
                let mut schema = build(self.tool.document(self.document));
                if let Some(form) = form {
                    set_entry_schema(&mut schema, form.entry_schema());
                    if self.document.get_envelope().failures_in_order() {
                        set_in_order_failure_schema(
                            &mut schema,
                            self.document.get_envelope().data_key(),
                            form.schema(),
                        );
                    }
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
/// The failure document is `<tool>/failure/v1`, or the document of the
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
    let tool = guide.tool_name(command);
    let mut served = vec![match guide.get_failure_form() {
        Some(form) => Served {
            document: form.document(),
            kind: Kind::Unwrapped,
            commands: Vec::new(),
            stream: false,
            schema: Build::Form(form),
            field_names: &[],
            tool,
        },
        None => Served {
            document: failure::DOCUMENT,
            kind: Kind::Output,
            commands: Vec::new(),
            stream: false,
            schema: Build::Function(|_| failure::document_schema()),
            field_names: guide.get_field_names(),
            tool,
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
        let id = tool.document(document).to_string();
        match served.iter_mut().find(|entry| entry.id() == id) {
            Some(entry) => entry.commands.push(line),
            None => served.push(Served {
                document,
                kind: if document.get_envelope().has_own_keys() {
                    Kind::OwnEnvelope
                } else if document.get_envelope().wraps() {
                    Kind::Output
                } else {
                    Kind::Unwrapped
                },
                commands: vec![line],
                stream: leaf.is_stream(),
                schema: Build::Leaf(schema, guide.get_failure_form()),
                // A failure form describes its own fields, so only
                // crumbkit's failures accept the guide's names.
                field_names: if guide.get_failure_form().is_some() {
                    &[]
                } else {
                    guide.get_field_names()
                },
                tool,
            }),
        }
    }
    served.extend(guide.get_documents().iter().map(|format| Served {
        document: format.document(),
        kind: Kind::File,
        commands: Vec::new(),
        stream: false,
        schema: Build::Function(format.schema_fn()),
        field_names: &[],
        tool,
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

/// Puts `failure` as the second kind of item of `schema`, the schema of a
/// batch that prints its failures in input order, in place of the
/// `crumbkit/failure/v1` document. `data_key` is the key that holds the
/// array in a document with keys that the tool declares.
fn set_in_order_failure_schema(schema: &mut Schema, data_key: &str, failure: Schema) {
    let slot = schema
        .as_object_mut()
        .and_then(|schema| in_order_failure(schema, data_key));
    if let Some(slot) = slot {
        *slot = failure.to_value();
    }
}

/// Returns the second kind of item of the in-order array of `schema`: the
/// array at the top of a document without an envelope, or the array under
/// `data_key` in a document with keys that the tool declares (ADR-0059).
fn in_order_failure<'a>(
    schema: &'a mut serde_json::Map<String, serde_json::Value>,
    data_key: &str,
) -> Option<&'a mut serde_json::Value> {
    let array = if schema.contains_key("items") {
        schema
    } else {
        pointer(schema, &["properties", data_key])?
    };
    array.get_mut("items")?.get_mut("anyOf")?.get_mut(1)
}

/// The names that a failure field has under crumbkit's rule, as the
/// failure schema states them in `propertyNames`.
const NAME_RULE: &str = "^[a-z0-9_]+$";

/// Widens `failure`, the schema of one failure in crumbkit's form, so its
/// `fields` also accept the names in `names`. Its `propertyNames` changes
/// from the name rule to the rule or one of `names`. A schema whose
/// `fields` does not hold exactly the rule, such as a failure form's
/// schema, stays as it is.
fn accept_field_names(failure: &mut serde_json::Map<String, serde_json::Value>, names: &[&str]) {
    let Some(fields) = pointer(failure, &["properties", "fields"]) else {
        return;
    };
    let rule = serde_json::json!({ "pattern": NAME_RULE });
    if fields.get("propertyNames") != Some(&rule) {
        return;
    }
    fields.insert(
        "propertyNames".to_owned(),
        serde_json::json!({ "anyOf": [rule, { "enum": names }] }),
    );
    if let Some(serde_json::Value::String(description)) = fields.get_mut("description") {
        description.push_str(
            " The tool also uses the names that propertyNames lists, which it takes from another system.",
        );
    }
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

/// Returns the object at `path`, a list of keys of nested objects below
/// `object`, or `None` when one of them is missing or not an object.
fn pointer<'a>(
    object: &'a mut serde_json::Map<String, serde_json::Value>,
    path: &[&str],
) -> Option<&'a mut serde_json::Map<String, serde_json::Value>> {
    let mut object = object;
    for key in path {
        object = object.get_mut(*key)?.as_object_mut()?;
    }
    Some(object)
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
    /// A command prints the document under keys that the tool declares
    /// with [`Envelope::Keys`](crate::Envelope::Keys) (ADR-0059).
    OwnEnvelope,
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
