//! The temporary directories that the usage log's unit tests write in, and
//! the command tree that the report's unit tests check records against.

use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, Command};
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

/// Returns a built tool named shelf with a global `--full`, `books list`
/// with `--all`, `--limit` and `-n`, and `books show` with ids.
pub(super) fn tool() -> Command {
    let flag = |name: &'static str| Arg::new(name).long(name).action(ArgAction::SetTrue);
    let mut tool = Command::new("shelf")
        .arg(flag("full").global(true))
        .subcommand(
            Command::new("books")
                .subcommand(
                    Command::new("list")
                        .alias("ls")
                        .arg(flag("all"))
                        .arg(Arg::new("limit").long("limit"))
                        .arg(Arg::new("number").short('n')),
                )
                .subcommand(Command::new("show").arg(Arg::new("ids").num_args(1..))),
        );
    tool.build();
    tool
}
