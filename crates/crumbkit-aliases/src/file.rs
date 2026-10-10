//! Where a tool's aliases file lives, and how it is read.

use std::io;
use std::path::{Path, PathBuf};

use etcetera::BaseStrategy as _;

/// Returns the path of the aliases file of the tool named `tool`, the name
/// of its root command, or `None` when the home directory cannot be found.
///
/// The file is `aliases.toml` in the tool's directory under the config
/// directory that the etcetera crate's base strategy gives. On Linux and
/// macOS that is `$XDG_CONFIG_HOME/<tool>/aliases.toml`, or
/// `~/.config/<tool>/aliases.toml` when `XDG_CONFIG_HOME` is unset, empty or
/// a relative path, because the XDG Base Directory specification says to
/// ignore a relative path. The home directory is `HOME`, or the user's entry
/// in the password database when `HOME` is unset or empty. The function
/// touches no file.
///
/// A tool's `main` calls it once and passes the result to
/// [`expand`](crate::expand) and [`list`](crate::list), so both read the same
/// file, and a test passes its own path to them instead.
#[must_use]
pub fn file(tool: &str) -> Option<PathBuf> {
    let strategy = etcetera::choose_base_strategy().ok()?;
    Some(strategy.config_dir().join(tool).join("aliases.toml"))
}

/// Returns the text of the aliases file at `path`, or `None` when no file is
/// there.
///
/// # Errors
///
/// Returns the I/O error of a file that is there but cannot be read, such
/// as a directory or a file that is not UTF-8.
pub(crate) fn read(path: &Path) -> io::Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}
