//! Where a tool's aliases file lives, and how it is read.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use etcetera::BaseStrategy as _;

#[cfg(test)]
mod tests;

/// Returns the path of the aliases file of the tool named `tool`, the name
/// of its root command, or `None` when the home directory cannot be found.
///
/// The file is `aliases.toml` in the tool's directory under the config
/// directory that the etcetera crate's base strategy gives. On Linux and
/// macOS that is `$XDG_CONFIG_HOME/<tool>/aliases.toml`, or
/// `~/.config/<tool>/aliases.toml` when `XDG_CONFIG_HOME` is unset, empty or
/// a relative path, because the XDG Base Directory specification says to
/// ignore a relative path. The home directory is `HOME`, or the user's entry
/// in the password database when `HOME` is unset or empty. An absolute
/// `XDG_CONFIG_HOME` is the config directory without that lookup. The
/// function touches no file.
///
/// A tool's `main` calls it once and passes the result to
/// [`expand`](crate::expand) and [`list`](crate::list), so both read the same
/// file, and a test passes its own path to them instead.
#[must_use]
pub fn file(tool: &str) -> Option<PathBuf> {
    #[expect(
        clippy::disallowed_methods,
        reason = "this module owns the aliases file, and XDG_CONFIG_HOME names its directory"
    )]
    let config_home = std::env::var_os("XDG_CONFIG_HOME");
    file_in(tool, config_home)
}

/// Returns the path of the aliases file of `tool` when `XDG_CONFIG_HOME` is
/// `config_home`, as [`file`] describes.
///
/// An absolute `config_home` is the config directory. Otherwise etcetera
/// finds it, and etcetera first finds the home directory, which reads the
/// password database when `HOME` is unset. Every run of a tool calls
/// [`file`], so this skips that work when `XDG_CONFIG_HOME` names the
/// directory.
fn file_in(tool: &str, config_home: Option<OsString>) -> Option<PathBuf> {
    let config = match config_home.map(PathBuf::from) {
        Some(config) if config.is_absolute() => config,
        Some(_) | None => etcetera::choose_base_strategy().ok()?.config_dir(),
    };
    Some(config.join(tool).join("aliases.toml"))
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
