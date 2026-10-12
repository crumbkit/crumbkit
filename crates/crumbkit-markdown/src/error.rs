//! The reasons that [`render`](crate::render) cannot write a reference.

use std::fmt;

/// Why [`render`](crate::render) cannot write a reference from a document.
///
/// Its `Display` names the reason in one sentence, such as ``the document
/// holds 25 of 40 commands, so the reference would leave some out; list
/// them with `commands list --all` ``.
#[derive(Debug)]
pub struct RenderError {
    /// The reason.
    kind: Kind,
}

/// The reasons that a reference cannot be written.
#[derive(Debug)]
pub(crate) enum Kind {
    /// The input is not JSON in the shape of a `crumbkit/commands-list/v1`
    /// document.
    NotADocument(serde_json::Error),
    /// The input is a JSON document with another schema id.
    OtherSchema {
        /// The id that the document names.
        found: String,
    },
    /// The document is one page of a longer listing.
    Page {
        /// The rows that the document holds.
        shown: usize,
        /// The rows that the listing has, when the tool counted them.
        total: Option<usize>,
    },
    /// The document holds no command.
    Empty,
    /// Two headings have one anchor.
    AnchorClash {
        /// The anchor.
        anchor: String,
        /// The text of the earlier heading, such as a command line.
        first: String,
        /// The text of the later heading.
        second: String,
    },
}

impl RenderError {
    /// Returns the error of `kind`.
    pub(crate) const fn new(kind: Kind) -> Self {
        Self { kind }
    }
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            Kind::NotADocument(_) => {
                f.write_str("the input is not a crumbkit/commands-list/v1 document")
            }
            Kind::OtherSchema { found } => write!(
                f,
                "the input is a {found} document, and the reference needs a crumbkit/commands-list/v1 document"
            ),
            Kind::Page {
                shown,
                total: Some(total),
            } => write!(
                f,
                "the document holds {shown} of {total} commands, so the reference would leave some out; list them with `commands list --all`"
            ),
            Kind::Page { shown, total: None } => write!(
                f,
                "the document leaves out commands after its first {shown}, so the reference would leave some out; list them with `commands list --all`"
            ),
            Kind::Empty => f.write_str("the document holds no command"),
            Kind::AnchorClash {
                anchor,
                first,
                second,
            } => write!(
                f,
                "`{first}` and `{second}` both have the anchor #{anchor}, so a link to the second would reach the first"
            ),
        }
    }
}

/// A document that is not JSON of the right shape has `serde_json`'s error
/// as its source.
impl std::error::Error for RenderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            Kind::NotADocument(error) => Some(error),
            Kind::OtherSchema { .. }
            | Kind::Page { .. }
            | Kind::Empty
            | Kind::AnchorClash { .. } => None,
        }
    }
}
