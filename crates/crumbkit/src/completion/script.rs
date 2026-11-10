//! The registration script that `completion print` prints, and the file
//! that `completion install` writes from it, with the mark that tells
//! install the file is its own (ADR-0195).

use clap::Command;
use clap_complete::env::Shells;

use super::shell::Shell;

#[cfg(test)]
mod tests;

/// Returns the name that the person types to run the tool, which names its
/// completion file: the command's binary name, or its name when it sets
/// none.
pub(crate) fn bin(command: &Command) -> &str {
    command.get_bin_name().unwrap_or(command.get_name())
}

/// Returns the registration script of `shell` for `command`, which sets the
/// completion variable `variable` and runs the tool by the name the person
/// types, so the shell finds it on `PATH` as it does when the person runs
/// it.
///
/// The script is `clap_complete`'s dynamic registration, with that name in
/// place of the path that `COMPLETE=<shell> <tool>` writes, so an installed
/// script follows the binary that the person types and stays correct when
/// the tool's commands change.
pub(crate) fn registration(
    shell: Shell,
    variable: &str,
    command: &Command,
) -> std::io::Result<String> {
    let shells = Shells::builtins();
    let completer = shells
        .completer(shell.name())
        .ok_or_else(|| std::io::Error::other("clap_complete does not know the shell"))?;
    let bin = bin(command);
    let mut script = Vec::new();
    completer.write_registration(variable, command.get_name(), bin, bin, &mut script)?;
    Ok(String::from_utf8_lossy(&script).into_owned())
}

/// Returns the mark line that names `bin`, which install writes into its
/// file.
fn mark(bin: &str) -> String {
    format!("# {bin} completion install wrote this file, and the next install replaces it.")
}

/// Returns the text that install writes for `shell` from the registration
/// script `script` of `command`: the script with the mark line first, or
/// second after zsh's `#compdef` line, which must stay first.
///
/// The zsh file also ends with a call of the completion function that the
/// script defines. Zsh runs the file as the function `_<tool>` on the first
/// Tab press of a session, and the script only defines the function and
/// registers it, so without the call that first press offers nothing. A
/// script that `print` prints is sourced in place of autoloaded, so it has
/// no such call.
pub(crate) fn installed(shell: Shell, command: &Command, script: &str) -> String {
    let mark = mark(bin(command));
    match (shell, script.split_once('\n')) {
        (Shell::Zsh, Some((first, rest))) if first.starts_with("#compdef") => {
            // `clap_complete` names the zsh function after the command's name
            // with each dash replaced.
            let function = command.get_name().replace('-', "_");
            format!("{first}\n{mark}\n{rest}_clap_dynamic_completer_{function} \"$@\"\n")
        }
        _ => format!("{mark}\n{script}"),
    }
}

/// Reports whether `text` is a file that install wrote for `bin`: its first
/// or second line is the mark that names `bin`.
pub(crate) fn written_by(text: &str, bin: &str) -> bool {
    let mark = mark(bin);
    text.lines().take(2).any(|line| line == mark)
}
