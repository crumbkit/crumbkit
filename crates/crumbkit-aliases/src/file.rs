//! Where a tool's aliases file lives, and how it is read.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

/// Returns the path of the aliases file of the tool named `tool`, the name
/// of its root command, or `None` when the environment gives no config
/// directory.
///
/// The file is `$XDG_CONFIG_HOME/<tool>/aliases.toml`, or
/// `$HOME/.config/<tool>/aliases.toml` when `XDG_CONFIG_HOME` is unset,
/// empty or a relative path, on Linux and macOS alike. The XDG Base
/// Directory specification says to ignore a relative path, and `HOME` must
/// be an absolute path too, so the file never depends on the current
/// directory. The function reads the two variables and touches no file.
///
/// A tool's `main` calls it once and passes the result to
/// [`expand`](crate::expand) and [`list`](crate::list), so both read the same
/// file, and a test passes its own path to them instead.
#[must_use]
pub fn file(tool: &str) -> Option<PathBuf> {
    #[expect(
        clippy::disallowed_methods,
        reason = "this function owns crumbkit-aliases' environment reads, which locate the aliases file"
    )]
    let (config_home, home) = (
        std::env::var_os("XDG_CONFIG_HOME"),
        std::env::var_os("HOME"),
    );
    locate(tool, config_home, home)
}

/// Returns the aliases file of `tool` from the values of `XDG_CONFIG_HOME`
/// and `HOME`, as [`file`] describes.
fn locate(tool: &str, config_home: Option<OsString>, home: Option<OsString>) -> Option<PathBuf> {
    let config = match config_home.map(PathBuf::from) {
        Some(config_home) if config_home.is_absolute() => config_home,
        _ => {
            let home = PathBuf::from(home?);
            if !home.is_absolute() {
                return None;
            }
            home.join(".config")
        }
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
