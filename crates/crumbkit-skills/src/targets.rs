//! The agent skills directories that `install` and `check` act on, and
//! what each holds at the skill's folder.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crumbkit::{Listing, Row};

#[cfg(test)]
mod tests;

/// Returns the agent skills directories that the process's environment
/// names, as [`agent_dirs`] describes.
///
/// It reads `HOME`, and on Windows `USERPROFILE` when `HOME` is unset,
/// and `CODEX_HOME`. It never asks the operating system for the home
/// directory, such as with `std::env::home_dir`, which reads the password
/// database when `HOME` is unset. A test or an example run clears the
/// environment, and that lookup would then install into the real home
/// (ADR-0027).
pub(crate) fn agent_dirs_from_env() -> Option<Vec<PathBuf>> {
    #[expect(
        clippy::disallowed_methods,
        reason = "this module owns the environment reads that find the agent skills directories"
    )]
    let read = |name: &str| std::env::var_os(name);
    let home = read("HOME");
    #[cfg(windows)]
    let home = home.or_else(|| read("USERPROFILE"));
    agent_dirs(home, read("CODEX_HOME"))
}

/// Returns the agent skills directories in the order `install` tries them,
/// when the home directory is `home` and `CODEX_HOME` is `codex_home`:
/// `~/.claude/skills`, `~/.agents/skills` and `$CODEX_HOME/skills`, where
/// `CODEX_HOME` defaults to `~/.codex`. Each of `home` and `codex_home`
/// counts only when it is an absolute path. It returns `None` when there
/// is no home directory, even when `CODEX_HOME` is set.
pub(crate) fn agent_dirs(
    home: Option<OsString>,
    codex_home: Option<OsString>,
) -> Option<Vec<PathBuf>> {
    let absolute =
        |value: Option<OsString>| value.map(PathBuf::from).filter(|path| path.is_absolute());
    let home = absolute(home)?;
    let codex = absolute(codex_home).unwrap_or_else(|| home.join(".codex"));
    Some(vec![
        home.join(".claude").join("skills"),
        home.join(".agents").join("skills"),
        codex.join("skills"),
    ])
}

/// Returns the listing of a run with `--dir` set to `dir`, when the
/// environment names the agent skills directories `agent_dirs`.
///
/// A run with `--dir` acts on that directory alone, and `refused` gives the
/// row of a `--dir` that is not a directory, with the reason. Otherwise the
/// run acts on the agent skills directories that exist, merged by real
/// path, and `rows` gives the row of each target. With no home directory,
/// or when none of the agent skills directories exists, the listing is
/// empty, and the document's empty note asks for `--dir`. The rows that
/// report a problem come first (OUTPUT-3).
pub(crate) fn listing<R: Row>(
    dir: Option<&Path>,
    agent_dirs: Option<Vec<PathBuf>>,
    refused: impl FnOnce(&Path, String) -> R,
    rows: impl FnOnce(Vec<Target>) -> Vec<R>,
) -> Listing<R> {
    let mut rows = match dir {
        Some(dir) => match named_dir(dir) {
            Ok(real) => rows(vec![Target {
                dirs: vec![dir.to_owned()],
                real,
            }]),
            Err(reason) => vec![refused(dir, reason)],
        },
        None => rows(merge(&agent_dirs.unwrap_or_default())),
    };
    rows.sort_by_key(|row| !row.is_problem());
    Listing::from_all(rows, None).described("skills directories")
}

/// Returns the real path of the directory `dir` that `--dir` names, or why
/// it cannot be used: it does not exist, it is not a directory, or it
/// cannot be read.
fn named_dir(dir: &Path) -> Result<PathBuf, String> {
    match fs::metadata(dir) {
        Ok(metadata) if !metadata.is_dir() => Err("the path is not a directory".to_owned()),
        Ok(_) => fs::canonicalize(dir).map_err(|error| error.to_string()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Err("the directory does not exist".to_owned())
        }
        Err(error) => Err(error.to_string()),
    }
}

/// A skills directory at one real path, and the paths that name it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Target {
    /// The paths that resolve to this directory, in the order given.
    pub(crate) dirs: Vec<PathBuf>,
    /// The directory's real path, with every symbolic link resolved.
    pub(crate) real: PathBuf,
}

/// Returns the directories among `dirs` that exist, with the paths that
/// resolve to the same real directory merged into one target, in the order
/// of each target's first path.
pub(crate) fn merge(dirs: &[PathBuf]) -> Vec<Target> {
    let mut targets: Vec<Target> = Vec::new();
    for dir in dirs {
        let Some(real) = real_dir(dir) else {
            continue;
        };
        match targets.iter_mut().find(|target| target.real == real) {
            Some(target) => target.dirs.push(dir.clone()),
            None => targets.push(Target {
                dirs: vec![dir.clone()],
                real,
            }),
        }
    }
    targets
}

/// Returns the real path of `dir`, or `None` when it is not a directory.
pub(crate) fn real_dir(dir: &Path) -> Option<PathBuf> {
    let real = fs::canonicalize(dir).ok()?;
    real.is_dir().then_some(real)
}

/// What a skills directory holds at the skill's folder, `<dir>/<name>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Place {
    /// Nothing is at `folder`.
    Absent {
        /// The folder's path.
        folder: PathBuf,
    },
    /// A real folder is at `folder`.
    Folder {
        /// The folder's path.
        folder: PathBuf,
    },
    /// A symbolic link at `link` leads to the folder `to`, a real path.
    Linked {
        /// The link's path.
        link: PathBuf,
        /// The real path of the folder that the link leads to.
        to: PathBuf,
    },
    /// A symbolic link at `link` leads to `to`, which is not a folder or
    /// does not exist.
    Broken {
        /// The link's path.
        link: PathBuf,
        /// The path that the link holds.
        to: PathBuf,
    },
    /// Something other than a folder or a link is at `path`, such as a
    /// file.
    NotFolder {
        /// The path of what is there.
        path: PathBuf,
    },
    /// The folder's path cannot be read.
    Unreadable {
        /// The folder's path.
        path: PathBuf,
        /// Why it cannot be read.
        reason: String,
    },
}

/// Returns what the skills directory at `real` holds at `<real>/<name>`.
pub(crate) fn place(real: &Path, name: &str) -> Place {
    let folder = real.join(name);
    let metadata = match fs::symlink_metadata(&folder) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Place::Absent { folder },
        Err(error) => {
            return Place::Unreadable {
                path: folder,
                reason: error.to_string(),
            };
        }
    };
    if metadata.is_dir() {
        return Place::Folder { folder };
    }
    if !metadata.is_symlink() {
        return Place::NotFolder { path: folder };
    }
    if let Some(to) = real_dir(&folder) {
        Place::Linked { link: folder, to }
    } else {
        let to = fs::read_link(&folder).unwrap_or_default();
        Place::Broken { link: folder, to }
    }
}

/// Returns `path` as a string for a row, with any bytes that are not UTF-8
/// replaced and without the `\\?\` prefix that Windows gives a canonical
/// path.
pub(crate) fn shown(path: &Path) -> String {
    without_verbatim(path.display().to_string())
}

/// Returns `text` without a leading `\\?\` when a drive letter follows it,
/// as in `\\?\C:\Users`. Windows gives a canonical path that prefix, and
/// the path reads the same without it. A verbatim UNC path, `\\?\UNC\`,
/// keeps the prefix, because it names a share in another form without it.
/// No Unix path starts with the prefix, so this runs on every platform.
///
/// RUST-10: the dunce crate also checks that the path stays valid without
/// the prefix, such as one longer than 260 characters or one that names a
/// reserved device. This strip only changes how a row shows the path, and
/// no file access reads the result, so it skips those checks. dunce
/// replaces it if a stripped path is ever opened.
pub(crate) fn without_verbatim(text: String) -> String {
    match text.strip_prefix(r"\\?\") {
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => rest.to_owned(),
        _ => text,
    }
}

/// Returns the paths of `dirs` as strings for a row.
pub(crate) fn shown_all(dirs: &[PathBuf]) -> Vec<String> {
    dirs.iter().map(|dir| shown(dir)).collect()
}
