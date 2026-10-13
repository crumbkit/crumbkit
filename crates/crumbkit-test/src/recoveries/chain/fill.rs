//! The words of a chain's steps: the value of each placeholder, the words
//! of a run with `--json`, and the path of the command that words name.

use clap::Command;

/// Returns the value of the placeholder `name`: the start's own value in
/// `own` whose name matches exactly, then the field in `fields` whose name
/// matches with capitals lowered and `-` read as `_`, then the shared value
/// in `shared` whose name matches exactly, or `None`. A value from a field
/// comes with the field's name.
//
// RUST-10: a field fills any placeholder whose name matches it, whatever
// the step means by that placeholder. crumbkit's usage failure names the
// refused `value`, so a recovery holding `<VALUE>` would run the refused
// value again. No recovery that crumbkit offers for its own failures holds
// such a placeholder today. A tool whose recovery does sets its own value
// with `Failing::values`, which wins over the field. Every fill from a
// field is a finding of `recoveries/field-value`, so a tool sees each one.
pub(super) fn value<'v>(
    own: &'v [(&str, &'v str)],
    fields: &'v [(String, String)],
    shared: &'v [(&str, &'v str)],
    name: &str,
) -> Option<(&'v str, Option<&'v str>)> {
    let exact = |table: &'v [(&str, &'v str)]| {
        table
            .iter()
            .find(|(key, _)| *key == name)
            .map(|&(_, value)| value)
    };
    let field_name = name.to_ascii_lowercase().replace('-', "_");
    exact(own)
        .map(|value| (value, None))
        .or_else(|| {
            fields
                .iter()
                .find(|(key, _)| *key == field_name)
                .map(|(key, value)| (value.as_str(), Some(key.as_str())))
        })
        .or_else(|| exact(shared).map(|value| (value, None)))
}

/// Returns `words` with `--json` added at the end when they lack it.
pub(super) fn with_json(words: &[String]) -> Vec<String> {
    let mut args = words.to_vec();
    if !args.iter().any(|word| word == "--json") {
        args.push("--json".to_owned());
    }
    args
}

/// Returns `word` with each placeholder replaced by the value that `lookup`
/// gives its name, or the first placeholder that `lookup` gives no value,
/// such as `<capability>`.
///
/// A placeholder is `<`, a name of one or more ASCII letters of either
/// case, digits, `_` and `-`, and `>`, anywhere in the word, so
/// `<plugin>.<capability>` holds two placeholders and keeps the `.` between
/// them, and `--to=<path>` keeps `--to=`. A `...` right after a placeholder
/// at the end of the word is clap's mark of a repeated value, which the
/// filled word drops. A `<` that starts no placeholder stays as it is.
//
// RUST-10: `split` returns plain words, so the check cannot
// tell a quoted value from a placeholder. A call whose real value holds
// such text, such as `'<ID>'` or `"a<path>b"`, reads as holding a
// placeholder. A call that kept its placeholders apart from its values
// replaces this if a tool needs one.
pub(in crate::recoveries) fn fill_word<'v>(
    word: &str,
    mut lookup: impl FnMut(&str) -> Option<&'v str>,
) -> Result<String, String> {
    let mut filled = String::with_capacity(word.len());
    let mut rest = word;
    while let Some(open) = rest.find('<') {
        filled.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let length = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
            .unwrap_or(after.len());
        if length == 0 || !after[length..].starts_with('>') {
            filled.push('<');
            rest = after;
            continue;
        }
        let name = &after[..length];
        let Some(value) = lookup(name) else {
            return Err(format!("<{name}>"));
        };
        filled.push_str(value);
        rest = &after[length + 1..];
        if rest == "..." {
            rest = "";
        }
    }
    filled.push_str(rest);
    Ok(filled)
}

/// Returns the path below `root` of the deepest command that `words` name
/// from their start, with its names joined by single spaces, such as
/// `books show` for `books show 7`.
pub(super) fn path_below<S: AsRef<str>>(root: &Command, words: &[S]) -> String {
    let mut command = root;
    let mut names = Vec::new();
    for word in words {
        let Some(sub) = command.find_subcommand(word.as_ref()) else {
            break;
        };
        names.push(sub.get_name());
        command = sub;
    }
    names.join(" ")
}
