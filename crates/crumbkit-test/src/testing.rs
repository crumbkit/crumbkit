//! Helpers that the checks' tests share.

use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};

/// Runs `check` and returns its panic message, or `None` when it passes.
///
/// `AssertUnwindSafe` is sound here, because a test drops the command tree
/// after the check panics and never reads it again.
pub(crate) fn panic_message(check: impl FnOnce()) -> Option<String> {
    let payload = panic::catch_unwind(AssertUnwindSafe(check)).err()?;
    Some(
        payload
            .downcast_ref::<String>()
            .cloned()
            .unwrap_or_default(),
    )
}

/// An empty directory under the system's temporary directory that one test
/// owns, and that is deleted with everything in it when the value drops.
///
/// Its name holds the process id and the test's name, so tests that run at
/// the same time in one process or in several never share a directory.
pub(crate) struct TempDir {
    /// The directory's path.
    path: PathBuf,
}

impl TempDir {
    /// Creates an empty directory for the test named `test`.
    pub(crate) fn new(test: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("crumbkit-test-{}-{test}", std::process::id()));
        #[expect(
            clippy::disallowed_methods,
            reason = "the test helper owns its temporary directory, and creates it empty"
        )]
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }

    /// Returns the directory's path.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Writes `contents` to the file `name` in the directory.
    pub(crate) fn write(&self, name: &str, contents: &str) {
        #[expect(
            clippy::disallowed_methods,
            reason = "the test helper owns its temporary directory, and writes the files a test starts from"
        )]
        fs::write(self.path.join(name), contents).unwrap();
    }

    /// Returns the names of the files in the directory, sorted.
    pub(crate) fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(&self.path)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        #[expect(
            clippy::disallowed_methods,
            reason = "the test helper owns its temporary directory, and deletes it when the test ends"
        )]
        let _ = fs::remove_dir_all(&self.path);
    }
}
