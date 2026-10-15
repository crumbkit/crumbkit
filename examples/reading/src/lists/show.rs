//! `reading lists show`, which prints the file of each list it names under
//! a header.

use std::path::Path;

use crumbkit::{Batch, Bytes, Document, Headed, Row};
use schemars::JsonSchema;
use serde::Serialize;

use super::{EntriesError, export};

/// The document that `reading lists show` prints: the file of each list it
/// names, in the order the names were given. Its text prints each file
/// unchanged under a `==> NAME <==` header.
pub(crate) const LISTS_SHOW: Document<Batch<ListFile, EntriesError>> = Document::new(
    "reading",
    "lists-show",
    1,
    "The file of each reading list named, in the order the names were given.",
)
.headed();

/// The file of one reading list.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct ListFile {
    /// The list's name.
    name: String,
    /// The bytes of the list's file, as `text` when they are UTF-8 and as
    /// `base64` otherwise.
    #[serde(flatten)]
    content: Bytes,
}

impl Row for ListFile {
    const COLUMNS: &'static [&'static str] = &["name"];
}

impl Headed for ListFile {
    fn bytes(&self) -> Option<&Bytes> {
        Some(&self.content)
    }
}

/// Returns the file of each list in `dir` that `names` names, in the order
/// of `names`, with a failure in place of each list that cannot be read.
///
/// Each failure is the `EntriesError` of `reading entries export` for that
/// list: `NotFound` when `dir` is `None` or holds no list with the name, and
/// `CannotOpen` when the list's file cannot be read.
pub(crate) fn show(dir: Option<&Path>, names: &[String]) -> Batch<ListFile, EntriesError> {
    let mut batch = Batch::new(&["lists", "show"], "names", names);
    batch.extend(names.iter().map(|name| {
        export(dir, name).map(|content| ListFile {
            name: name.clone(),
            content,
        })
    }));
    batch
}
