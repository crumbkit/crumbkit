//! The shells that completion serves, the shell a run chooses, and the file
//! that `completion install` writes for each (ADR-0195).

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::{Path, PathBuf};

use clap::ValueEnum;
use serde::{Serialize, Serializer};

use crate::files::{self, absolute};
use crate::{Code, Failure, Recovery, codes};

#[cfg(test)]
mod tests;

/// A shell that `clap_complete` writes a registration script for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub(crate) enum Shell {
    /// Bash, which loads a completion file from bash-completion's user
    /// directory.
    Bash,
    /// Elvish, which has no completion directory.
    Elvish,
    /// Fish, which loads a completion file from its completions directory.
    Fish,
    /// PowerShell, which has no completion directory.
    Powershell,
    /// Zsh, which loads a completion function from a directory on `fpath`.
    Zsh,
}

impl Shell {
    /// Returns the shell's name as `--shell` takes it and `clap_complete`
    /// names it, such as `fish`.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::Elvish => "elvish",
            Self::Fish => "fish",
            Self::Powershell => "powershell",
            Self::Zsh => "zsh",
        }
    }
}

impl fmt::Display for Shell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Serializes the shell as its name, such as `"fish"`. crumbkit writes its
/// `Serialize` impls by hand, so it needs no `serde_derive`.
impl Serialize for Shell {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.name())
    }
}

/// The environment variables that choose the shell and its completion
/// directory.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Env {
    /// The home directory, from [`files::home`].
    pub(crate) home: Option<PathBuf>,
    /// The value of `SHELL`.
    pub(crate) shell: Option<OsString>,
    /// `XDG_CONFIG_HOME`, when it is an absolute path.
    pub(crate) config_home: Option<PathBuf>,
    /// `XDG_DATA_HOME`, when it is an absolute path.
    pub(crate) data_home: Option<PathBuf>,
    /// The value of `FPATH`, which zsh exports only when the person's
    /// startup file exports it.
    pub(crate) fpath: Option<OsString>,
}

impl Env {
    /// Returns the process's environment. It never asks the operating
    /// system for the home directory, so a run with a cleared environment
    /// writes nowhere outside the directories that its variables name.
    pub(crate) fn read() -> Self {
        #[expect(
            clippy::disallowed_methods,
            reason = "the completion module owns the reads of SHELL, XDG_CONFIG_HOME, XDG_DATA_HOME and FPATH that find the shell and its completion directory"
        )]
        let read = |name: &str| std::env::var_os(name);
        Self {
            home: files::home(),
            shell: read("SHELL"),
            config_home: absolute(read("XDG_CONFIG_HOME")),
            data_home: absolute(read("XDG_DATA_HOME")),
            fpath: read("FPATH"),
        }
    }
}

/// Returns the shell that a run serves: the one that `--shell` names, or
/// the one whose file stem `SHELL` holds, such as `zsh` for `/bin/zsh`.
pub(crate) fn chosen(
    flag: Option<Shell>,
    variable: Option<&OsString>,
) -> Result<Shell, ShellError> {
    if let Some(shell) = flag {
        return Ok(shell);
    }
    let variable = variable
        .filter(|value| !value.is_empty())
        .ok_or(ShellError::Unset)?;
    let name = Path::new(variable)
        .file_stem()
        .unwrap_or(OsStr::new(""))
        .to_string_lossy();
    Shell::from_str(&name, false).map_err(|_| ShellError::Unknown {
        name: name.into_owned(),
    })
}

/// The file that `completion install` writes, and a note that the person
/// needs before the shell loads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Target {
    /// The completion file's path.
    pub(crate) file: PathBuf,
    /// What the person adds to their startup file, when the shell does not
    /// yet load the file's directory.
    pub(crate) note: Option<String>,
}

/// Returns the completion file of the command `bin` for `shell`, or `None`
/// when the environment names no home directory and no XDG directory that
/// would hold it.
///
/// - Fish: `$XDG_CONFIG_HOME/fish/completions/<bin>.fish`, with
///   `XDG_CONFIG_HOME` defaulting to `~/.config`.
/// - Bash: `$XDG_DATA_HOME/bash-completion/completions/<bin>`, with
///   `XDG_DATA_HOME` defaulting to `~/.local/share`.
/// - Zsh: `<dir>/_<bin>` in the first directory of `FPATH` that is inside
///   the home directory and exists, or in `~/.zfunc` with a note that says
///   how to add it to `fpath`.
///
/// Elvish and PowerShell have no completion directory, so `install` and
/// `check` refuse them.
pub(crate) fn target(shell: Shell, bin: &str, env: &Env) -> Result<Option<Target>, ShellError> {
    let under_home = |relative: &str| env.home.as_ref().map(|home| home.join(relative));
    let plain = |file: Option<PathBuf>| file.map(|file| Target { file, note: None });
    match shell {
        Shell::Fish => Ok(plain(
            env.config_home
                .clone()
                .or_else(|| under_home(".config"))
                .map(|dir| {
                    dir.join("fish")
                        .join("completions")
                        .join(format!("{bin}.fish"))
                }),
        )),
        Shell::Bash => Ok(plain(
            env.data_home
                .clone()
                .or_else(|| under_home(".local/share"))
                .map(|dir| dir.join("bash-completion").join("completions").join(bin)),
        )),
        Shell::Zsh => Ok(env
            .home
            .as_deref()
            .map(|home| zsh(home, env.fpath.as_deref(), bin))),
        Shell::Elvish | Shell::Powershell => Err(ShellError::NoDirectory { shell }),
    }
}

/// Returns the zsh completion file of `bin` for the home directory `home`
/// and the value of `FPATH`, as [`target`] describes.
//
// RUST-10: zsh may load completions from a directory outside the home
// directory that the person can write, such as Homebrew's site-functions.
// Telling whether the person can write a directory needs `access(2)`, which
// the standard library does not offer, and writing a person's completion
// into a system directory would surprise them, so install chooses only
// directories inside the home directory. An access check through rustix
// replaces this once a person asks for those directories.
fn zsh(home: &Path, fpath: Option<&OsStr>, bin: &str) -> Target {
    let name = format!("_{bin}");
    let on_fpath = fpath
        .map(|fpath| std::env::split_paths(fpath).collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .find(|dir| dir.is_absolute() && dir.starts_with(home) && dir.is_dir());
    if let Some(dir) = on_fpath {
        return Target {
            file: dir.join(name),
            note: None,
        };
    }
    let dir = home.join(".zfunc");
    let note = format!(
        "zsh loads completions only from a directory on fpath; add `fpath=({} $fpath)` before `compinit` in ~/.zshrc.",
        files::display(&dir)
    );
    Target {
        file: dir.join(name),
        note: Some(note),
    }
}

/// A shell that completion cannot serve.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ShellError {
    /// No `--shell` was given and `SHELL` is unset or empty.
    Unset,
    /// `SHELL` names a shell that `clap_complete` does not know.
    Unknown {
        /// The shell's file name, such as `tcsh`.
        name: String,
    },
    /// `install` or `check` was asked for a shell without a completion
    /// directory.
    NoDirectory {
        /// The shell.
        shell: Shell,
    },
}

/// Prints the message, such as `SHELL names tcsh, which completion does not
/// serve`.
impl fmt::Display for ShellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unset => {
                f.write_str("SHELL is unset, so completion cannot tell which shell to serve")
            }
            Self::Unknown { name } => {
                write!(f, "SHELL names {name}, which completion does not serve")
            }
            Self::NoDirectory { shell } => {
                write!(f, "{shell} has no completion directory that install knows")
            }
        }
    }
}

impl std::error::Error for ShellError {}

impl Failure for ShellError {
    fn code(&self) -> &'static Code {
        match self {
            Self::Unset => &codes::SHELL_UNSET,
            Self::Unknown { .. } => &codes::SHELL_UNKNOWN,
            Self::NoDirectory { .. } => &codes::SHELL_NO_DIRECTORY,
        }
    }

    fn recovery(&self) -> Recovery {
        match self {
            Self::Unset | Self::Unknown { .. } => {
                Recovery::hint("name the shell with --shell: bash, elvish, fish, powershell or zsh")
            }
            Self::NoDirectory { shell } => Recovery::hint(format!(
                "load the script that completion print --shell {shell} prints from the shell's startup file"
            )),
        }
    }
}
