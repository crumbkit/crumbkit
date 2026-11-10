//! Helpers that the unit tests share.

use std::fs;
use std::path::Path;

/// Writes `text` to the file at `path`, as a person or another program left
/// it before a test runs.
pub(crate) fn write(path: &Path, text: &str) {
    #[expect(
        clippy::disallowed_methods,
        reason = "the tests write the files that install and check then find"
    )]
    fs::write(path, text).unwrap();
}

/// Makes a symbolic link at `link` that leads to `to`.
#[cfg(unix)]
pub(crate) fn link(to: &Path, link: &Path) {
    std::os::unix::fs::symlink(to, link).unwrap();
}
