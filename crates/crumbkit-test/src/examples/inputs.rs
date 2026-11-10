//! The standard input that the example check gives each example and the
//! values that it redacts in each output.

use std::fs;
use std::path::Path;

use crumbkit::Tool;
use snapbox::Redactions;

use super::example_golden_name;
use crate::help::command_line;
use crate::runs::{WorkDir, Xdg};

/// Returns the name of the file whose bytes are an example's standard
/// input: the name `name` of its output's golden file with `.stdin` in
/// place of `.txt`, such as `shelf-books-list.stdin`.
pub(crate) fn stdin_name(name: &str) -> String {
    format!("{}.stdin", name.strip_suffix(".txt").unwrap_or(name))
}

/// Returns the bytes of the input file of the example whose golden file is
/// `name` in `dir`, `None` when the file does not exist, or the problem
/// when it cannot be read.
pub(crate) fn read_stdin(dir: &Path, name: &str) -> Result<Option<Vec<u8>>, String> {
    let file = dir.join(stdin_name(name));
    if !file.is_file() {
        return Ok(None);
    }
    fs::read(&file).map(Some).map_err(|error| {
        format!(
            "the input file `{}` cannot be read: {error}",
            file.display()
        )
    })
}

/// Adds a problem to `problems` for each `.stdin` file in `dir` that no
/// example of `tool` names, whether the check runs that example or an
/// opt-out skips it. The check never deletes such a file, because it holds
/// input that only its author can write again.
pub(super) fn stale_stdin(tool_name: &str, tool: &Tool, dir: &Path, problems: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut owners = Vec::new();
    for leaf in tool.get_help().get_leaves() {
        let text = std::iter::once(tool_name)
            .chain(leaf.path().iter().copied())
            .collect::<Vec<_>>()
            .join(" ");
        for example in leaf.get_examples() {
            let line = command_line(&text, example.words());
            owners.push(stdin_name(&example_golden_name(&line)));
        }
    }
    let mut stale: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|found| found == "stdin"))
        .filter(|path| {
            path.file_name()
                .is_none_or(|name| !owners.iter().any(|known| name == known.as_str()))
        })
        .collect();
    stale.sort();
    for file in stale {
        problems.push(format!(
            "the input file `{}` belongs to no example in the tool record, and the check never deletes it",
            file.display()
        ));
    }
}

/// Returns the spellings of `path` that a tool's output may hold: the text
/// of `path` itself and, when that text starts with the `\\?\` prefix that
/// Windows gives a canonical path, the text without it. A tool prints a path
/// through `crumbkit::files::display`, which drops the prefix, so the
/// redaction needs both. The strip is that function's, so the two agree on a
/// verbatim UNC path, which keeps its prefix.
pub(super) fn spellings(path: &Path) -> Vec<String> {
    let full = path.display().to_string();
    let shown = crumbkit::files::display(path);
    if shown == full {
        vec![full]
    } else {
        vec![full, shown]
    }
}

/// The redactions of one example run, with the placeholder of each
/// redacted directory.
pub(super) struct RunRedactions {
    values: Redactions,
    placeholders: Vec<&'static str>,
}

impl RunRedactions {
    /// Returns `text` with each redacted path replaced by its placeholder,
    /// and the rest of each path under a placeholder written with `/` as
    /// [`slash_paths_under`] does, so a golden written on Unix matches the
    /// output of a run on Windows.
    pub(super) fn redact(&self, text: &str) -> String {
        slash_paths_under(&self.values.redact(text), &self.placeholders)
    }
}

/// Returns the redactions of a run: the path of each of the run's XDG
/// directories in `xdg` under its placeholder, such as `[XDG_CONFIG_HOME]`,
/// and each path of the fresh directory `work` under `[ROOT]`, each in every
/// spelling that [`spellings`] returns.
pub(super) fn redactions(xdg: &Xdg, work: &WorkDir) -> RunRedactions {
    let mut values = Redactions::new();
    let mut placeholders = Vec::new();
    // The placeholders are fixed and valid, so `insert` cannot fail.
    for (placeholder, path) in xdg.placeholders() {
        placeholders.push(placeholder);
        for spelling in spellings(path) {
            let _ = values.insert(placeholder, spelling);
        }
    }
    placeholders.push("[ROOT]");
    for path in work.fresh_paths() {
        for spelling in spellings(&path) {
            let _ = values.insert("[ROOT]", spelling);
        }
    }
    RunRedactions {
        values,
        placeholders,
    }
}

/// Returns `text` with each `\` written as `/` in the rest of every path
/// that follows one of `placeholders` directly with a `\`, such as
/// `[XDG_CONFIG_HOME]\fish\shelf.fish`. The path ends at whitespace or at a
/// character that a Windows path cannot hold, such as a quote. Every other
/// `\` in `text` stays as it is.
pub(super) fn slash_paths_under(text: &str, placeholders: &[&str]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some((start, placeholder)) = placeholders
        .iter()
        .filter_map(|placeholder| rest.find(placeholder).map(|start| (start, *placeholder)))
        .min()
    {
        let after = start + placeholder.len();
        out.push_str(&rest[..after]);
        rest = &rest[after..];
        if !rest.starts_with('\\') {
            continue;
        }
        let end = rest
            .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '`' | '<' | '>' | '|'))
            .unwrap_or(rest.len());
        out.extend(rest[..end].chars().map(|c| if c == '\\' { '/' } else { c }));
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}
