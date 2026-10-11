//! `cargo crumbkit weigh`, which measures in bytes what each path through a
//! tool's help costs an agent to read (ADR-0033).

use crumbkit::{Document, Row};
use schemars::JsonSchema;
use serde::Serialize;

use crate::cargo;
use crate::cli::WeighArgs;
use crate::error::WeighError;
use crate::goldens;
use crate::tool::Tool;

#[cfg(test)]
mod tests;

/// The document that `cargo crumbkit weigh` prints: one weight for each
/// visible command of the tool, in depth-first order from the root. Its
/// schema is `crates/cargo-crumbkit/schemas/weigh-v1.json`.
pub(crate) const WEIGH: Document<Vec<Weight>> = Document::new("crumbkit", "weigh", 1);

/// The directory of the example golden files in the binary's package, when
/// `--goldens` names none. shelf keeps its example golden files there.
const GOLDENS: &str = "tests/golden/examples";

/// What one visible command of a tool costs an agent to read, in bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub(crate) struct Weight {
    /// The command line that runs the command, such as `shelf books list`.
    path: String,
    /// The bytes of the command's `-h` page.
    short_help: u64,
    /// The bytes of the command's `--help` page.
    long_help: u64,
    /// The bytes of the largest golden file of the command's examples, or null when no example has a golden file.
    output: Option<u64>,
    /// The bytes that an agent reads to find and run the command: each `-h` page from the root down to the command, and the command's output.
    path_total: u64,
}

impl Row for Weight {
    // The columns follow the order in which an agent reads: the short page,
    // the long page when it needs the details, the output, and then the sum.
    const COLUMNS: &'static [&'static str] =
        &["path", "short_help", "long_help", "output", "path_total"];
}

impl Weight {
    /// Returns the weight of the command at `path` with a path total of 0,
    /// which [`add_path_totals`] sets.
    pub(crate) const fn new(
        path: String,
        short_help: u64,
        long_help: u64,
        output: Option<u64>,
    ) -> Self {
        Self {
            path,
            short_help,
            long_help,
            output,
            path_total: 0,
        }
    }

    /// Returns the number of commands between the root and the command,
    /// which is 0 for the root.
    fn depth(&self) -> usize {
        self.path.matches(' ').count()
    }
}

/// Sets the path total of each of `rows`: the `-h` bytes of the row's
/// command and of each command above it, up to the root, plus the row's
/// output bytes.
///
/// `rows` hold one command each in depth-first order from the root, as
/// `commands list` lists them, so the commands above a row are the nearest
/// earlier row at each smaller depth.
pub(crate) fn add_path_totals(rows: &mut [Weight]) {
    // `above[depth]` holds the sum of the `-h` bytes from the root down to
    // the latest row at that depth.
    let mut above: Vec<u64> = Vec::new();
    for row in rows {
        above.truncate(row.depth());
        let pages = above
            .last()
            .copied()
            .unwrap_or(0)
            .saturating_add(row.short_help);
        above.push(pages);
        row.path_total = pages.saturating_add(row.output.unwrap_or(0));
    }
}

/// Builds the binary that `args` names, and returns the weight of each of
/// its visible commands in depth-first order from the root.
///
/// It runs the binary once for the command tree and twice for each visible
/// command, so its work grows with the size of the tree.
///
/// # Errors
///
/// Returns the [`WeighError`] of a build that fails, of a binary that
/// cannot run or lists no commands, of a help page that fails, or of a
/// golden file that cannot be read.
pub(crate) fn weigh(args: &WeighArgs) -> Result<Vec<Weight>, WeighError> {
    let built = cargo::build(&cargo::program(), &args.bin)?;
    let goldens = args
        .goldens
        .clone()
        .unwrap_or_else(|| built.package.join(GOLDENS));
    let tool = Tool::new(&built.executable, &std::env::temp_dir())?;
    let mut rows = Vec::new();
    for command in tool.commands()? {
        let short_help = tool.help_bytes(&command.path, "-h")?;
        let long_help = tool.help_bytes(&command.path, "--help")?;
        let output = goldens::largest(&goldens, &command.examples)?;
        rows.push(Weight::new(command.path, short_help, long_help, output));
    }
    add_path_totals(&mut rows);
    Ok(rows)
}
