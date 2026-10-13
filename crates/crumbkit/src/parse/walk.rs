//! The walk, which reads the typed words down the command tree before clap
//! parses them, and finds the word `help` that crumbkit answers. The module
//! also names the program by its file stem on Windows.

use std::ffi::{OsStr, OsString};
use std::path::Path;

use clap::{Arg, Command};

use crate::dashed;

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
    /// Whether a word reads as the flag `-h` or `--help`, so clap prints
    /// help.
    pub(super) help: bool,
    /// Whether a word reads as the flag `--json`, so a refusal prints as a
    /// JSON document.
    pub(super) json: bool,
}

/// What [`given`] read from the words of a command line.
pub(super) struct Given {
    /// The real names of the subcommands from the root to the deepest
    /// command that the words name.
    pub(super) path: Vec<String>,
    /// The clap ids of the arguments that the words pass as flags.
    pub(super) ids: Vec<String>,
    /// The number of words that are positional values of the deepest
    /// command.
    pub(super) positionals: usize,
}

/// Returns what `words`, the words after the program name, give the
/// deepest command that they name in `command`: the path to it, the ids of
/// the flags typed, and the number of positional values.
///
/// It reads the words as [`walk`] does, and a word that is a subcommand's
/// dashed form, such as `reading-lists` for `reading_lists`, names that
/// subcommand (ADR-0015). A word that names no subcommand, and each word
/// after `--`, is a positional value, and the walk descends into no
/// subcommand after the first positional value.
pub(super) fn given<T: AsRef<OsStr>>(command: &Command, words: &[T]) -> Given {
    let mut reader = Words::new(command);
    let mut given = Given {
        path: Vec::new(),
        ids: Vec::new(),
        positionals: 0,
    };
    let mut ended = false;
    for word in words {
        let word = word.as_ref();
        if ended {
            given.positionals += 1;
            continue;
        }
        match reader.read(word) {
            Kind::End => ended = true,
            Kind::Value => {}
            Kind::Flag => given.ids.extend(reader.ids(word)),
            Kind::Name => {
                let subcommand = if given.positionals == 0 {
                    dashed::subcommand(reader.current, word)
                } else {
                    None
                };
                match subcommand {
                    Some(subcommand) => {
                        given.path.push(subcommand.get_name().to_owned());
                        reader.enter(subcommand);
                    }
                    None => given.positionals += 1,
                }
            }
        }
    }
    given
}

/// Returns the real names of the subcommands from the root `command` down
/// to the deepest command that `words`, the words after the program name,
/// name, such as `["books", "list"]` for `--json books ls` when `ls` is an
/// alias of `list`. An empty path names the root. A word that is a
/// subcommand's [`dashed_form`](crate::dashed_form) names that subcommand,
/// as it does for [`parse`](crate::parse()).
///
/// The function reads the words as [`parse`](crate::parse) does before
/// clap parses them, and it never builds or clones the tree. It skips each
/// flag and each value of an option, including the global options of the
/// commands above, so a value that equals a command's name names no
/// command. It stops at `--` and at the first word that names no
/// subcommand, so it also names the command of a line that clap refuses,
/// such as `books` for `books lst`. A tool can use it to learn which
/// command a run named, for its own log of a refused run or to choose
/// work before it parses.
///
/// ```
/// use clap::{Arg, Command};
///
/// let tool = Command::new("shelf")
///     .arg(Arg::new("dir").short('C').global(true))
///     .subcommand(Command::new("books").subcommand(Command::new("list").alias("ls")));
///
/// assert_eq!(crumbkit::command_path(&tool, &["-C", "books", "books", "ls"]), ["books", "list"]);
/// assert_eq!(crumbkit::command_path(&tool, &["books", "lst"]), ["books"]);
/// ```
#[must_use]
pub fn command_path<T: AsRef<OsStr>>(command: &Command, words: &[T]) -> Vec<String> {
    given(command, words).path
}

/// Replaces each word of `args` that reaches a subcommand by its
/// [`dashed_form`](crate::dashed_form) with the subcommand's own name, so
/// clap, help, the usage log and the parsed words all name the command as
/// the tool does. `args` starts with the program name.
///
/// It reads the words as [`given`] does, so an option's value that equals a
/// dashed form stays as typed. It skips the word `help` where no
/// subcommand has that name, because crumbkit answers it, so the words
/// after it in `shelf help reading-lists` change too. It stops at `--` and
/// at the first other word that names no subcommand, which is a positional
/// value. A run that types only the tool's own names changes no word.
pub(crate) fn own_names(command: &Command, args: &mut [OsString]) {
    let mut reader = Words::new(command);
    for word in args.iter_mut().skip(1) {
        match reader.read(word) {
            Kind::End => return,
            Kind::Flag | Kind::Value => continue,
            Kind::Name => {}
        }
        let current = reader.current;
        if let Some(subcommand) = current.find_subcommand(&*word) {
            reader.enter(subcommand);
        } else if let Some(subcommand) = dashed::by_form(current, word) {
            *word = OsString::from(subcommand.get_name());
            reader.enter(subcommand);
        } else if word != "help" {
            return;
        }
    }
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
/// whether `args` asks for help, for long help and for JSON.
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
        help: false,
        json: false,
    };
    for word in args.iter().skip(1) {
        match words.read(word) {
            Kind::End => break,
            Kind::Value => {}
            Kind::Flag => {
                walked.long_help |= word == "--help";
                // RUST-10: only the words `-h` and `--help` count, so `-h`
                // inside a group of short flags, such as `-qh`, and a help
                // flag that the tool renamed ask for help without counting.
                // The only cost is that the root's help then has no line
                // that names a person's aliases.
                walked.help |= word == "-h" || word == "--help";
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
        takes_next(self.find_long(long)?)
    }

    /// Returns the argument whose long name or alias is `long`.
    fn find_long(&self, long: &str) -> Option<&'a Arg> {
        self.find(|arg| {
            arg.get_long() == Some(long)
                || arg
                    .get_all_aliases()
                    .is_some_and(|aliases| aliases.contains(&long))
        })
    }

    /// Returns the argument whose short name or short alias is `letter`.
    fn find_short(&self, letter: char) -> Option<&'a Arg> {
        self.find(|arg| {
            arg.get_short() == Some(letter)
                || arg
                    .get_all_short_aliases()
                    .is_some_and(|aliases| aliases.contains(&letter))
        })
    }

    /// Returns the clap ids of the arguments that the flag word `word`
    /// passes: the argument of a long flag, with or without `=value`, or
    /// each argument of a group of short flags up to the first that takes
    /// a value. A name that no argument has passes nothing.
    fn ids(&self, word: &OsStr) -> Vec<String> {
        let Some(text) = word.to_str() else {
            return Vec::new();
        };
        if let Some(long) = text.strip_prefix("--") {
            let long = long.split_once('=').map_or(long, |(name, _)| name);
            return self
                .find_long(long)
                .map(|arg| arg.get_id().to_string())
                .into_iter()
                .collect();
        }
        let mut ids = Vec::new();
        for letter in text.trim_start_matches('-').chars() {
            let Some(arg) = self.find_short(letter) else {
                break;
            };
            ids.push(arg.get_id().to_string());
            if most_values(arg) > 0 {
                break;
            }
        }
        ids
    }

    /// Returns the option that takes the next words as its values after the
    /// group of short flags `shorts`, written without its `-`, or `None`
    /// when no next word is its value.
    ///
    /// The first letter in the group that names an option ends the flags,
    /// and the rest of the word, when there is any, is its value.
    fn read_short(&self, shorts: &str) -> Option<(&'a Arg, usize)> {
        for (index, letter) in shorts.char_indices() {
            let arg = self.find_short(letter)?;
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

/// Replaces the program path in `args[0]` with its file stem. On Windows,
/// clap names the tool after the file name of `args[0]`, so without this a
/// tool's help and usage lines would say `shelf.exe` where they say `shelf`
/// on Unix.
pub(super) fn program_stem(args: &mut [OsString]) {
    if let Some(program) = args.first_mut()
        && let Some(stem) = Path::new(program).file_stem()
    {
        *program = stem.to_owned();
    }
}
