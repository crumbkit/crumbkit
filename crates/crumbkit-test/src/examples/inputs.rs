//! The standard input that the example check gives each example and the
//! values that it redacts in each output.

use std::fs;
use std::path::Path;

use crumbkit::help::Guide;
use snapbox::Redactions;

use super::example_golden_name;
use crate::guide::command_line;
use crate::runs::Xdg;

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
/// example of `guide` names, whether the check runs that example or an
/// opt-out skips it. The check never deletes such a file, because it holds
/// input that only its author can write again.
pub(super) fn stale_stdin(tool: &str, guide: &Guide, dir: &Path, problems: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut owners = Vec::new();
    for leaf in guide.get_leaves() {
        let text = std::iter::once(tool)
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
            "the input file `{}` belongs to no example in the guide, and the check never deletes it",
            file.display()
        ));
    }
}

/// Returns the redactions of a run: the path of each of the run's XDG
/// directories in `xdg` under its placeholder, such as `[XDG_CONFIG_HOME]`.
pub(super) fn redactions(xdg: &Xdg) -> Redactions {
    let mut run = Redactions::new();
    // The placeholders are fixed and valid, so `insert` cannot fail.
    for (placeholder, path) in xdg.placeholders() {
        let _ = run.insert(placeholder, path.display().to_string());
    }
    run
}
