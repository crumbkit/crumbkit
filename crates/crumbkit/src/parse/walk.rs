//! The walk, which reads the typed words down the command tree before clap
//! parses them, and finds the word `help` that crumbkit answers.

use std::ffi::{OsStr, OsString};

use clap::{Arg, Command};

#[cfg(test)]
mod tests;

/// The word `help` that crumbkit answers in place of clap.
pub(super) struct HelpWord {
    /// The position of the word in the arguments.
    pub(super) position: usize,
    /// Whether the word comes directly after the guide's root, where it
    /// can also print the tool's topics.
    pub(super) at_root: bool,
}

/// What the walk read from the typed words.
pub(super) struct Walked {
    /// The real names of the subcommands from the root to the deepest
    /// command that the words name.
    pub(super) path: Vec<String>,
    /// Whether a word reads as the flag `--help`, so clap prints long help.
    pub(super) long_help: bool,
    /// Whether a word reads as the flag `--json`, so a refusal prints as a
    /// JSON document.
    pub(super) json: bool,
}

/// Returns the word `help` that crumbkit answers in `args`, or `None` when
/// clap reads every word.
///
/// It reads the words as [`walk`] does: it skips the program name, stops at
/// `--`, skips each flag and each value of an option, and descends into each
/// word that names a subcommand of the current command, resolving aliases.
/// It answers `help` when that word comes next below a command with
/// subcommands, or directly after the words of `root`, the path of the
/// guide's root, when the tool `has_topics`, and it stops at the first word
/// that names no subcommand, such as the value of a positional argument.
///
/// A command without subcommands reads `help` as an ordinary word, unless it
/// is the guide's root of a tool with topics, and a command with its own
/// subcommand named or aliased `help` lets clap parse that subcommand.
pub(super) fn help_word(
    command: &Command,
    root: &[&str],
    has_topics: bool,
    args: &[OsString],
) -> Option<HelpWord> {
    let mut words = Words::new(command);
    let mut path: Vec<&str> = Vec::new();
    for (position, word) in args.iter().enumerate().skip(1) {
        match words.read(word) {
            Kind::End => return None,
            Kind::Flag | Kind::Value => continue,
            Kind::Name => {}
        }
        let current = words.current;
        match current.find_subcommand(word) {
            Some(subcommand) => {
                path.push(subcommand.get_name());
                words.enter(subcommand);
            }
            None if word == "help" => {
                let at_root = path == root;
                if !(current.has_subcommands() || at_root && has_topics) {
                    return None;
                }
                return Some(HelpWord { position, at_root });
            }
            None => return None,
        }
    }
    None
}

/// Returns the deepest command that `args` names after the program name, and
/// whether `args` asks for long help and for JSON.
///
/// It skips the program name, stops at `--`, skips each flag and each value
/// of an option, and descends into each word that names a subcommand of the
/// current command. `Command::find_subcommand` resolves an alias to its
/// command and finds a hidden command, because the caller typed its name.
/// The walk runs before parsing, because help needs its lines before clap
/// prints it. A word counts as `--help` or `--json` only where the reader
/// reads it as a flag, so the value of an option that allows hyphen values
/// counts as neither.
pub(super) fn walk(command: &Command, args: &[OsString]) -> Walked {
    let mut words = Words::new(command);
    let mut walked = Walked {
        path: Vec::new(),
        long_help: false,
        json: false,
    };
    for word in args.iter().skip(1) {
        match words.read(word) {
            Kind::End => break,
            Kind::Value => {}
            Kind::Flag => {
                walked.long_help |= word == "--help";
                walked.json |= word == "--json";
            }
            Kind::Name => {
                if let Some(subcommand) = words.current.find_subcommand(word) {
                    walked.path.push(subcommand.get_name().to_owned());
                    words.enter(subcommand);
                }
            }
        }
    }
    walked
}

/// How [`Words::read`] reads one word.
enum Kind {
    /// The word `--`, after which every word is a positional value.
    End,
    /// A word that starts with `-` and is no option's value.
    Flag,
    /// A value of the option before it.
    Value,
    /// Any other word, which can name a subcommand or be a positional value.
    Name,
}

/// Reads the typed words one at a time as clap's parser does, so a value of
/// an option is never read as a subcommand.
///
/// An option's values are the rest of its own word in `--name=value`,
/// `-nvalue` and `-n=value`, and otherwise the next words, up to the most
/// values that the option takes. A word that starts with `-` ends the values
/// unless the option allows hyphen values, and the option's value terminator
/// ends them too. An option that requires `=` takes no value from the next
/// word.
//
// RUST-10: the reader reads the arguments that a command holds before clap
// builds it, and the global arguments of the commands above it, which clap
// copies down when it builds each command. It misses what clap adds only in
// the build: the arguments of `Command::defer`, and the hyphen values that a
// command's own `allow_hyphen_values` allows. It also misses the prefixes
// that `infer_long_args` accepts. It reads each word it misses as a flag
// without values. A walk over the commands that clap builds along the path
// replaces it if a tool needs those settings.
struct Words<'a> {
    /// The current command, the deepest that the reader entered.
    current: &'a Command,
    /// The commands from the root down to the one above `current`.
    above: Vec<&'a Command>,
    /// The option whose values the next words are, and how many more values
    /// it takes.
    pending: Option<(&'a Arg, usize)>,
}

impl<'a> Words<'a> {
    /// Returns a reader that starts at the root `command`.
    const fn new(command: &'a Command) -> Self {
        Self {
            current: command,
            above: Vec::new(),
            pending: None,
        }
    }

    /// Makes `subcommand` of the current command the current command.
    fn enter(&mut self, subcommand: &'a Command) {
        self.above.push(self.current);
        self.current = subcommand;
        self.pending = None;
    }

    /// Reads `word`, the next word after the program name.
    fn read(&mut self, word: &OsStr) -> Kind {
        if let Some((arg, left)) = self.pending.take() {
            if arg
                .get_value_terminator()
                .is_some_and(|terminator| word == terminator.as_str())
            {
                return Kind::Value;
            }
            if !starts_with_hyphen(word) || arg.is_allow_hyphen_values_set() {
                self.pending = (left > 1).then_some((arg, left - 1));
                return Kind::Value;
            }
        }
        if word == "--" {
            return Kind::End;
        }
        if !starts_with_hyphen(word) {
            return Kind::Name;
        }
        // A word that is not UTF-8 names no flag that the reader can find.
        let Some(text) = word.to_str() else {
            return Kind::Flag;
        };
        self.pending = match text.strip_prefix("--") {
            Some(long) => self.read_long(long),
            None => self.read_short(text.trim_start_matches('-')),
        };
        Kind::Flag
    }

    /// Returns the option that takes the next words as its values after the
    /// long flag `long`, written without its `--`, or `None` when no next
    /// word is its value.
    fn read_long(&self, long: &str) -> Option<(&'a Arg, usize)> {
        if long.contains('=') {
            return None;
        }
        let arg = self.find(|arg| {
            arg.get_long() == Some(long)
                || arg
                    .get_all_aliases()
                    .is_some_and(|aliases| aliases.contains(&long))
        })?;
        takes_next(arg)
    }

    /// Returns the option that takes the next words as its values after the
    /// group of short flags `shorts`, written without its `-`, or `None`
    /// when no next word is its value.
    ///
    /// The first letter in the group that names an option ends the flags,
    /// and the rest of the word, when there is any, is its value.
    fn read_short(&self, shorts: &str) -> Option<(&'a Arg, usize)> {
        for (index, letter) in shorts.char_indices() {
            let arg = self.find(|arg| {
                arg.get_short() == Some(letter)
                    || arg
                        .get_all_short_aliases()
                        .is_some_and(|aliases| aliases.contains(&letter))
            })?;
            if most_values(arg) > 0 {
                let rest = &shorts[index + letter.len_utf8()..];
                return if rest.is_empty() {
                    takes_next(arg)
                } else {
                    None
                };
            }
        }
        None
    }

    /// Returns the first argument that `matches` accepts among the current
    /// command's arguments, and then among the global arguments of each
    /// command above it, nearest first, as clap copies them down.
    fn find(&self, matches: impl Fn(&Arg) -> bool) -> Option<&'a Arg> {
        let own = self.current.get_arguments().find(|arg| matches(arg));
        own.or_else(|| {
            self.above
                .iter()
                .rev()
                .flat_map(|command| command.get_arguments())
                .find(|arg| arg.is_global_set() && matches(arg))
        })
    }
}

/// Returns `arg` and the number of words after its flag that can be its
/// values, or `None` when it takes none from the next words.
fn takes_next(arg: &Arg) -> Option<(&Arg, usize)> {
    let most = most_values(arg);
    (most > 0 && !arg.is_require_equals_set()).then_some((arg, most))
}

/// Returns the most values that one occurrence of `arg` takes.
///
/// clap sets `num_args` on each argument without one when it builds the
/// command, and the walk runs before that. An argument without `num_args`
/// takes what clap's build sets: one value for each of its value names when
/// it has more than one, one value when its action takes values, and none
/// otherwise.
fn most_values(arg: &Arg) -> usize {
    if let Some(range) = arg.get_num_args() {
        return range.max_values();
    }
    match arg.get_value_names() {
        Some(names) if names.len() > 1 => names.len(),
        _ => usize::from(arg.get_action().takes_values()),
    }
}

/// Reports whether `word` starts with `-`.
fn starts_with_hyphen(word: &OsStr) -> bool {
    word.as_encoded_bytes().starts_with(b"-")
}
