//! `cargo crumbkit docs`, which writes a tool's Markdown command reference
//! (ADR-0044).

use std::fmt;
use std::fs;
use std::path::Path;

use crumbkit::{Document, Prose};
use schemars::JsonSchema;
use serde::Serialize;

use crate::cargo;
use crate::cli::DocsArgs;
use crate::error::DocsError;
use crate::tool::Tool;

#[cfg(test)]
mod tests;

/// The document that `cargo crumbkit docs` prints: the file that holds the
/// reference and the number of commands it describes. Its schema is
/// `crates/cargo-crumbkit/schemas/docs-v1.json`.
pub(crate) const DOCS: Document<Prose<Written>> = Document::new(
    "crumbkit",
    "docs",
    1,
    "The file that holds a tool's Markdown command reference.",
);

/// The file of the reference in the binary's package, when `--out` names
/// none. shelf keeps its reference there.
const REFERENCE: &str = "docs/commands.md";

/// The reference that `docs` wrote.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct Written {
    /// The path of the file that holds the reference.
    file: String,
    /// The number of commands that the reference describes.
    commands: u64,
}

impl Written {
    /// Returns the reference of `commands` commands written to `file`.
    pub(crate) fn new(file: &Path, commands: usize) -> Self {
        Self {
            file: file.display().to_string(),
            // A `usize` always fits in a `u64` on the 32-bit and 64-bit
            // platforms that Rust supports, so the conversion never
            // saturates.
            commands: u64::try_from(commands).unwrap_or(u64::MAX),
        }
    }
}

/// Prints one sentence, such as `Wrote the reference of 14 commands to
/// docs/commands.md.`
impl fmt::Display for Written {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Wrote the reference of {} commands to {}.",
            self.commands, self.file
        )
    }
}

/// Builds the binary `--bin` names, as `weigh` does, reads its
/// `commands list --all --json --full` document, and writes the reference
/// that crumbkit-markdown renders from it to `--out`, or to
/// `docs/commands.md` in the binary's package.
///
/// # Errors
///
/// Returns the [`DocsError`] of a build that fails, of a binary that cannot
/// run or lists no commands, of a document that crumbkit-markdown cannot
/// render, or of a file that cannot be written.
pub(crate) fn docs(args: &DocsArgs) -> Result<Prose<Written>, DocsError> {
    let built = cargo::build(&cargo::program(), &args.bin)?;
    let file = args
        .out
        .clone()
        .unwrap_or_else(|| built.package.join(REFERENCE));
    let tool = Tool::new(&built.executable, &std::env::temp_dir())?;
    let document = tool.document()?;
    let reference =
        crumbkit_markdown::render(&document).map_err(|error| DocsError::Unrenderable {
            line: tool.commands_line(),
            error,
        })?;
    write(&file, reference.text())?;
    Ok(Prose::new(Written::new(&file, reference.commands())))
}

/// Writes `text` to `file`, after creating the directory that holds it.
///
/// # Errors
///
/// Returns [`DocsError::WriteFailed`] when the directory or the file cannot
/// be written.
fn write(file: &Path, text: &str) -> Result<(), DocsError> {
    let failed = |error| DocsError::WriteFailed {
        path: file.to_path_buf(),
        error,
    };
    if let Some(dir) = file.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        #[expect(
            clippy::disallowed_methods,
            reason = "docs owns the file of the reference, and creates its directory"
        )]
        fs::create_dir_all(dir).map_err(failed)?;
    }
    #[expect(
        clippy::disallowed_methods,
        reason = "docs owns the file of the reference, and writes it"
    )]
    fs::write(file, text).map_err(failed)
}
