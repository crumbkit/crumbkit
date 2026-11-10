//! The home directory and the file writes that a command shares when it
//! installs a file of the tool's into a person's own directories, such as
//! crumbkit's `completion install` and crumbkit-skills' `skills install`
//! (ADR-0195).
//!
//! The module owns the reads of `HOME` and `USERPROFILE` and the writes that
//! replace a file through a temporary file and a rename (RUST-4).

use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, Write as _};
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

/// Writes `text` to a new temporary file beside `file` and renames it over
/// `file`, so a reader sees the old text or the new text and never part of
/// it. The rename replaces a symbolic link at `file` itself, never the file
/// that the link leads to.
///
/// The temporary file is `.<name>.<pid>.tmp` in `file`'s folder, where
/// `<name>` is `file`'s name and `<pid>` the process's id. It must be new,
/// so the write never follows a link or fills a file that someone else left
/// at that name, and it fails with that file's error instead. The temporary
/// file is removed when a later step fails.
///
/// # Errors
///
/// Returns the I/O error of the step that failed: creating the temporary
/// file, writing it, flushing it to disk or renaming it.
pub fn write_atomically(file: &Path, text: &str) -> io::Result<()> {
    let name = file.file_name().unwrap_or_default().to_string_lossy();
    let temporary = file.with_file_name(format!(".{name}.{}.tmp", std::process::id()));
    #[expect(
        clippy::disallowed_methods,
        reason = "the files module owns the writes that replace a file through a new temporary file"
    )]
    let mut handle = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    #[expect(
        clippy::disallowed_methods,
        reason = "the files module owns the rename of its temporary file over the file it replaces"
    )]
    let result = handle
        .write_all(text.as_bytes())
        .and_then(|()| handle.sync_all())
        .and_then(|()| fs::rename(&temporary, file));
    if result.is_err() {
        #[expect(
            clippy::disallowed_methods,
            reason = "the files module removes its own temporary file after a failed write"
        )]
        let _ = fs::remove_file(&temporary);
    }
    result
}
