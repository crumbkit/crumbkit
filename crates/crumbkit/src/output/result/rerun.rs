//! The recovery of a batch over the output budget as the caller's own
//! command line: the words the caller typed, with the keys left out in
//! place of the keys the caller gave.

use std::ffi::{OsStr, OsString};

use clap::{Arg, ArgMatches, Command};

use crate::{Call, CallArg, CallTarget, Rows};

#[cfg(test)]
mod tests;

/// The caller's words after the program name, split at the place of a
/// batch's keys, which the recoveries of the answers left out pass in place
/// of the keys the caller typed.
#[derive(Debug)]
pub(super) struct Rerun {
    /// The words before the place of the keys.
    before: Vec<String>,
    /// The words after the place of the keys, without the words that held
    /// the caller's keys.
    after: Vec<String>,
    /// The flag that the keys follow, such as `--name`, or `None` for a
    /// positional argument.
    flag: Option<String>,
    /// Whether one occurrence of the argument takes many values.
    many: bool,
    /// Whether a `--` comes before the keys, so clap reads a key that
    /// starts with `-` as a value.
    escaped: bool,
    /// The argument as clap's usage line shows it, such as `<NAME>...` or
    /// `--name <NAME>...`, and the field whose values the call that fetches
    /// every key left out takes, when clap read the words of that call as
    /// the caller's with other keys.
    all: Option<(String, &'static str)>,
    /// Whether clap read the words of a call that fetches one key as the
    /// caller's with that key.
    one: bool,
}

impl Rerun {
    /// Returns the caller's `words` split at the place of the keys of
    /// `value`, a batch whose command `command` parsed them, or `None` when
    /// the words do not pass the batch's keys.
    ///
    /// clap's parse of the words decides which words are the keys: the
    /// batch's call names the command and the argument that takes the keys,
    /// and the parse gives the argument's values as the caller typed them.
    /// A scan of the words after the command's name finds those values in
    /// order, each after the argument's flag for an option. clap's matches
    /// count positions in their own way, which splits `--name=a` and `-abc`,
    /// so they do not say which word holds a value.
    ///
    /// The scan can mistake the value of another argument for a key, so clap
    /// parses the words that the split builds. The split holds only when the
    /// caller's own keys at its place read as the caller's words. Each form
    /// of the recovery then holds only when its words read as the caller's
    /// with other keys: every other argument has the values it had in the
    /// caller's parse, from the same source, and the argument of the keys
    /// has the new keys. The form that fetches every key left out is read
    /// with every key of the batch that the run can leave out, and the form
    /// that fetches one key with the first of them. It returns `None` when
    /// no form holds, and the batch's recoveries name its command alone.
    ///
    /// The parses run on a clone of `command`, at most four of them, and
    /// only for a batch that leaves out an answer.
    pub(super) fn find<T: Rows>(command: &Command, words: &[OsString], value: &T) -> Option<Self> {
        let fetch = value.fetch()?;
        let CallTarget::Command { path, args } = fetch.target() else {
            return None;
        };
        let [CallArg::All { id, field }] = args.as_slice() else {
            return None;
        };
        let keys: Vec<&str> = value.left_out(0).into_iter().flatten().collect();
        let first = *keys.first()?;
        let words: Vec<String> = words
            .iter()
            .map(|word| word.to_str().map(str::to_owned))
            .collect::<Option<_>>()?;
        // Parsing needs `&mut`, and the output's command stays as `finish`
        // found it.
        let mut command = command.clone();
        let typed = parse(&mut command, &words)?;
        let leaf = leaf(&command, path)?;
        let arg = leaf.get_arguments().find(|arg| arg.get_id() == *id)?;
        let given = raw(leaf_matches(&typed, path)?, id)?;
        let start = after_path(&command, &words, path)?;
        let (held, escaped) = locate(arg, &words, start, &given)?;
        let at = *held.first()?;
        let before = words.get(..at)?.to_vec();
        let after = words
            .iter()
            .enumerate()
            .skip(at)
            .filter(|(index, _)| held.binary_search(index).is_err())
            .map(|(_, word)| word.clone())
            .collect();
        let flag = match (arg.get_long(), arg.get_short()) {
            (Some(long), _) => Some(format!("--{long}")),
            (None, Some(short)) => Some(format!("-{short}")),
            (None, None) => None,
        };
        let many = arg
            .get_num_args()
            .is_some_and(|range| range.max_values() > 1);
        let placeholder = arg.to_string();
        let mut rerun = Self {
            before,
            after,
            flag,
            many,
            escaped,
            all: None,
            one: false,
        };
        let mut reads = |words: Option<Vec<String>>, keys: &[&str]| {
            words.is_some_and(|words| {
                parse(&mut command, &words)
                    .is_some_and(|parsed| same(&command, &typed, &parsed, path, id, keys))
            })
        };
        // The caller's own keys at the place must read as the caller's
        // words, which a scan that took another argument's value for a key
        // breaks.
        let own: Vec<&str> = given.iter().flatten().map(String::as_str).collect();
        if !reads(rerun.with(&own), &own) {
            return None;
        }
        // The placeholder stands for keys after one flag, which a key that
        // starts with `-` cannot follow.
        let one_flag = rerun.flag.is_none() || !keys.iter().any(|key| starts_with_hyphen(key));
        if many && one_flag && reads(rerun.with(&keys), &keys) {
            rerun.all = Some((placeholder, *field));
        }
        rerun.one = reads(rerun.with(&[first]), &[first]);
        (rerun.all.is_some() || rerun.one).then_some(rerun)
    }

    /// Returns the call that fetches every key left out, with the
    /// argument's placeholder at the place of the keys, or `None` when clap
    /// did not read that call as the caller's words.
    pub(super) fn all(&self) -> Option<Call> {
        let (placeholder, field) = self.all.as_ref()?;
        let words = [self.before.as_slice(), self.after.as_slice()].concat();
        Some(Call::typed_all(
            words,
            self.before.len(),
            placeholder.clone(),
            field,
        ))
    }

    /// Returns the call that fetches `key` alone, or `None` when clap did
    /// not read such a call as the caller's words or would read `key` as a
    /// flag.
    pub(super) fn one(&self, key: &str) -> Option<Call> {
        if !self.one {
            return None;
        }
        self.with(&[key]).map(Call::typed)
    }

    /// Returns the words with `keys` at the place of the keys, or `None`
    /// for a positional key that starts with `-` without a `--` before the
    /// place, which clap would read as a flag.
    ///
    /// An option's keys follow one flag when an occurrence takes many
    /// values and none of the keys starts with `-`. Otherwise each key
    /// follows a flag of its own, joined to it by `=` when the key starts
    /// with `-`.
    fn with(&self, keys: &[&str]) -> Option<Vec<String>> {
        let hyphen = keys.iter().any(|key| starts_with_hyphen(key));
        let mut middle = Vec::new();
        match &self.flag {
            None if hyphen && !self.escaped => return None,
            None => middle.extend(keys.iter().map(|&key| key.to_owned())),
            Some(flag) if self.many && !hyphen => {
                middle.push(flag.clone());
                middle.extend(keys.iter().map(|&key| key.to_owned()));
            }
            Some(flag) => {
                for &key in keys {
                    if starts_with_hyphen(key) {
                        middle.push(format!("{flag}={key}"));
                    } else {
                        middle.push(flag.clone());
                        middle.push(key.to_owned());
                    }
                }
            }
        }
        Some([self.before.as_slice(), &middle, self.after.as_slice()].concat())
    }
}

/// Returns the call that fetches every key that `value` leaves out: the
/// caller's words from `rerun`, or the batch's own call without them.
pub(super) fn fetch_all<T: Rows>(value: &T, rerun: Option<&Rerun>) -> Option<Call> {
    rerun.and_then(Rerun::all).or_else(|| value.fetch())
}

/// Returns the call that fetches `key` of `value` alone: the caller's
/// words from `rerun`, or the batch's own call without them.
pub(super) fn fetch_key<T: Rows>(value: &T, key: &str, rerun: Option<&Rerun>) -> Option<Call> {
    rerun
        .and_then(|rerun| rerun.one(key))
        .or_else(|| value.fetch_key(key))
}

/// Reports whether clap reads `key` as a flag when it stands alone: it
/// starts with `-` and is longer than `-`.
fn starts_with_hyphen(key: &str) -> bool {
    key.len() > 1 && key.starts_with('-')
}

/// Parses `words`, the words after the program name, against `command`, or
/// returns `None` when clap refuses them.
fn parse(command: &mut Command, words: &[String]) -> Option<ArgMatches> {
    let name = command.get_name().to_owned();
    command
        .try_get_matches_from_mut(std::iter::once(name).chain(words.iter().cloned()))
        .ok()
}

/// Returns the command at `path`, the names of the subcommands from the
/// root of `command`.
fn leaf<'a>(command: &'a Command, path: &[&str]) -> Option<&'a Command> {
    path.iter()
        .try_fold(command, |node, name| node.find_subcommand(name))
}

/// Returns the matches of the command at `path` in `matches`, or `None`
/// when the parse named another command.
fn leaf_matches<'a>(matches: &'a ArgMatches, path: &[&str]) -> Option<&'a ArgMatches> {
    path.iter().try_fold(matches, |level, name| {
        level
            .subcommand()
            .filter(|(parsed, _)| parsed == name)
            .map(|(_, next)| next)
    })
}

/// Returns the values of the argument `id` in `matches`, each occurrence
/// as the words that held it, or `None` when the argument is absent or a
/// value is not UTF-8.
fn raw(matches: &ArgMatches, id: &str) -> Option<Vec<Vec<String>>> {
    let occurrences = matches.try_get_raw_occurrences(id).ok().flatten()?;
    occurrences
        .map(|values| {
            values
                .map(|value| value.to_str().map(str::to_owned))
                .collect()
        })
        .collect()
}

/// Returns the values of the argument `id` in `matches` as raw text, for
/// comparing two parses.
fn raw_os<'a>(matches: &'a ArgMatches, id: &str) -> Option<Vec<Vec<&'a OsStr>>> {
    let occurrences = matches.try_get_raw_occurrences(id).ok().flatten()?;
    Some(occurrences.map(Iterator::collect).collect())
}

/// Returns the index of the word after the name of the command at `path`
/// among `words`, finding each name of the path in turn before any `--`.
//
// RUST-10: the scan takes the first word that names the next subcommand,
// so an option's value that equals a subcommand's name ahead of the real
// one misplaces the start. The check in `Rerun::find` then refuses the
// split, and the recovery names the batch's command alone.
fn after_path(command: &Command, words: &[String], path: &[&str]) -> Option<usize> {
    let end = words.iter().position(|word| word == "--");
    let mut node = command;
    let mut start = 0;
    for name in path {
        let found = words
            .iter()
            .enumerate()
            .take(end.unwrap_or(words.len()))
            .skip(start)
            .find_map(|(index, word)| {
                node.find_subcommand(word)
                    .filter(|subcommand| subcommand.get_name() == *name)
                    .map(|subcommand| (index, subcommand))
            })?;
        start = found.0 + 1;
        node = found.1;
    }
    Some(start)
}

/// Returns the indexes of the words from `start` on that hold `given`, the
/// occurrences of `arg` that clap parsed, in order, and whether a `--`
/// comes before the first of them.
///
/// A positional value is the next word equal to it. An option's occurrence
/// starts at the next word before any `--` that names the option by its
/// long name, its short name or an alias, as `--name`, `--name=a`, `-n`,
/// `-n=a` or `-na`, and its other values are the words after it.
//
// RUST-10: this reads the forms above and misses a short option inside a
// cluster of short flags, such as `-vn a`, and values that clap split at a
// delimiter. Either one leaves a value unfound, and the recovery names the
// batch's command alone.
fn locate(
    arg: &Arg,
    words: &[String],
    start: usize,
    given: &[Vec<String>],
) -> Option<(Vec<usize>, bool)> {
    let end = words.iter().position(|word| word == "--");
    let options = end.unwrap_or(words.len());
    let positional = arg.get_long().is_none() && arg.get_short().is_none();
    let mut held = Vec::new();
    let mut cursor = start;
    for occurrence in given {
        let mut values = occurrence.iter();
        if !positional {
            let at = (cursor..options).find(|&index| {
                words
                    .get(index)
                    .is_some_and(|word| named(arg, word).is_some())
            })?;
            let attached = match words.get(at).and_then(|word| named(arg, word)) {
                Some(Named::With(value)) => Some(value),
                _ => None,
            };
            held.push(at);
            cursor = at + 1;
            if let Some(text) = attached
                && values.next()? != text
            {
                return None;
            }
            for value in values {
                if words.get(cursor)? != value {
                    return None;
                }
                held.push(cursor);
                cursor += 1;
            }
            continue;
        }
        for value in values {
            let at = (cursor..words.len())
                .find(|&index| Some(index) != end && words.get(index) == Some(value))?;
            held.push(at);
            cursor = at + 1;
        }
    }
    let first = *held.first()?;
    Some((held, end.is_some_and(|end| end < first)))
}

/// How a word names an option.
enum Named<'w> {
    /// The word is the option's name alone.
    Alone,
    /// The word is the option's name with its first value attached.
    With(&'w str),
}

/// Returns how `word` names the option `arg`, or `None` when it does not
/// name it.
fn named<'w>(arg: &Arg, word: &'w str) -> Option<Named<'w>> {
    if let Some(rest) = word.strip_prefix("--") {
        let (name, value) = match rest.split_once('=') {
            Some((name, value)) => (name, Some(value)),
            None => (rest, None),
        };
        let longs = arg
            .get_long()
            .into_iter()
            .chain(arg.get_all_aliases().unwrap_or_default());
        let found = longs.into_iter().any(|long| long == name);
        return found.then_some(value.map_or(Named::Alone, Named::With));
    }
    let rest = word.strip_prefix('-')?;
    let mut chars = rest.chars();
    let short = chars.next()?;
    let shorts = arg
        .get_short()
        .into_iter()
        .chain(arg.get_all_short_aliases().unwrap_or_default());
    if !shorts.into_iter().any(|known| known == short) {
        return None;
    }
    let value = chars.as_str();
    let value = value.strip_prefix('=').unwrap_or(value);
    Some(if value.is_empty() {
        Named::Alone
    } else {
        Named::With(value)
    })
}

/// Reports whether `parsed` reads as `typed` with `keys` in place of the
/// values of the argument `id` of the command at `path`: the same commands,
/// every other argument of each with the same values from the same source,
/// and the argument `id` with `keys`.
fn same(
    command: &Command,
    typed: &ArgMatches,
    parsed: &ArgMatches,
    path: &[&str],
    id: &str,
    keys: &[&str],
) -> bool {
    let mut node = command;
    let (mut left, mut right) = (typed, parsed);
    let mut rest = path;
    loop {
        let at_leaf = rest.is_empty();
        let others = node
            .get_arguments()
            .map(|arg| arg.get_id().as_str())
            .filter(|&other| !(at_leaf && other == id));
        for other in others {
            if raw_os(left, other) != raw_os(right, other)
                || left.value_source(other) != right.value_source(other)
            {
                return false;
            }
        }
        if at_leaf {
            let given = raw_os(right, id).map(|occurrences| occurrences.concat());
            let wanted: Vec<&OsStr> = keys.iter().map(OsStr::new).collect();
            return given == Some(wanted) && right.subcommand_name().is_none();
        }
        let Some((name, next)) = rest.split_first() else {
            return false;
        };
        let (Some((typed_name, typed_next)), Some((parsed_name, parsed_next))) =
            (left.subcommand(), right.subcommand())
        else {
            return false;
        };
        let Some(child) = node.find_subcommand(name) else {
            return false;
        };
        if typed_name != *name || parsed_name != *name {
            return false;
        }
        node = child;
        left = typed_next;
        right = parsed_next;
        rest = next;
    }
}
