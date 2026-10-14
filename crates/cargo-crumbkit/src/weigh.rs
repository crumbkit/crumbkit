//! `cargo crumbkit weigh`, which measures in bytes what each path through a
//! tool's help costs an agent to read (ADR-0033).

use std::path::PathBuf;

use crumbkit::{Document, Row};
use etcetera::BaseStrategy as _;
use schemars::JsonSchema;
use serde::Serialize;

use crate::cargo;
use crate::cli::WeighArgs;
use crate::error::WeighError;
use crate::tool::{Listed, Tool, Used};

#[cfg(test)]
mod tests;

/// The document that `cargo crumbkit weigh` prints: one weight for each
/// visible command of the tool, in depth-first order from the root. Its
/// schema is `crates/cargo-crumbkit/schemas/weigh-v1.json`.
pub(crate) const WEIGH: Document<Vec<Weight>> = Document::new(
    "crumbkit",
    "weigh",
    1,
    "The bytes that an agent reads on each path through a tool's help.",
);

/// The directory of the example golden files in the binary's package, when
/// `--goldens` names none. shelf keeps its example golden files there.
const GOLDENS: &str = "tests/golden/examples";

/// The file name of a tool's usage log in its directory under the XDG state
/// directory, as crumbkit-usage writes it (ADR-0029).
const LOG: &str = "usage.jsonl";

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
    /// With --usage, the runs of the command in the usage log that printed no help page, and null without it.
    runs: Option<u64>,
    /// With --usage, the help pages that each run's session read since its previous run that printed no help page, summed over the command's runs, and null without it.
    help_before: Option<u64>,
    /// With --usage, the 95th percentile of the bytes that the command's runs wrote to standard output, and null without it or when no run printed a result.
    p95_output: Option<u64>,
}

impl Row for Weight {
    // The columns follow the order in which an agent reads: the short page,
    // the output, and then the sum. The usage figures follow, so the
    // measured runs sit beside the estimate. The long page is behind
    // `--long`, because an agent reads it only when it needs the details.
    const COLUMNS: &'static [&'static str] = &[
        "path",
        "short_help",
        "output",
        "path_total",
        "runs",
        "help_before",
        "p95_output",
    ];
    const LONG_COLUMNS: &'static [&'static str] = &["long_help"];
}

impl Weight {
    /// Returns the weight of the command at `path` with a path total of 0,
    /// which `weigh` sets, and no usage figures.
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
            runs: None,
            help_before: None,
            p95_output: None,
        }
    }

    /// Returns the number of commands between the root and the command,
    /// which is 0 for the root.
    fn depth(&self) -> usize {
        self.path.matches(' ').count()
    }

    /// Returns the name that `self figures list` gives the command: its
    /// path below the tool's name, or the tool's name for the root.
    fn below(&self) -> &str {
        self.path
            .split_once(' ')
            .map_or(self.path.as_str(), |(_, below)| below)
    }
}

/// Sets the usage figures of each of `rows` from `used`, the figures of
/// each command in a usage log. A command that the log does not name has 0
/// runs, 0 help pages before them and no output.
pub(crate) fn join(rows: &mut [Weight], used: &[Used]) {
    for row in rows {
        let figures = used.iter().find(|used| used.command == row.below());
        row.runs = Some(figures.map_or(0, |figures| figures.runs));
        row.help_before = Some(figures.map_or(0, |figures| figures.help_before));
        row.p95_output = figures.and_then(|figures| figures.p95_out_bytes);
    }
}

/// Returns the usage log that `args` names with `--log`, or else the log of
/// the tool whose commands are `commands`: `usage.jsonl` in the tool's
/// directory under [`state_home`], where crumbkit-usage
/// writes it.
fn usage_log(args: &WeighArgs, commands: &[Listed]) -> Result<PathBuf, WeighError> {
    if let Some(log) = &args.log {
        return Ok(log.clone());
    }
    // The first command is the root, whose path is the tool's name.
    let tool = commands.first().map_or("", |root| root.path.as_str());
    let state = state_home().ok_or(WeighError::NoStateDir)?;
    Ok(state.join(tool).join(LOG))
}

/// Returns the directory that holds each tool's log directory, found as
/// crumbkit-usage finds it (ADR-0029): the XDG state
/// directory on Unix, and on Windows `XDG_STATE_HOME` when it is an
/// absolute path, and otherwise `%LOCALAPPDATA%`.
#[cfg(not(windows))]
fn state_home() -> Option<PathBuf> {
    etcetera::choose_base_strategy().ok()?.state_dir()
}

/// Returns the directory that holds each tool's log directory on Windows,
/// as the Unix version of this function describes.
#[cfg(windows)]
fn state_home() -> Option<PathBuf> {
    #[expect(
        clippy::disallowed_methods,
        reason = "weigh reads XDG_STATE_HOME to find the usage log where crumbkit writes it"
    )]
    let xdg = std::env::var_os("XDG_STATE_HOME");
    windows_state_home(xdg, || {
        etcetera::choose_base_strategy()
            .ok()
            .map(|strategy| strategy.cache_dir())
    })
}

/// Returns `xdg` when it is an absolute path, and otherwise what `local`
/// returns, as crumbkit's usage log chooses its directory on Windows.
#[cfg(windows)]
fn windows_state_home(
    xdg: Option<std::ffi::OsString>,
    local: impl FnOnce() -> Option<PathBuf>,
) -> Option<PathBuf> {
    match xdg.map(PathBuf::from) {
        Some(path) if path.is_absolute() => Some(path),
        _ => local(),
    }
}

/// Builds the binary that `args` names, and returns the weight of each of
/// its visible commands in depth-first order from the root.
///
/// It runs the binary once for the command tree and twice for each visible
/// command, so its work grows with the size of the tree. With `--usage` or
/// `--log`, it runs the binary once more for the figures of the usage log.
/// The path totals and output weights come from crumbkit-test's
/// `path_totals` and `output_weight`, which its path weight check also
/// uses.
///
/// # Errors
///
/// Returns the [`WeighError`] of a build that fails, of a binary that
/// cannot run or lists no commands, of a help page that fails, of a golden
/// file that cannot be read, or of usage figures that cannot be read.
pub(crate) fn weigh(args: &WeighArgs) -> Result<Vec<Weight>, WeighError> {
    let built = cargo::build(&cargo::program(), &args.bin)?;
    let goldens = args
        .goldens
        .clone()
        .unwrap_or_else(|| built.package.join(GOLDENS));
    let tool = Tool::new(&built.executable, &std::env::temp_dir())?;
    let commands = tool.commands()?;
    let mut rows = Vec::new();
    for command in &commands {
        let short_help = tool.help_bytes(&command.path, "-h")?;
        let long_help = tool.help_bytes(&command.path, "--help")?;
        let output =
            crumbkit_test::output_weight(&goldens, &command.examples).map_err(|error| {
                let (path, error) = error.into_parts();
                WeighError::GoldenUnreadable { path, error }
            })?;
        rows.push(Weight::new(
            command.path.clone(),
            short_help,
            long_help,
            output,
        ));
    }
    let sums: Vec<(usize, u64, Option<u64>)> = rows
        .iter()
        .map(|row| (row.depth(), row.short_help, row.output))
        .collect();
    for (row, total) in rows.iter_mut().zip(crumbkit_test::path_totals(&sums)) {
        row.path_total = total;
    }
    if args.usage() {
        let log = usage_log(args, &commands)?;
        join(&mut rows, &tool.figures(&log, args.since.as_deref())?);
    }
    Ok(rows)
}
