//! What a person's completion directory holds at the tool's completion
//! file, which `install` and `check` both read.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::script::written_by;

/// What is at the completion file's path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Place {
    /// Nothing is at the path.
    Absent,
    /// A file is at `path`: the completion file itself, or the file that a
    /// link at the completion file's path leads to when install wrote that
    /// file. `own` tells whether install wrote it.
    File {
        /// The file's path.
        path: PathBuf,
        /// The file's text, with any bytes that are not UTF-8 replaced.
        text: String,
        /// Whether the file's mark names the tool.
        own: bool,
    },
    /// A symbolic link at the path leads to `to`, which is not a file that
    /// install wrote, or does not exist.
    Linked {
        /// The link's target: its real path when it resolves, or the path
        /// that the link holds.
        to: PathBuf,
    },
    /// Something other than a file or a link is at the path, such as a
    /// directory.
    NotFile,
    /// The path or the file cannot be read.
    Unreadable {
        /// Why it cannot be read.
        reason: String,
    },
}

/// Returns what is at `file`, the completion file of the command `bin`.
/// It follows a symbolic link only to a file whose mark names `bin`.
pub(crate) fn place(file: &Path, bin: &str) -> Place {
    let metadata = match fs::symlink_metadata(file) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Place::Absent,
        Err(error) => {
            return Place::Unreadable {
                reason: error.to_string(),
            };
        }
    };
    if metadata.is_file() {
        return read(file, bin);
    }
    if !metadata.is_symlink() {
        return Place::NotFile;
    }
    match fs::canonicalize(file) {
        Ok(real) if real.is_file() => match read(&real, bin) {
            own @ Place::File { own: true, .. } => own,
            _ => Place::Linked { to: real },
        },
        Ok(real) => Place::Linked { to: real },
        Err(_) => Place::Linked {
            to: fs::read_link(file).unwrap_or_default(),
        },
    }
}

/// Returns the file at `path`, which is a file, with its text.
fn read(path: &Path, bin: &str) -> Place {
    match fs::read(path) {
        Ok(bytes) => {
            let text = String::from_utf8_lossy(&bytes).into_owned();
            let own = written_by(&text, bin);
            Place::File {
                path: path.to_owned(),
                text,
                own,
            }
        }
        Err(error) => Place::Unreadable {
            reason: error.to_string(),
        },
    }
}
