//! `RenderCall`, which renders a [`Call`] against the command tree a tool
//! builds at runtime.

use std::fmt::Write as _;

use clap::Command;

use crate::help::leaf::quote;
use crate::{Call, CallError, CallTarget};

#[cfg(test)]
mod tests;

/// Renders a [`Call`] as text against a `clap::Command`.
///
/// `Call` lives in crumbkit-core, which has no clap, so its `render` method
/// comes from this trait. A caller imports the trait with
/// `use crumbkit::RenderCall as _;`. crumbkit implements it for `Call`, and
/// a tool cannot implement it.
///
/// ```
/// use clap::{CommandFactory, Parser, Subcommand};
/// use crumbkit::{Call, RenderCall as _};
///
/// #[derive(Parser)]
/// #[command(name = "shelf")]
/// struct Cli {
///     #[command(subcommand)]
///     command: Books,
/// }
///
/// #[derive(Subcommand)]
/// enum Books {
///     /// Show the books with these ids.
///     Show {
///         #[arg(required = true, value_name = "ID")]
///         ids: Vec<u32>,
///     },
/// }
///
/// let call = Call::new(&["show"]).arg("ids");
/// assert_eq!(call.render(&mut Cli::command()).unwrap(), "shelf show <ID>...");
/// ```
pub trait RenderCall: sealed::Sealed {
    /// Returns the call as text: the name of `command`, then the name of each
    /// subcommand on the path, then each argument in the form clap's usage
    /// line gives it, such as `<ID>...` for a positional that takes many
    /// values or `--author <AUTHOR>` for an option, then each word.
    ///
    /// `command` is the root of the tool's tree. `render` first calls
    /// `Command::build`, which clap requires before a command's arguments are
    /// read and which does nothing to a command already built. Each path
    /// segment is looked up with `Command::find_subcommand`, so an alias
    /// resolves and prints as the subcommand's name. Each argument is looked
    /// up on the final command, which after the build also holds the global
    /// arguments of the commands above it. A word that a POSIX shell would
    /// split or expand prints between single quotes.
    ///
    /// # Errors
    ///
    /// Returns a [`CallError`] naming the first path segment or argument that
    /// is missing or hidden. A hidden command or argument counts as
    /// unresolved, because crumbkit never prints what the tool hid.
    fn render(&self, command: &mut Command) -> Result<String, CallError>;
}

impl RenderCall for Call {
    fn render(&self, command: &mut Command) -> Result<String, CallError> {
        command.build();
        render_built(self, command)
    }
}

/// Builds the part of `command` that `call` needs and returns `call` as
/// text, as [`RenderCall::render`] describes. `finish` renders the `Next:`
/// line and each `Recovery::Run` call through it, so a run builds only the
/// branch that a call names and the tree's other branches stay unbuilt.
///
/// A global argument of the root resolves only when the root is already
/// built, as parsing leaves it. In a tree that never went through parsing,
/// a call into a branch below the root that names one does not resolve.
pub(crate) fn render(call: &Call, command: &mut Command) -> Result<String, CallError> {
    build_branch(call, command);
    render_built(call, command)
}

/// Builds the branch below the root of `command` that the first path
/// segment of `call` names, or the whole tree for a call to the root.
///
/// clap has no public way to build one command alone, and
/// `Command::build` builds the command it is called on and everything below
/// it. Parsing has already built the root, which copied the root's global
/// arguments into each command one level below it, so a build of that
/// command copies them down to every command of the branch. A call from
/// `Call::help` is rendered without a lookup and needs no build.
fn build_branch(call: &Call, command: &mut Command) {
    let CallTarget::Command { path, .. } = call.target() else {
        return;
    };
    match path.first() {
        Some(name) => {
            if let Some(branch) = command.find_subcommand_mut(name) {
                branch.build();
            }
        }
        None => command.build(),
    }
}

/// Returns `call` as text, as [`RenderCall::render`] does, against a command
/// whose branch for `call` is already built.
///
/// `Arg`'s `Display` panics on an option of a command that was never built,
/// because clap sets an option's number of values during the build.
///
/// A call from `Call::help` renders its path without looking it up, because
/// the path holds the real names of commands that the caller typed, and
/// crumbkit points the caller back at a hidden command that the caller
/// named.
pub(crate) fn render_built(call: &Call, command: &Command) -> Result<String, CallError> {
    let mut text = command.get_name().to_owned();
    let (path, args, words) = match call.target() {
        CallTarget::Help(path) => {
            for name in path {
                text.push(' ');
                text.push_str(name);
            }
            text.push_str(" -h");
            return Ok(text);
        }
        CallTarget::Command { path, args, words } => (*path, args, words),
    };
    let mut current = command;
    for &name in path {
        let Some(subcommand) = current.find_subcommand(name) else {
            return Err(CallError::MissingCommand {
                command: text,
                name: name.to_owned(),
            });
        };
        if subcommand.is_hide_set() {
            return Err(CallError::HiddenCommand {
                command: text,
                name: name.to_owned(),
            });
        }
        text.push(' ');
        text.push_str(subcommand.get_name());
        current = subcommand;
    }

    for &id in args {
        let Some(arg) = current.get_arguments().find(|arg| arg.get_id() == id) else {
            return Err(CallError::MissingArgument {
                command: text,
                id: id.to_owned(),
            });
        };
        if arg.is_hide_set() {
            return Err(CallError::HiddenArgument {
                command: text,
                id: id.to_owned(),
            });
        }
        // `Arg`'s `Display` writes the argument as clap's usage line does,
        // without styling. Writing to a `String` cannot fail.
        let _ = write!(text, " {arg}");
    }
    for word in words {
        text.push(' ');
        text.push_str(&quote(word));
    }
    Ok(text)
}

/// Holds the supertrait that keeps tools from implementing [`RenderCall`].
mod sealed {
    /// A type that crumbkit implements [`RenderCall`](super::RenderCall)
    /// for. The trait is public so it can bound `RenderCall`, and it sits in
    /// a private module so no tool can name it.
    pub trait Sealed {}

    impl Sealed for crate::Call {}
}
