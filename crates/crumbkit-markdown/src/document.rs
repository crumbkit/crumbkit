//! The `crumbkit/commands-list/v1` document that a tool's
//! `commands list --all --json --full` prints, with the fields that the
//! reference reads. The schema of the document is crumbkit's
//! `crates/crumbkit/schemas/commands-list-v1.json`, which describes each
//! field.

use serde::Deserialize;

use crate::error::{Kind, RenderError};

#[cfg(test)]
mod tests;

/// The id of the document that the reference reads.
pub(crate) const COMMANDS_LIST: &str = "crumbkit/commands-list/v1";

/// The start of any crumbkit document, which names its schema id.
#[derive(Deserialize)]
struct Head {
    /// The document's id.
    schema: String,
}

/// The `crumbkit/commands-list/v1` document. serde ignores the fields that
/// the reference does not read.
#[derive(Deserialize)]
struct Document {
    /// One row for each visible command, in depth-first order.
    data: Vec<Command>,
    /// The number of commands in the branch, of which `data` holds a page.
    /// A listing whose tool did not count its rows leaves it out.
    total: Option<usize>,
    /// Whether the document leaves out commands. A document from a
    /// crumbkit release before the field existed has none.
    #[serde(default)]
    truncated: bool,
}

/// One visible command.
#[derive(Debug, Deserialize)]
pub(crate) struct Command {
    /// The command line that runs the command, such as `shelf books list`.
    pub(crate) path: String,
    /// The one-line description.
    pub(crate) about: Option<String>,
    /// The full description that `--help` shows.
    pub(crate) long_about: Option<String>,
    /// The arguments, in definition order.
    pub(crate) arguments: Vec<Argument>,
    /// The examples, in guide order.
    pub(crate) examples: Vec<Example>,
    /// The columns of the result.
    pub(crate) columns: Vec<Column>,
    /// `stream`, `bytes`, `headed`, `document`, or `None` for a command
    /// without a document.
    pub(crate) form: Option<String>,
    /// The tool's own exit codes for the command.
    pub(crate) exits: Vec<Exit>,
    /// The effects, or `None` when the tool declares none.
    pub(crate) effects: Option<Effects>,
    /// The write guards, or `None` for a command that writes nothing or a
    /// document from a crumbkit release before the field existed.
    #[serde(default)]
    pub(crate) writes: Option<Writes>,
}

/// One argument of a command.
#[derive(Debug, Deserialize)]
pub(crate) struct Argument {
    /// The name as help writes it, such as `--all`, `-a` or `<ID>`.
    pub(crate) name: String,
    /// The short form, such as `-n`.
    pub(crate) short: Option<String>,
    /// The names of the values that the argument takes.
    pub(crate) value_names: Vec<String>,
    /// Whether the command refuses a command line without the argument.
    pub(crate) required: bool,
    /// Whether the argument takes more than one value.
    pub(crate) many: bool,
    /// The values that the argument takes when the caller leaves it out.
    pub(crate) defaults: Vec<String>,
    /// The values that the argument accepts.
    pub(crate) possible_values: Vec<String>,
    /// The one-line help.
    pub(crate) help: Option<String>,
    /// The full help that `--help` shows.
    pub(crate) long_help: Option<String>,
    /// Whether the argument also works after every subcommand.
    pub(crate) global: bool,
    /// The arguments that can be given in place of this one.
    pub(crate) one_of: Vec<String>,
}

/// One example of a command.
#[derive(Debug, Deserialize)]
pub(crate) struct Example {
    /// The full command line.
    pub(crate) command: String,
    /// The one-line description, which is empty when there is none.
    pub(crate) about: String,
}

/// One column of a command's result.
#[derive(Debug, Deserialize)]
pub(crate) struct Column {
    /// The column's name.
    pub(crate) name: String,
    /// Whether `--long` adds the column.
    pub(crate) long: bool,
    /// The column's description.
    pub(crate) description: Option<String>,
}

/// One exit code of a command.
#[derive(Debug, Deserialize)]
pub(crate) struct Exit {
    /// The exit code.
    pub(crate) exit: u8,
    /// What the code means.
    pub(crate) meaning: String,
}

/// A command's effects, as the Model Context Protocol's tool annotations
/// name them.
#[derive(Clone, Copy, Debug, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the four flags are the four booleans of the document's effects object"
)]
pub(crate) struct Effects {
    /// Whether the command changes nothing.
    pub(crate) read_only: bool,
    /// Whether the command may delete or overwrite data.
    pub(crate) destructive: bool,
    /// Whether a second run changes nothing more than the first.
    pub(crate) idempotent: bool,
    /// Whether the command reaches systems outside the tool.
    pub(crate) open_world: bool,
}

/// The guards of a command that writes (ADR-0047).
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Writes {
    /// Whether the command replaces existing data.
    pub(crate) replaces: bool,
    /// Whether the command takes crumbkit's `--expect`.
    pub(crate) expect: bool,
    /// `all_or_none`, `per_item`, or `None` when the tool declares neither.
    pub(crate) items: Option<String>,
}

/// Returns the commands of `input`, the bytes of a
/// `crumbkit/commands-list/v1` document that holds every command of its
/// branch.
///
/// # Errors
///
/// Returns an error when `input` is not JSON of the document's shape, when
/// it names another schema id, when it holds one page of a longer listing,
/// or when it holds no command.
pub(crate) fn read(input: &[u8]) -> Result<Vec<Command>, RenderError> {
    let not_a_document = |error| RenderError::new(Kind::NotADocument(error));
    let head: Head = serde_json::from_slice(input).map_err(not_a_document)?;
    if head.schema != COMMANDS_LIST {
        return Err(RenderError::new(Kind::OtherSchema { found: head.schema }));
    }
    let document: Document = serde_json::from_slice(input).map_err(not_a_document)?;
    let shown = document.data.len();
    if document.truncated || document.total.is_none_or(|total| shown < total) {
        return Err(RenderError::new(Kind::Page {
            shown,
            total: document.total,
        }));
    }
    if document.data.is_empty() {
        return Err(RenderError::new(Kind::Empty));
    }
    Ok(document.data)
}
