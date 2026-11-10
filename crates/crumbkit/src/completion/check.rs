//! What `completion check` reports about the shell's completion file
//! (ADR-0195).

use std::path::Path;

use clap::Command;

use super::place::{Place, place};
use super::rows::{CheckRow, Checked};
use super::script::{bin, installed, registration};
use super::shell::{Env, Shell, chosen, target};
use crate::files::display;
use crate::{Error, Listing};

#[cfg(test)]
mod tests;

/// Checks the completion file of `command` as [`check`](super::check)
/// describes, for the shell that `shell` or `env` names, `env` and the
/// completion variable `variable`.
pub(crate) fn check_in(
    command: &Command,
    shell: Option<Shell>,
    env: &Env,
    variable: &str,
) -> Result<Listing<CheckRow>, Error> {
    let shell = chosen(shell, env.shell.as_ref())?;
    let bin = bin(command);
    let Some(target) = target(shell, bin, env)? else {
        return Ok(Listing::from_all(Vec::new(), None).described("completion files"));
    };
    let script = registration(shell, variable, command).map_err(Error::unclassified)?;
    let text = installed(shell, command, &script);
    let row = inspect(shell, bin, &target.file, &text);
    Ok(Listing::from_all(vec![row], None).described("completion files"))
}

/// Returns the row of `file`, the completion file of `bin` for `shell`,
/// which should hold `text`.
fn inspect(shell: Shell, bin: &str, file: &Path, text: &str) -> CheckRow {
    let row = |state, path: &Path, reason: Option<String>| CheckRow {
        state,
        shell,
        path: display(path),
        reason,
    };
    match place(file, bin) {
        Place::Absent => row(Checked::Missing, file, None),
        Place::File {
            path, text: old, ..
        } if old == text => row(Checked::Current, &path, None),
        Place::File {
            path, own: true, ..
        } => {
            let reason = format!("written by another build of {bin}; install replaces it");
            row(Checked::Differs, &path, Some(reason))
        }
        Place::File { path, .. } => {
            let reason =
                format!("not written by {bin} completion install; install --force replaces it");
            row(Checked::Differs, &path, Some(reason))
        }
        Place::Linked { to } => {
            let reason = format!("linked to {}", display(&to));
            row(Checked::Linked, file, Some(reason))
        }
        Place::NotFile => {
            let reason = format!("{} is not a file", display(file));
            row(Checked::Unreadable, file, Some(reason))
        }
        Place::Unreadable { reason } => row(Checked::Unreadable, file, Some(reason)),
    }
}
