//! Temporary directories, which hold the XDG directories of each example
//! run and the golden files that the checks' tests write.

use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The number of temporary directories that this process has created, which
/// gives each new directory a number of its own.
static CREATED: AtomicUsize = AtomicUsize::new(0);

/// An empty directory under the system's temporary directory, which is
/// deleted with everything in it when the value drops.
///
/// Its name holds the process id, a number that no other directory of the
/// process has, and a label, so runs and tests at the same time in one
/// process or in several never share a directory.
pub(crate) struct TempDir {
    /// The directory's path.
    path: PathBuf,
}

impl TempDir {
    /// Creates an empty directory whose name ends with `label`, such as the
    /// name of the test that owns it.
    pub(crate) fn new(label: &str) -> io::Result<Self> {
        let number = CREATED.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "crumbkit-test-{}-{number}-{label}",
            std::process::id()
        ));
        #[expect(
            clippy::disallowed_methods,
            reason = "the temporary directory owns its path, and creates it empty"
        )]
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    /// Returns the directory's path.
    #[cfg(test)]
    pub(crate) fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Creates the empty directory `name` in the directory, and returns its
    /// path.
    pub(crate) fn create_dir(&self, name: &str) -> io::Result<PathBuf> {
        let path = self.path.join(name);
        #[expect(
            clippy::disallowed_methods,
            reason = "the temporary directory owns everything below it"
        )]
        fs::create_dir_all(&path)?;
        Ok(path)
    }

    /// Writes `contents` to the file `name` in the directory.
    #[cfg(test)]
    pub(crate) fn write(&self, name: &str, contents: &str) {
        #[expect(
            clippy::disallowed_methods,
            reason = "the temporary directory owns everything below it, and a test writes the files it starts from"
        )]
        fs::write(self.path.join(name), contents).unwrap();
    }

    /// Returns the names of the files in the directory, sorted.
    #[cfg(test)]
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
            reason = "the temporary directory owns its path, and deletes it when the value drops"
        )]
        let _ = fs::remove_dir_all(&self.path);
    }
}
