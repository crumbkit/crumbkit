//! The rules that a twin's two lines keep before the check runs them: they
//! differ by one word, the admitted line does not skip the command's work,
//! and both lines run the same command of the tool's command tree.

use clap::Command;

/// The words that make clap or crumbkit's `parse` answer without running
/// the command: help and the version.
const SKIPPING: [&str; 4] = ["-h", "--help", "-V", "--version"];

/// Reports whether `a` and `b` differ by one word added, removed or
/// replaced.
pub(super) fn one_word_apart(a: &[&str], b: &[&str]) -> bool {
    let (long, short) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    let first = long
        .iter()
        .zip(short)
        .position(|(x, y)| x != y)
        .unwrap_or(short.len());
    match long.len() - short.len() {
        0 => first < long.len() && long[first + 1..] == short[first + 1..],
        1 => long[first + 1..] == short[first..],
        _ => false,
    }
}

/// Returns the word that `admitted` adds or puts in place of a word of
/// `refused` when that word makes the tool answer without running the
/// command: `-h`, `--help`, `-V`, `--version`, or `help` as the first word.
/// The two lines differ by one word.
pub(super) fn skipping_word<'w>(refused: &[&str], admitted: &[&'w str]) -> Option<&'w str> {
    if admitted.len() < refused.len() {
        return None;
    }
    let at = admitted
        .iter()
        .zip(refused)
        .position(|(a, r)| a != r)
        .unwrap_or(refused.len());
    let word = *admitted.get(at)?;
    (SKIPPING.contains(&word) || (word == "help" && at == 0)).then_some(word)
}

/// The command that the words of a line name in the tool's command tree.
pub(super) struct Named {
    /// The real names of the subcommands from the root to the command.
    pub(super) path: Vec<String>,
    /// Whether the command has no subcommands, so that the line runs it.
    pub(super) runs: bool,
}

/// Returns the command that `words`, the words after the tool's name, name
/// in `tree`, the tool's unbuilt command tree, as
/// [`crumbkit::command_path`] reads them: it skips options and their
/// values, global options before the path included, and resolves aliases.
pub(super) fn named(tree: &Command, words: &[&str]) -> Named {
    let path = crumbkit::command_path(tree, words);
    let mut node = tree;
    for name in &path {
        // The path holds only real names of subcommands of the tree.
        if let Some(subcommand) = node.find_subcommand(name) {
            node = subcommand;
        }
    }
    Named {
        runs: !node.has_subcommands(),
        path,
    }
}
