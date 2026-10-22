//! The walk, which reads the typed words down the command tree before clap
//! parses them, and finds the word `help` that crumbkit answers. The module
//! also names the program by its file stem on Windows.

use std::ffi::{OsStr, OsString};
use std::path::Path;

use clap::{Arg, Command};

use crate::dashed;
use crate::help::Guide;

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

/// What [`given`] read from the words of a command line, borrowing the
/// arguments from the command for `'c` and the values from the words for
/// `'w`.
pub(super) struct Given<'c, 'w> {
    /// The real names of the subcommands from the root to the deepest
    /// command that the words name.
    pub(super) path: Vec<String>,
    /// The clap ids of the arguments that the words pass as flags.
    pub(super) ids: Vec<String>,
    /// Each value that the words give an option, with that option, in the
    /// order typed. A value in the option's own word, as in `--name=value`,
    /// `-nvalue` or `-n=value`, is the text after the name and its `=`.
    pub(super) values: Vec<(&'c Arg, &'w OsStr)>,
    /// The number of words that are positional values of the deepest
    /// command.
    pub(super) positionals: usize,
}

/// Returns what `words`, the words after the program name, give the
/// deepest command that they name in `command`: the path to it, the ids of
/// the flags typed, the values of the options typed, and the number of
/// positional values.
///
/// It reads the words as [`walk`] does. With `dashed_forms`, a word that is
/// a subcommand's dashed form, such as `reading-lists` for `reading_lists`,
/// names that subcommand (ADR-0015). A word that names no subcommand, and each word
/// after `--`, is a positional value, and the walk descends into no
/// subcommand after the first positional value.
pub(super) fn given<'c, 'w, T: AsRef<OsStr>>(
    command: &'c Command,
    words: &'w [T],
    dashed_forms: bool,
) -> Given<'c, 'w> {
    let mut reader = Words::new(command);
    let mut given = Given {
        path: Vec::new(),
        ids: Vec::new(),
        values: Vec::new(),
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
            Kind::Value(arg) => given.values.extend(arg.map(|arg| (arg, word))),
            Kind::Flag => {
                given.ids.extend(reader.ids(word));
                given.values.extend(reader.inline(word));
            }
            Kind::Name => {
                let subcommand = match (given.positionals, dashed_forms) {
                    (0, true) => dashed::subcommand(reader.current, word),
                    (0, false) => reader.current.find_subcommand(word),
                    _ => None,
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
/// alias of `list`. An empty path names the root. When `guide` declares
/// [`Guide::dashed_forms`], a word that is a subcommand's
/// [`dashed_form`](crate::dashed_form) names that subcommand, as it does
/// for [`parse`](crate::parse()).
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
/// use crumbkit::help::Guide;
///
/// let guide = Guide::new();
/// let tool = Command::new("shelf")
///     .arg(Arg::new("dir").short('C').global(true))
///     .subcommand(Command::new("books").subcommand(Command::new("list").alias("ls")));
///
/// assert_eq!(
///     crumbkit::command_path(&tool, &guide, &["-C", "books", "books", "ls"]),
///     ["books", "list"]
/// );
/// assert_eq!(crumbkit::command_path(&tool, &guide, &["books", "lst"]), ["books"]);
/// ```
#[must_use]
pub fn command_path<T: AsRef<OsStr>>(command: &Command, guide: &Guide, words: &[T]) -> Vec<String> {
    given(command, words, guide.get_dashed_forms()).path
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
/// value. A run that types only the tool's own names changes no word, and
/// without `dashed_forms`, the guide's [`Guide::dashed_forms`] declaration,
/// no word changes.
///
/// It returns the position in `args` and the typed text of each word that
/// it replaced, so a refusal can report the words as the caller typed them.
/// A run that changes no word allocates nothing.
pub(crate) fn own_names(
    command: &Command,
    dashed_forms: bool,
    args: &mut [OsString],
) -> Vec<(usize, OsString)> {
    let mut replaced = Vec::new();
    if !dashed_forms {
        return replaced;
    }
    let mut reader = Words::new(command);
    for (position, word) in args.iter_mut().enumerate().skip(1) {
        match reader.read(word) {
            Kind::End => break,
            Kind::Flag | Kind::Value(_) => continue,
            Kind::Name => {}
        }
        let current = reader.current;
        if let Some(subcommand) = current.find_subcommand(&*word) {
            reader.enter(subcommand);
        } else if let Some(subcommand) = dashed::by_form(current, word) {
            let typed = std::mem::replace(word, OsString::from(subcommand.get_name()));
            replaced.push((position, typed));
            reader.enter(subcommand);
        } else if word != "help" {
            break;
        }
    }
    replaced
}

/// Returns the words after the program name in `args` as the caller typed
/// them, with each word of `replaced`, the positions in `args` and the
/// typed text that [`own_names`] replaced, put back.
pub(super) fn typed_words(args: &[OsString], replaced: Vec<(usize, OsString)>) -> Vec<OsString> {
    let mut typed = args.to_vec();
    for (position, word) in replaced {
        if let Some(slot) = typed.get_mut(position) {
            *slot = word;
        }
    }
    typed.remove(0);
    typed
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
            Kind::Flag | Kind::Value(_) => continue,
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
/// counts as neither. The word `--json` counts only where the current
/// command, or a command above it through a global argument, declares an
/// argument with the long name `json`, because clap refuses an undeclared
/// `--json` as an unexpected argument.
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
            Kind::Value(_) => {}
            Kind::Flag => {
                walked.long_help |= word == "--help";
                // RUST-10: only the words `-h` and `--help` count, so `-h`
                // inside a group of short flags, such as `-qh`, and a help
                // flag that the tool renamed ask for help without counting.
                // The only cost is that the root's help then has no line
                // that names a person's aliases.
                walked.help |= word == "-h" || word == "--help";
                walked.json |= word == "--json" && words.declares_json();
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

/// How [`Words::read`] reads one word, borrowing an option from the
/// command for `'a`.
pub(super) enum Kind<'a> {
    /// The word `--`, after which every word is a positional value.
    End,
    /// A word that starts with `-` and is no option's value.
    Flag,
    /// A value of the option before it, which the variant holds, or the
    /// option's value terminator, which ends its values and holds `None`.
    Value(Option<&'a Arg>),
    /// Any other word, which can name a subcommand or be a positional value.
    Name,
}

/// Reads the typed words one at a time as clap's parser does, so a value of
/// an option is never read as a subcommand, and names the option of each
/// value.
///
/// An option's values are the rest of its own word in `--name=value`,
/// `-nvalue` and `-n=value`, and otherwise the next words, up to the most
/// values that the option takes. A word that starts with `-` ends the values
/// unless the option allows hyphen values, and the option's value terminator
/// ends them too. An option that requires `=` takes no value from the next
/// word. The `help_word` module reads the words after `help` with this
/// reader too, so a value of an option names no topic.
//
// RUST-10: the reader reads the arguments that a command holds before clap
// builds it, and the global arguments of the commands above it, which clap
// copies down when it builds each command. It misses what clap adds only in
// the build: the arguments of `Command::defer`, and the hyphen values that a
// command's own `allow_hyphen_values` allows. It also misses the prefixes
// that `infer_long_args` accepts. It reads each word it misses as a flag
// without values. It gives each value as one word and never splits it at
// the option's value delimiter. A walk over the commands that clap builds
// along the path replaces it if a tool needs those settings.
pub(super) struct Words<'a> {
    /// The current command, the deepest that the reader entered.
    pub(super) current: &'a Command,
    /// The commands from the root down to the one above `current`.
    above: Vec<&'a Command>,
    /// The option whose values the next words are, and how many more values
    /// it takes.
    pending: Option<(&'a Arg, usize)>,
}

impl<'a> Words<'a> {
    /// Returns a reader that starts at the root `command`.
    pub(super) const fn new(command: &'a Command) -> Self {
        Self {
            current: command,
            above: Vec::new(),
            pending: None,
        }
    }

    /// Makes `subcommand` of the current command the current command.
    pub(super) fn enter(&mut self, subcommand: &'a Command) {
        self.above.push(self.current);
        self.current = subcommand;
        self.pending = None;
    }

    /// Reads `word`, the next word after the program name.
    pub(super) fn read(&mut self, word: &OsStr) -> Kind<'a> {
        if let Some((arg, left)) = self.pending.take() {
            if arg
                .get_value_terminator()
                .is_some_and(|terminator| word == terminator.as_str())
            {
                return Kind::Value(None);
            }
            if !starts_with_hyphen(word) || arg.is_allow_hyphen_values_set() {
                self.pending = (left > 1).then_some((arg, left - 1));
                return Kind::Value(Some(arg));
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

    /// Reports whether the current command declares an argument with the
    /// long name `json`, or a command above it declares one as global.
    fn declares_json(&self) -> bool {
        self.find(|arg| arg.get_long() == Some("json")).is_some()
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
        let (arg, rest) = self.short_option(shorts)?;
        if rest.is_empty() {
            takes_next(arg)
        } else {
            None
        }
    }

    /// Returns the option in the group of short flags `shorts`, written
    /// without its `-`, and the rest of the word after the option's letter,
    /// or `None` when the group holds no option before its first letter
    /// that names no argument.
    fn short_option<'w>(&self, shorts: &'w str) -> Option<(&'a Arg, &'w str)> {
        for (index, letter) in shorts.char_indices() {
            let arg = self.find_short(letter)?;
            if most_values(arg) > 0 {
                return Some((arg, &shorts[index + letter.len_utf8()..]));
            }
        }
        None
    }

    /// Returns the option that the flag word `word` gives a value in its
    /// own word, and that value: the text after the first `=` of
    /// `--name=value`, or the rest of a group of short flags after the
    /// option's letter, without a leading `=`, as in `-nvalue` and
    /// `-n=value`. A word that gives no option a value returns `None`.
    fn inline<'w>(&self, word: &'w OsStr) -> Option<(&'a Arg, &'w OsStr)> {
        let text = word.to_str()?;
        let (arg, value) = if let Some(long) = text.strip_prefix("--") {
            let (name, value) = long.split_once('=')?;
            (self.find_long(name)?, value)
        } else {
            let (arg, rest) = self.short_option(text.trim_start_matches('-'))?;
            if rest.is_empty() {
                return None;
            }
            (arg, rest.strip_prefix('=').unwrap_or(rest))
        };
        (most_values(arg) > 0).then_some((arg, OsStr::new(value)))
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
