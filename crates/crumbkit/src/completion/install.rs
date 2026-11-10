//! What `completion install` does with the shell's completion file
//! (ADR-0195).

use std::fs;
use std::io;
use std::path::Path;

use clap::Command;

use super::CompletionInstall;
use super::place::{Place, place};
use super::rows::{InstallRow, Installed};
use super::script::{bin, installed, registration};
use super::shell::{Env, Shell, chosen, target};
use crate::files::{display, write_atomically};
use crate::{Error, Listing};

#[cfg(test)]
mod tests;

/// Installs the completion of `command` as [`install`](super::install)
/// describes, for `env` and the completion variable `variable`.
pub(crate) fn install_in(
    command: &Command,
    args: &CompletionInstall,
    env: &Env,
    variable: &str,
) -> Result<Listing<InstallRow>, Error> {
    let shell = chosen(args.shell, env.shell.as_ref())?;
    let bin = bin(command);
    let Some(target) = target(shell, bin, env)? else {
        return Ok(Listing::from_all(Vec::new(), None).described("completion files"));
    };
    let script = registration(shell, variable, command).map_err(Error::unclassified)?;
    let text = installed(shell, command, &script);
    let row = write(shell, bin, &target.file, &text, args.force);
    // The note tells the person how to make the shell load a file that
    // install just wrote, so an unchanged, skipped or failed row has none.
    let note = target.note.filter(|_| row.state == Installed::Written);
    let listing = Listing::from_all(vec![row], None).described("completion files");
    Ok(match note {
        Some(note) => listing.note(note),
        None => listing,
    })
}

/// Writes `text` to `file`, the completion file of `bin` for `shell`, as
/// [`install`](super::install) describes, and returns the row.
fn write(shell: Shell, bin: &str, file: &Path, text: &str, force: bool) -> InstallRow {
    let row = |state, path: &Path, reason: Option<String>| InstallRow {
        state,
        shell,
        path: display(path),
        reason,
    };
    let written = |path: &Path, reason: Option<String>| match write_atomically(path, text) {
        Ok(()) => row(Installed::Written, path, reason),
        Err(error) => row(Installed::Failed, path, Some(error.to_string())),
    };
    let foreign = |reason: String| {
        if force {
            written(file, Some(format!("replaced: {reason}")))
        } else {
            row(Installed::Skipped, file, Some(reason))
        }
    };
    match place(file, bin) {
        Place::Absent => match create_parent(file) {
            Ok(()) => written(file, None),
            Err(error) => row(Installed::Failed, file, Some(error.to_string())),
        },
        Place::File {
            path, text: old, ..
        } if old == text => row(Installed::Unchanged, &path, None),
        Place::File {
            path, own: true, ..
        } => written(&path, None),
        Place::File { .. } => foreign(format!("not written by {bin} completion install")),
        Place::Linked { to } => foreign(format!("linked to {}", display(&to))),
        Place::NotFile => {
            let reason = format!(
                "{} is not a file, and install never removes it",
                display(file)
            );
            row(Installed::Failed, file, Some(reason))
        }
        Place::Unreadable { reason } => row(Installed::Failed, file, Some(reason)),
    }
}

/// Creates the folder of `file` and its parents when they do not exist.
fn create_parent(file: &Path) -> io::Result<()> {
    let Some(parent) = file.parent() else {
        return Ok(());
    };
    #[expect(
        clippy::disallowed_methods,
        reason = "the completion module owns the creation of the shell's completion directory, which install writes into"
    )]
    fs::create_dir_all(parent)
}
