//! The real names that a refusal suggests in place of the names that clap
//! suggests, which can be hidden aliases.

use clap::Command;

/// The start of clap's tip that suggests one flag.
const ARGUMENT: &str = "a similar argument exists: ";
/// The start of clap's tip that suggests one subcommand.
const SUBCOMMAND: &str = "a similar subcommand exists: ";
/// The start of clap's tip that suggests several subcommands.
const SUBCOMMANDS: &str = "some similar subcommands exist: ";

/// Returns `part`, one part of a refusal's message, with each name that
/// clap's tip suggests replaced by the real name that it belongs to, and
/// each repeated name left out. `commands` holds the root and each command
/// on the walked path, so the last is the command that refused the line.
///
/// clap suggests from every name that a flag or a subcommand accepts, so it
/// can suggest a hidden alias, such as `--max` for `--maxx` when `--max` is
/// a hidden alias of `--limit`. A suggested flag becomes the long name of
/// the argument of the last command, or of a global argument of a command
/// above it, that has that long name or alias. A suggested subcommand
/// becomes the name of the subcommand of the last command that has that
/// name or alias. A part that is no such tip, and a name that resolves to
/// nothing, stay as clap wrote them.
//
// RUST-10: this reads clap's wording of the tip, as `message` does, because
// clap gives the suggestions through `Error::get` only with its
// `error-context` feature, which a tool chooses. It misses a suggestion when
// clap changes the wording, and it leaves clap's other tip, which names a
// flag of another subcommand as in `'books --limit' exists`, as clap wrote
// it. Reading `ContextKind::SuggestedArg` and
// `ContextKind::SuggestedSubcommand` through `Error::get` replaces it if
// crumbkit enables that feature.
pub(super) fn real_names(part: &str, commands: &[&Command]) -> String {
    let Some(last) = commands.last() else {
        return part.to_owned();
    };
    if let Some(names) = part.strip_prefix(ARGUMENT).and_then(quoted) {
        let names = resolve(names, |name| real_flag(name, commands));
        return format!("{ARGUMENT}{}", list(&names));
    }
    let names = part
        .strip_prefix(SUBCOMMAND)
        .or_else(|| part.strip_prefix(SUBCOMMANDS))
        .and_then(quoted);
    let Some(names) = names else {
        return part.to_owned();
    };
    let names = resolve(names, |name| {
        last.find_subcommand(name)
            .map(|subcommand| subcommand.get_name().to_owned())
    });
    let start = if names.len() == 1 {
        SUBCOMMAND
    } else {
        SUBCOMMANDS
    };
    format!("{start}{}", list(&names))
}

/// Returns the names in `text`, a list such as `'books', 'book'`, or `None`
/// when a name in it is not in single quotes.
fn quoted(text: &str) -> Option<Vec<&str>> {
    text.split(", ")
        .map(|name| name.strip_prefix('\'')?.strip_suffix('\''))
        .collect()
}

/// Returns each of `names` as `real` names it, or as it is when `real`
/// returns `None`, without repeats and in the order of their first
/// appearance.
fn resolve(names: Vec<&str>, real: impl Fn(&str) -> Option<String>) -> Vec<String> {
    let mut resolved: Vec<String> = Vec::new();
    for name in names {
        let name = real(name).unwrap_or_else(|| name.to_owned());
        if !resolved.contains(&name) {
            resolved.push(name);
        }
    }
    resolved
}

/// Returns the long name, with its `--`, of the argument that the flag
/// `name` names by its long name or an alias: an argument of the last of
/// `commands`, or else a global argument of a command above it, nearest
/// first, as clap copies global arguments down.
fn real_flag(name: &str, commands: &[&Command]) -> Option<String> {
    let long = name.strip_prefix("--")?;
    let (last, above) = commands.split_last()?;
    let names_flag = |arg: &&clap::Arg| {
        arg.get_long() == Some(long)
            || arg
                .get_all_aliases()
                .is_some_and(|aliases| aliases.contains(&long))
    };
    let own = last.get_arguments().find(names_flag);
    let arg = own.or_else(|| {
        above
            .iter()
            .rev()
            .flat_map(|command| command.get_arguments())
            .filter(|arg| arg.is_global_set())
            .find(names_flag)
    })?;
    Some(format!("--{}", arg.get_long()?))
}

/// Returns `names` in single quotes, separated by commas, as clap lists
/// them.
fn list(names: &[String]) -> String {
    names
        .iter()
        .map(|name| format!("'{name}'"))
        .collect::<Vec<_>>()
        .join(", ")
}
