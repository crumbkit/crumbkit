//! The completion commands: `<tool> completion install`, which writes the
//! tool's completion registration where the person's shell loads it,
//! `<tool> completion check`, which reports a registration file that is
//! missing or differs, and `<tool> completion print`, which prints the
//! registration script (ADR-0195).
//!
//! The registration script is `clap_complete`'s dynamic registration, which
//! runs the tool for each Tab press, so [`complete`](crate::complete())
//! answers it from the tool's current commands and an installed file stays
//! correct when they change. Install writes no startup file, such as
//! `~/.zshrc`, and acts on one shell: the one that `--shell` names, or the
//! one whose name `SHELL` holds.
//!
//! The tool owns the commands. It adds a `completion` group with `install`,
//! `check` and `print` subcommands that take [`CompletionInstall`],
//! [`CompletionCheck`] and [`CompletionPrint`], and its handlers pass the
//! function that builds its command, as it passes it to `complete`, and the
//! results go to [`finish`](crate::finish) with the documents
//! [`COMPLETION_INSTALL`], [`COMPLETION_CHECK`] and [`COMPLETION_PRINT`]:
//!
//! ```
//! use std::process::ExitCode;
//!
//! use clap::{CommandFactory, Parser, Subcommand};
//! use crumbkit::Tool;
//! use crumbkit::completion::{
//!     self, COMPLETION_CHECK, COMPLETION_INSTALL, COMPLETION_PRINT, CompletionCheck,
//!     CompletionInstall, CompletionPrint,
//! };
//! use crumbkit::help::{Help, Leaf};
//! use crumbkit::ViewFlags;
//!
//! /// Browse a small catalogue.
//! #[derive(Parser)]
//! #[command(name = "shelf")]
//! struct Cli {
//!     #[command(subcommand)]
//!     resource: Resource,
//!     #[command(flatten)]
//!     view: ViewFlags,
//! }
//!
//! #[derive(Subcommand)]
//! enum Resource {
//!     /// Set up shelf's shell completion.
//!     Completion {
//!         #[command(subcommand)]
//!         command: CompletionCommand,
//!     },
//! }
//!
//! #[derive(Subcommand)]
//! enum CompletionCommand {
//!     /// Install shelf's completion for the shell.
//!     Install(CompletionInstall),
//!     /// Check that shelf's completion file is current.
//!     Check(CompletionCheck),
//!     /// Print shelf's completion registration script.
//!     Print(CompletionPrint),
//! }
//!
//! const TOOL: Tool = Tool::new().help(Help::new().leaves(&[
//!     Leaf::new(&["completion", "install"]).document(COMPLETION_INSTALL),
//!     Leaf::new(&["completion", "check"]).document(COMPLETION_CHECK),
//!     Leaf::new(&["completion", "print"]).document(COMPLETION_PRINT),
//! ]));
//!
//! fn run(args: &[&str]) -> ExitCode {
//!     let (cli, run) = match crumbkit::parse_from::<Cli>(&TOOL, args) {
//!         Ok(parsed) => parsed,
//!         Err(stop) => return stop.exit_code(),
//!     };
//!     let Resource::Completion { command } = cli.resource;
//!     match command {
//!         CompletionCommand::Install(args) => {
//!             let result = completion::install(Cli::command, &args);
//!             crumbkit::finish(run, COMPLETION_INSTALL, result)
//!         }
//!         CompletionCommand::Check(args) => {
//!             let result = completion::check(Cli::command, &args);
//!             crumbkit::finish(run, COMPLETION_CHECK, result)
//!         }
//!         CompletionCommand::Print(args) => {
//!             let result = completion::print(Cli::command, &args);
//!             crumbkit::finish(run, COMPLETION_PRINT, result)
//!         }
//!     }
//! }
//!
//! assert_eq!(
//!     run(&["shelf", "completion", "print", "--shell", "fish"]),
//!     ExitCode::SUCCESS
//! );
//! ```

mod check;
mod install;
mod place;
mod rows;
mod script;
mod shell;

use clap::{Args, Command};

pub use self::rows::{CheckRow, InstallRow, Script};
use self::shell::{Env, Shell, chosen};
use crate::{Document, Error, Listing, Prose};

/// The document that `completion install` prints: the row of the shell's
/// completion file. Its schema is
/// `crates/crumbkit/schemas/completion-install-v1.json`.
pub const COMPLETION_INSTALL: Document<Listing<InstallRow>> = Document::new(
    "crumbkit",
    "completion-install",
    1,
    "What install did with the shell's completion file: written, unchanged, skipped or failed.",
)
.when_empty("no home directory to install into; set HOME");

/// The document that `completion check` prints: the row of the shell's
/// completion file, which the text lists only when the file is missing,
/// differs or cannot be read. Its schema is
/// `crates/crumbkit/schemas/completion-check-v1.json`.
pub const COMPLETION_CHECK: Document<Listing<CheckRow>> = Document::new(
    "crumbkit",
    "completion-check",
    1,
    "Whether the shell's completion file holds the script that install writes: current, linked, missing, differs or unreadable.",
)
.when_empty("no home directory to check; set HOME")
.only_problems();

/// The document that `completion print` prints: the shell's registration
/// script. Its schema is `crates/crumbkit/schemas/completion-print-v1.json`.
pub const COMPLETION_PRINT: Document<Prose<Script>> = Document::new(
    "crumbkit",
    "completion-print",
    1,
    "The shell's registration script for the tool's completion.",
)
.always_print_first();

/// The arguments of `<tool> completion install`: the shell, and whether to
/// replace a file or a link that install did not write.
#[derive(Args, Clone, Debug, Default, PartialEq, Eq)]
// The doc comment is rustdoc, and clap would copy it into the help of a
// command without a description of its own.
#[command(about = None, long_about = None)]
pub struct CompletionInstall {
    /// Install for this shell in place of the one that SHELL names
    #[arg(long, value_enum, value_name = "SHELL")]
    shell: Option<Shell>,

    /// Replace a file or a link that install did not write; a link's target stays as it is
    #[arg(long)]
    force: bool,
}

/// The arguments of `<tool> completion check`: the shell.
#[derive(Args, Clone, Debug, Default, PartialEq, Eq)]
// The doc comment is rustdoc, and clap would copy it into the help of a
// command without a description of its own.
#[command(about = None, long_about = None)]
pub struct CompletionCheck {
    /// Check the file of this shell in place of the one that SHELL names
    #[arg(long, value_enum, value_name = "SHELL")]
    shell: Option<Shell>,
}

/// The arguments of `<tool> completion print`: the shell.
#[derive(Args, Clone, Debug, Default, PartialEq, Eq)]
// The doc comment is rustdoc, and clap would copy it into the help of a
// command without a description of its own.
#[command(about = None, long_about = None)]
pub struct CompletionPrint {
    /// Print the script of this shell in place of the one that SHELL names
    #[arg(long, value_enum, value_name = "SHELL")]
    shell: Option<Shell>,
}

/// Writes the registration script of the shell into the file that the
/// shell loads completions from, and returns its row, for the tool to pass
/// to [`finish`](crate::finish) with [`COMPLETION_INSTALL`]. `command`
/// builds the tool's command, as the function passed to
/// [`complete`](crate::complete()) does.
///
/// The shell is the one that `--shell` names, or the one whose file stem
/// `SHELL` holds, such as `zsh` for `/bin/zsh`. Its file is:
///
/// - Fish: `$XDG_CONFIG_HOME/fish/completions/<tool>.fish`, with
///   `XDG_CONFIG_HOME` defaulting to `~/.config`.
/// - Bash: `$XDG_DATA_HOME/bash-completion/completions/<tool>`, with
///   `XDG_DATA_HOME` defaulting to `~/.local/share`.
/// - Zsh: `_<tool>` in the first directory of `FPATH` that is inside the
///   home directory and exists, or in `~/.zfunc`. Zsh exports `FPATH` only
///   when the person's startup file does, so install usually writes
///   `~/.zfunc/_<tool>`, and the listing's note then gives the `fpath`
///   line to add before `compinit` in `~/.zshrc`.
///
/// The home directory is `HOME`, or `USERPROFILE` on Windows when `HOME` is
/// unset, and an XDG variable counts only when it is an absolute path. With
/// neither, install writes nothing and the empty note asks for `HOME`.
/// Install creates the file's directory when it does not exist, and writes
/// no startup file.
///
/// The file starts with a comment that names the tool, after zsh's
/// `#compdef` line, and install replaces only a file that starts so. It
/// skips any other file, and a symbolic link to anything other than such a
/// file, with the reason, unless `--force` is given. `--force` replaces the
/// file or the link itself and never the file that a link leads to. A link
/// to a file that install wrote is followed, and that file is updated.
///
/// Each write goes to a new temporary file in the same directory, which
/// install then renames over the file, so a shell never reads half a file.
/// A file or a link already at the temporary name makes the row `failed`,
/// and nothing is overwritten. A `failed` row makes the command exit with 1.
///
/// # Errors
///
/// Returns a failure with the code `<tool>::shell::unset` when no `--shell`
/// is given and `SHELL` is unset or empty, `<tool>::shell::unknown` when
/// `SHELL` names a shell that completion does not serve, and
/// `<tool>::shell::no_directory` for elvish or PowerShell, which have no
/// completion directory that install knows.
pub fn install(
    command: impl FnOnce() -> Command,
    args: &CompletionInstall,
) -> Result<Listing<InstallRow>, Error> {
    install::install_in(&command(), args, &Env::read(), crate::complete::variable())
}

/// Compares the shell's completion file with the text that
/// [`install`] writes, and returns its row, for the tool to pass to
/// [`finish`](crate::finish) with [`COMPLETION_CHECK`].
///
/// It reads the file that `install` writes for the same shell and follows a
/// symbolic link to a file that install wrote. A file is `current` when it
/// holds exactly what this binary's install writes, `missing` when it does
/// not exist, `differs` when it holds anything else, and `unreadable` when
/// it cannot be read or is not a file. A link to a file that install did
/// not write is `linked`, because install skips it, and is no problem. The
/// text lists only a problem row, and the command exits with 1 when there
/// is one. With no home directory, it checks nothing.
///
/// # Errors
///
/// Returns the failures of [`install`] for the shell.
pub fn check(
    command: impl FnOnce() -> Command,
    args: &CompletionCheck,
) -> Result<Listing<CheckRow>, Error> {
    check::check_in(
        &command(),
        args.shell,
        &Env::read(),
        crate::complete::variable(),
    )
}

/// Returns the registration script of the shell, for the tool to pass to
/// [`finish`](crate::finish) with [`COMPLETION_PRINT`].
///
/// The script is the one that `COMPLETE=<shell> <tool>` prints, except that
/// it runs the tool by the name that the person types in place of the
/// binary's path. A person loads it from their startup file, such as with
/// `source <(shelf completion print --shell zsh)` in `~/.zshrc`. It serves
/// every shell that `clap_complete` serves: bash, elvish, fish, PowerShell
/// and zsh. The document always prints the script whole, even when it
/// passes the tool's output budget, because a cut script would break the
/// shell.
///
/// # Errors
///
/// Returns a failure with the code `<tool>::shell::unset` when no `--shell`
/// is given and `SHELL` is unset or empty, and `<tool>::shell::unknown`
/// when `SHELL` names a shell that completion does not serve.
pub fn print(
    command: impl FnOnce() -> Command,
    args: &CompletionPrint,
) -> Result<Prose<Script>, Error> {
    print_in(
        &command(),
        args.shell,
        &Env::read(),
        crate::complete::variable(),
    )
}

/// Returns the script that [`print`] prints, for `env` and the completion
/// variable `variable`.
fn print_in(
    command: &Command,
    shell: Option<Shell>,
    env: &Env,
    variable: &str,
) -> Result<Prose<Script>, Error> {
    let shell = chosen(shell, env.shell.as_ref())?;
    let text = script::registration(shell, variable, command).map_err(Error::unclassified)?;
    Ok(Prose::new(Script { shell, text }))
}

#[cfg(test)]
mod tests;
