//! `RenderCall`, which renders a [`Call`] against the command tree a tool
//! builds at runtime.

use std::fmt::Write as _;

use clap::{Arg, Command};

use crate::help::words::quote;
use crate::{Call, CallArg, CallError, CallTarget};

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
    /// values or `--author <AUTHOR>` for an option, or with its value, such
    /// as `--author 'Le Guin'`.
    ///
    /// `command` is the root of the tool's tree. `render` first calls
    /// `Command::build`, which clap requires before a command's arguments are
    /// read and which does nothing to a command already built. Each path
    /// segment is looked up with `Command::find_subcommand`, so an alias
    /// resolves and prints as the subcommand's name. Each argument is looked
    /// up on the final command, which after the build also holds the global
    /// arguments of the commands above it. A value that a POSIX shell would
    /// split or expand prints between single quotes.
    ///
    /// # Errors
    ///
    /// Returns a [`CallError`] naming the first path segment or argument that
    /// is missing or hidden, or the first value that the argument would
    /// refuse: a value for a flag, a value outside the argument's possible
    /// values, or a positional value that clap would read as a flag. A hidden
    /// command or argument counts as unresolved, because crumbkit never
    /// prints what the tool hid.
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
/// `Call::help` or `Call::typed` is rendered without a lookup and needs no
/// build.
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
/// A call from `Call::typed` renders the caller's words in the same way,
/// each quoted when a POSIX shell would split or expand it.
pub(crate) fn render_built(call: &Call, command: &Command) -> Result<String, CallError> {
    let mut text = command.get_name().to_owned();
    let (path, args) = match call.target() {
        CallTarget::Help(path) => {
            for name in path {
                text.push(' ');
                text.push_str(name);
            }
            text.push_str(" -h");
            return Ok(text);
        }
        CallTarget::Typed(words) => {
            for word in words {
                text.push(' ');
                text.push_str(&quote(word));
            }
            return Ok(text);
        }
        CallTarget::Command { path, args } => (*path, args),
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

    // Each error names the command without the arguments before it, so the
    // loop reads the text up to `command_end`.
    let command_end = text.len();
    for call_arg in args {
        let (id, value) = match call_arg {
            CallArg::Placeholder(id) => (*id, None),
            CallArg::Value { id, value } => (*id, Some(value.as_str())),
        };
        let arg = find_arg(current, &text[..command_end], id)?;
        match value {
            // `Arg`'s `Display` writes the argument as clap's usage line
            // does, without styling. Writing to a `String` cannot fail.
            None => {
                let _ = write!(text, " {arg}");
            }
            Some(value) => {
                let rendered = render_value(arg, &text[..command_end], value)?;
                text.push(' ');
                text.push_str(&rendered);
            }
        }
    }
    Ok(text)
}

/// Returns the words of the command line that `call` names, from the
/// program name on, as a shell passes the line that
/// [`RenderCall::render`] prints, or `None` when `call` holds a placeholder
/// from `Call::arg`, which names no value to parse. `command` is the root
/// of a tree that `call` already rendered against, so each lookup finds what
/// the rendering found, and `None` also covers a lookup that would fail.
pub(crate) fn words(call: &Call, command: &Command) -> Option<Vec<String>> {
    let mut words = vec![command.get_name().to_owned()];
    let (path, args) = match call.target() {
        CallTarget::Help(path) => {
            words.extend(path.iter().cloned());
            words.push("-h".to_owned());
            return Some(words);
        }
        CallTarget::Typed(typed) => {
            words.extend(typed.iter().cloned());
            return Some(words);
        }
        CallTarget::Command { path, args } => (*path, args),
    };
    let mut current = command;
    for &name in path {
        current = current.find_subcommand(name)?;
        words.push(current.get_name().to_owned());
    }
    for call_arg in args {
        let CallArg::Value { id, value } = call_arg else {
            return None;
        };
        let arg = current.get_arguments().find(|arg| arg.get_id() == id)?;
        let hyphen = value.len() > 1 && value.starts_with('-');
        let flag = match (arg.get_long(), arg.get_short()) {
            (Some(long), _) => format!("--{long}"),
            (None, Some(short)) => format!("-{short}"),
            (None, None) => {
                words.push(value.clone());
                continue;
            }
        };
        if hyphen {
            words.push(format!("{flag}={value}"));
        } else {
            words.push(flag);
            words.push(value.clone());
        }
    }
    Some(words)
}

/// Returns the visible argument of `command` whose id is `id`. `text` is the
/// call's text up to the command, which each error names.
fn find_arg<'a>(command: &'a Command, text: &str, id: &str) -> Result<&'a Arg, CallError> {
    let Some(arg) = command.get_arguments().find(|arg| arg.get_id() == id) else {
        return Err(CallError::MissingArgument {
            command: text.to_owned(),
            id: id.to_owned(),
        });
    };
    if arg.is_hide_set() {
        return Err(CallError::HiddenArgument {
            command: text.to_owned(),
            id: id.to_owned(),
        });
    }
    Ok(arg)
}

/// Returns `arg` with `value` as the caller types it: `--author 'Le Guin'`
/// for an option, with its short name when it has no long name, and the
/// value alone for a positional. `text` is the call's text up to the
/// command, which each error names.
///
/// An option value that starts with `-` follows an `=`, such as
/// `--author=-x`, because clap reads a separate word that starts with `-` as
/// a flag. A positional value that starts with `-` is refused unless the
/// argument allows hyphen values, or allows negative numbers and the value is
/// one. A lone `-` is a value to clap.
///
/// RUST-10: rendering checks a value against the argument's possible values
/// only. clap keeps the argument's value parser private, so a value that the
/// parser would refuse, such as `x` for a number, still renders. Each value
/// renders as one occurrence, so two values of an option that takes several
/// values in one occurrence, such as `--ids 2 9`, render as `--ids 2 --ids
/// 9`, which clap refuses for an option that may not repeat. A call that
/// needs these reads the argument's value range and parses the value once
/// clap exposes its parser.
fn render_value(arg: &Arg, text: &str, value: &str) -> Result<String, CallError> {
    let id = arg.get_id().as_str();
    if !arg.get_action().takes_values() {
        return Err(CallError::TakesNoValue {
            command: text.to_owned(),
            id: id.to_owned(),
        });
    }
    let possible = arg.get_possible_values();
    if !possible.is_empty()
        && !possible
            .iter()
            .any(|known| known.matches(value, arg.is_ignore_case_set()))
    {
        return Err(CallError::UnknownValue {
            command: text.to_owned(),
            id: id.to_owned(),
            value: value.to_owned(),
        });
    }
    let quoted = quote(value);
    let hyphen = value.len() > 1 && value.starts_with('-');
    let flag = match (arg.get_long(), arg.get_short()) {
        (Some(long), _) => format!("--{long}"),
        (None, Some(short)) => format!("-{short}"),
        (None, None) => {
            let allowed = arg.is_allow_hyphen_values_set()
                || (arg.is_allow_negative_numbers_set() && value.parse::<f64>().is_ok());
            if hyphen && !allowed {
                return Err(CallError::HyphenValue {
                    command: text.to_owned(),
                    id: id.to_owned(),
                    value: value.to_owned(),
                });
            }
            return Ok(quoted);
        }
    };
    let separator = if hyphen { '=' } else { ' ' };
    Ok(format!("{flag}{separator}{quoted}"))
}

/// Holds the supertrait that keeps tools from implementing [`RenderCall`].
mod sealed {
    /// A type that crumbkit implements [`RenderCall`](super::RenderCall)
    /// for. The trait is public so it can bound `RenderCall`, and it sits in
    /// a private module so no tool can name it.
    pub trait Sealed {}

    impl Sealed for crate::Call {}
}
