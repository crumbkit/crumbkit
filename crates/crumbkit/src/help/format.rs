//! File formats, which a tool registers in its guide so that the schema
//! commands serve them beside the documents that its commands print.

use schemars::{JsonSchema, Schema};

use crate::Document;

/// A file format that users write and no command prints, such as a
/// configuration or manifest file, with its JSON Schema.
///
/// A tool registers its formats with [`Guide::documents`](super::Guide::documents),
/// and `schemas list` and `schemas show` serve each one after the documents
/// that the tool's commands print (ADR-0035). The document's name says what
/// the file holds, such as `shelf/config/v1`, because no command path names
/// it. Its schema is the schema of `T` with the id as its `title` and the
/// document's description, without crumbkit's envelope, because users write
/// the file.
///
/// ```
/// # #[derive(schemars::JsonSchema)]
/// # struct Config { page: u32 }
/// use crumbkit::Document;
/// use crumbkit::help::{FileFormat, Guide};
///
/// const CONFIG: Document<Config> =
///     Document::new("shelf", "config", 1, "The settings file that users write.");
///
/// const GUIDE: Guide = Guide::new().documents(&[FileFormat::new(CONFIG)]);
///
/// assert_eq!(GUIDE.get_documents()[0].document().to_string(), "shelf/config/v1");
/// ```
#[derive(Clone, Copy, Debug)]
pub struct FileFormat {
    /// The format's id and description.
    document: Document<()>,
    /// Builds the format's JSON Schema from its id and description.
    schema: fn(Document<()>) -> Schema,
}

impl FileFormat {
    /// Returns the file format of `document`, whose type `T` is the Rust
    /// type that the file holds.
    #[must_use]
    pub const fn new<T: JsonSchema>(document: Document<T>) -> Self {
        Self {
            document: document.untyped(),
            schema: crate::schemas::file_format::<T>,
        }
    }

    /// Returns the format's id and description.
    #[must_use]
    pub const fn document(&self) -> Document<()> {
        self.document
    }

    /// Returns the format's 2020-12 JSON Schema. It builds the schema on
    /// each call.
    #[must_use]
    pub fn schema(&self) -> Schema {
        (self.schema)(self.document)
    }

    /// Returns the function that builds the format's JSON Schema.
    pub(crate) fn schema_fn(&self) -> fn(Document<()>) -> Schema {
        self.schema
    }
}
