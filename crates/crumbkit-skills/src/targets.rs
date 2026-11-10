//! The agent skills directories that `install` and `check` act on, and
//! what each holds at the skill's folder.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

/// The directories that a run reads from its environment: the home
/// directory and the Codex home.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Homes {
    /// The home directory, or `None` when the environment names none.
    home: Option<PathBuf>,
    /// `CODEX_HOME`, or `None` when it is unset, empty or relative.
    codex_home: Option<PathBuf>,
}

impl Homes {
    /// Returns the homes that the process's environment names.
    ///
    /// It reads `HOME`, and on Windows `USERPROFILE` when `HOME` is unset,
    /// and `CODEX_HOME`. It never asks the operating system for the home
    /// directory, such as with `std::env::home_dir`, which reads the
    /// password database when `HOME` is unset. A test or an example run
    /// clears the environment, and that lookup would then install into the
    /// real home (ADR-0027).
    pub(crate) fn from_env() -> Self {
        #[expect(
            clippy::disallowed_methods,
            reason = "this module owns the environment reads that find the agent skills directories"
        )]
        let read = |name: &str| std::env::var_os(name);
        let home = read("HOME");
        #[cfg(windows)]
        let home = home.or_else(|| read("USERPROFILE"));
        Self::new(home, read("CODEX_HOME"))
    }

    /// Returns the homes when the home directory is `home` and `CODEX_HOME`
    /// is `codex_home`. Each counts only when it is an absolute path.
    pub(crate) fn new(home: Option<OsString>, codex_home: Option<OsString>) -> Self {
        let absolute =
            |value: Option<OsString>| value.map(PathBuf::from).filter(|path| path.is_absolute());
        Self {
            home: absolute(home),
            codex_home: absolute(codex_home),
        }
    }

    /// Returns the agent skills directories in the order `install` tries
    /// them: `~/.claude/skills`, `~/.agents/skills` and
    /// `$CODEX_HOME/skills`, where `CODEX_HOME` defaults to `~/.codex`. It
    /// returns `None` when there is no home directory.
    pub(crate) fn agent_dirs(&self) -> Option<Vec<PathBuf>> {
        let home = self.home.as_ref()?;
        let codex = self
            .codex_home
            .clone()
            .unwrap_or_else(|| home.join(".codex"));
        Some(vec![
            home.join(".claude").join("skills"),
            home.join(".agents").join("skills"),
            codex.join("skills"),
        ])
    }
}

/// The skills directories that a run acts on.
#[derive(Debug)]
pub(crate) enum Chosen {
    /// The directory that `--dir` names, or the agent skills directories
    /// that exist, merged by real path.
    Targets(Vec<Target>),
    /// The directory that `--dir` names, which does not exist.
    NamedMissing(PathBuf),
    /// No agent skills directory exists; it holds the paths tried.
    NoAgents(Vec<PathBuf>),
    /// The environment names no home directory.
    NoHome,
}

/// Returns the skills directories that a run with `--dir` set to `dir`
/// acts on, when the environment gives `homes`.
pub(crate) fn choose(dir: Option<&Path>, homes: &Homes) -> Chosen {
    if let Some(dir) = dir {
        return match real_dir(dir) {
            Some(real) => Chosen::Targets(vec![Target {
                dirs: vec![dir.to_owned()],
                real,
            }]),
            None => Chosen::NamedMissing(dir.to_owned()),
        };
    }
    let Some(dirs) = homes.agent_dirs() else {
        return Chosen::NoHome;
    };
    let targets = merge(&dirs);
    if targets.is_empty() {
        Chosen::NoAgents(dirs)
    } else {
        Chosen::Targets(targets)
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
/// replaced.
pub(crate) fn shown(path: &Path) -> String {
    path.display().to_string()
}

/// Returns the paths of `dirs` as strings for a row.
pub(crate) fn shown_all(dirs: &[PathBuf]) -> Vec<String> {
    dirs.iter().map(|dir| shown(dir)).collect()
}
