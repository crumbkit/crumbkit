//! The temporary directories that the usage log's unit tests write in.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

/// The number of temporary directories that this process has created, which
/// gives each new directory a number of its own.
static CREATED: AtomicUsize = AtomicUsize::new(0);

/// A directory under the system's temporary directory, which is deleted with
/// everything in it when the value drops. Its name holds the process id and
/// a number, so tests at the same time never share a directory. The
/// directory does not exist until a test creates it.
pub(super) struct TempDir {
    /// The directory's path.
    path: PathBuf,
}

impl TempDir {
    /// Returns a new directory path that ends with `label`.
    pub(super) fn new(label: &str) -> Self {
        let number = CREATED.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "crumbkit-usage-{}-{number}-{label}",
            std::process::id()
        ));
        Self { path }
    }

    /// Returns the directory's path.
    pub(super) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        #[expect(
            clippy::disallowed_methods,
            reason = "the temporary directory owns its path, and deletes it when the value drops"
        )]
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
