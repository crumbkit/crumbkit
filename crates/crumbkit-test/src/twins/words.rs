//! The rules that a twin's two lines keep before the check runs them: they
//! differ by one word, the admitted line does not skip the command's work,
//! and both lines run the same command of the guide.

use crumbkit::help::{Guide, Leaf};

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

/// Returns the path of the leaf of `guide` that `words`, the words after
/// the tool's name, run: the longest leaf path that `words` start with, as
/// the example check builds a command line from a leaf's path and an
/// example's words. A guide joined with runtime leaves lists them too.
//
// RUST-10: the leaf is found from the leading words alone, so a line that
// puts a global flag before the command's path, such as `--json books
// list`, runs no leaf of the guide here. The clap tree would resolve such a
// line, and `check_twin` takes no tree. A twin writes its flags after the
// path.
pub(super) fn leaf(guide: &Guide, words: &[&str]) -> Option<&'static [&'static str]> {
    guide
        .get_leaves()
        .iter()
        .map(Leaf::path)
        .filter(|path| words.starts_with(path))
        .max_by_key(|path| path.len())
}
