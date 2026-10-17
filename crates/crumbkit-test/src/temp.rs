//! Temporary directories, which hold the XDG directories of each example
//! run and the golden files that the checks' tests write.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
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
    pub(crate) fn path(&self) -> &Path {
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

    /// Writes `contents` to the file `name` in the directory and lets its
    /// owner run it, so a test can run a script as a tool's binary.
    #[cfg(unix)]
    #[cfg(test)]
    pub(crate) fn write_script(&self, name: &str, contents: &str) {
        use std::os::unix::fs::PermissionsExt as _;

        self.write(name, contents);
        fs::set_permissions(self.path.join(name), fs::Permissions::from_mode(0o700)).unwrap();
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

/// Copies every file and directory below `from` into the existing
/// directory `to`, keeping their relative paths, or returns the error of
/// the first entry that cannot be copied.
//
// RUST-10: the copy handles plain files and directories, which a fixture
// of a tool's XDG directories holds. It refuses a symbolic link, because a
// copied link could point a run at files outside its fresh directories,
// and it keeps no permissions beyond what `fs::copy` keeps. A crate such as
// fs_extra replaces it if a fixture needs links.
pub(crate) fn copy_tree(from: &Path, to: &Path) -> io::Result<()> {
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let source = entry.path();
        let target = to.join(entry.file_name());
        if kind.is_symlink() {
            return Err(io::Error::other(format!(
                "`{}` is a symbolic link, which the check does not copy",
                source.display()
            )));
        }
        if kind.is_dir() {
            #[expect(
                clippy::disallowed_methods,
                reason = "the copy writes only below `to`, a fresh directory that a TempDir owns"
            )]
            fs::create_dir_all(&target)?;
            copy_tree(&source, &target)?;
        } else {
            fs::copy(&source, &target)?;
        }
    }
    Ok(())
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
