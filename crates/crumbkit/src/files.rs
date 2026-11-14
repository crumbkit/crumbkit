//! The home directory and the path display that a command shares when it
//! installs a file of the tool's into a person's own directories, such as
//! crumbkit's `completion install` and crumbkit-skills' `skills install`
//! (ADR-0195).
//!
//! The module owns the reads of `HOME` and `USERPROFILE`. The writes that
//! replace a file go through crumbkit-durable (RUST-4).

use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

/// Returns the home directory that the process's environment names, or
/// `None` when it names none.
///
/// It reads `HOME`, and on Windows `USERPROFILE` when `HOME` is unset, and
/// counts the value only when it is an absolute path. It never asks the
/// operating system for the home directory, as `std::env::home_dir` does
/// by reading the password database when `HOME` is unset. A test or an
/// example run clears the environment, and that lookup would then write
/// into the real home directory (ADR-0027).
#[must_use]
pub fn home() -> Option<PathBuf> {
    #[expect(
        clippy::disallowed_methods,
        reason = "the files module owns the reads of HOME and USERPROFILE that find the home directory"
    )]
    home_in(|name| std::env::var_os(name))
}

/// Returns the home directory as [`home`] does, with `env` giving the value
/// of an environment variable by its name.
pub(crate) fn home_in(env: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let home = env("HOME");
    #[cfg(windows)]
    let home = home.or_else(|| env("USERPROFILE"));
    absolute(home)
}

/// Returns `value` as a path when it is an absolute path, and `None` when
/// it is unset, empty or relative.
pub(crate) fn absolute(value: Option<OsString>) -> Option<PathBuf> {
    value.map(PathBuf::from).filter(|path| path.is_absolute())
}

/// Returns `path` as a string for a row, with any bytes that are not UTF-8
/// replaced and without the `\\?\` prefix that Windows gives a canonical
/// path, as in `\\?\C:\Users`. A verbatim UNC path, `\\?\UNC\`, keeps the
/// prefix, because it names a share in another form without it. No Unix
/// path starts with the prefix, so this runs on every platform.
//
// RUST-10: the dunce crate also checks that the path stays valid without
// the prefix, such as one longer than 260 characters or one that names a
// reserved device. This strip only changes how a row shows the path, and no
// file access reads the result, so it skips those checks. dunce replaces it
// if a stripped path is ever opened.
#[must_use]
pub fn display(path: &Path) -> String {
    let text = path.display().to_string();
    match text.strip_prefix(r"\\?\") {
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => rest.to_owned(),
        _ => text,
    }
}
