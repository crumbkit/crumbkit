//! A differential test of the walk: it generates a command tree and a
//! command line, and checks that `walk` and `given` read the command path,
//! the flags typed and `--json` as clap reads them from the same line.
//
// RUST-10: the generator covers boolean flags and options with one value on
// a tree of subcommands with aliases and global arguments, and lines that
// clap accepts. It misses what the comment on `Words` says the reader
// misses, options with more than one value, hyphen values, value
// terminators and `require_equals`, which the tests in `walk/tests.rs` check
// against written expectations. It never puts a positional value before a
// subcommand word, where `walk` keeps descending and `given` stops, and it
// compares no line that clap refuses. A generator that also builds those
// arguments and lines replaces it once the two readings agree there.

use std::collections::BTreeSet;
use std::ffi::OsString;

use clap::parser::ValueSource;
use clap::{Arg, ArgAction, ArgMatches, Command};
use proptest::collection::vec;
use proptest::prelude::*;

use super::super::{given, walk};

/// The names of subcommands and their aliases, which are also the values of
/// options and positional arguments, so a value often equals a command's
/// name.
const NAMES: [&str; 6] = ["id", "list", "name", "new", "show", "sort"];

/// The long names of the arguments, three for each level of the tree, so no
/// two arguments on one path from the root share a name. The first letter of
/// each is its short name, and no name starts with `h`, which clap keeps for
/// help.
const LONGS: [&str; 12] = [
    "alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "india", "kilo", "lima",
    "mike", "november",
];

/// The most arguments that one command declares.
const ARGUMENTS: usize = 3;

/// The most subcommands that one command has.
const SUBCOMMANDS: usize = 3;

/// The number of levels of subcommands below the root.
const LEVELS: usize = 3;

/// The id of the optional positional argument of a command without
/// subcommands.
const POSITIONAL: &str = "target";

/// One generated argument: a boolean flag, or an option with one value.
#[derive(Debug, Clone)]
struct Argument {
    /// The long name, which is also the clap id.
    long: &'static str,
    /// The short name, when the argument has one.
    short: Option<char>,
    /// Whether the argument is an option with one value.
    takes_value: bool,
    /// Whether the commands below read the argument too.
    global: bool,
    /// Whether the option has a default value.
    default: bool,
}

impl Argument {
    /// Returns the clap argument.
    fn arg(&self) -> Arg {
        let action = if self.takes_value {
            ArgAction::Set
        } else {
            ArgAction::SetTrue
        };
        let mut arg = Arg::new(self.long)
            .long(self.long)
            .action(action)
            .global(self.global);
        if let Some(short) = self.short {
            arg = arg.short(short);
        }
        if self.default {
            arg = arg.default_value("fallback");
        }
        arg
    }
}

/// One generated subcommand.
#[derive(Debug, Clone)]
struct Subcommand {
    /// The real name.
    name: &'static str,
    /// The alias, when the subcommand has one.
    alias: Option<&'static str>,
    /// The arguments and subcommands.
    node: Node,
}

/// The arguments and subcommands of one generated command.
#[derive(Debug, Clone)]
struct Node {
    /// The arguments that the command declares.
    arguments: Vec<Argument>,
    /// The subcommands. A command without any takes one optional positional
    /// value.
    subcommands: Vec<Subcommand>,
}

impl Node {
    /// Returns the clap command with the name `name`.
    fn command(&self, name: &'static str) -> Command {
        let mut command = Command::new(name).args(self.arguments.iter().map(Argument::arg));
        if self.subcommands.is_empty() {
            command = command.arg(Arg::new(POSITIONAL));
        }
        command.subcommands(self.subcommands.iter().map(|subcommand| {
            let command = subcommand.node.command(subcommand.name);
            match subcommand.alias {
                Some(alias) => command.alias(alias),
                None => command,
            }
        }))
    }
}

/// Returns a strategy for the argument in place `slot` of a command `depth`
/// levels below the root. About half the arguments are options, about half
/// have a short name and about a third are global. About one option in eight
/// has a default value.
fn argument(depth: usize, slot: usize) -> impl Strategy<Value = Argument> {
    let long = LONGS[depth * ARGUMENTS + slot];
    (any::<bool>(), any::<bool>(), 0..3_u8, 0..8_u8).prop_map(
        move |(takes_value, short, global, default)| Argument {
            long,
            short: long.chars().next().filter(|_| short),
            takes_value,
            global: global == 0,
            default: takes_value && default == 0,
        },
    )
}

/// Returns a strategy for a command `depth` levels below the root, with up
/// to three arguments and, above the lowest level, up to three subcommands.
/// The subcommands take their names and aliases from a shuffle of `NAMES`,
/// so no two share one. A command without subcommands has no global
/// argument.
fn node(depth: usize) -> BoxedStrategy<Node> {
    let arguments: Vec<_> = (0..ARGUMENTS)
        .map(|slot| proptest::option::of(argument(depth, slot)))
        .collect();
    let subcommands = if depth == LEVELS {
        Just(Vec::new()).boxed()
    } else {
        (
            Just(NAMES).prop_shuffle(),
            vec((any::<bool>(), node(depth + 1)), 0..=SUBCOMMANDS),
        )
            .prop_map(|(names, nodes)| {
                nodes
                    .into_iter()
                    .zip(names.as_chunks::<2>().0)
                    .map(|((aliased, node), &[name, alias])| Subcommand {
                        name,
                        alias: aliased.then_some(alias),
                        node,
                    })
                    .collect()
            })
            .boxed()
    };
    (arguments, subcommands)
        .prop_map(|(arguments, subcommands): (_, Vec<Subcommand>)| {
            let mut arguments: Vec<Argument> = arguments.into_iter().flatten().collect();
            if subcommands.is_empty() {
                for argument in &mut arguments {
                    argument.global = false;
                }
            }
            Node {
                arguments,
                subcommands,
            }
        })
        .boxed()
}

/// Returns the global boolean flag `--json`.
const fn json() -> Argument {
    Argument {
        long: "json",
        short: None,
        takes_value: false,
        global: true,
        default: false,
    }
}

/// Returns a strategy for the root of a tree. About half the trees declare
/// the global flag `--json` on the root.
fn tree() -> impl Strategy<Value = Node> {
    (node(0), any::<bool>()).prop_map(|(mut root, declares_json)| {
        if declares_json {
            root.arguments.push(json());
        }
        root
    })
}

/// The numbers that decide how a line reads, taken in order. The numbers
/// after the last are zero, which leaves out every choice.
struct Picks<'a> {
    /// The numbers not yet taken.
    numbers: std::slice::Iter<'a, u8>,
}

impl Picks<'_> {
    /// Returns the next number, reduced to less than `bound`.
    fn below(&mut self, bound: usize) -> usize {
        usize::from(self.numbers.next().copied().unwrap_or(0)) % bound
    }

    /// Reports whether the next number is odd.
    fn chance(&mut self) -> bool {
        self.below(2) == 1
    }

    /// Reports whether the next number is not a multiple of four.
    fn likely(&mut self) -> bool {
        self.below(4) > 0
    }

    /// Returns one of `NAMES`.
    fn name(&mut self) -> &'static str {
        NAMES[self.below(NAMES.len())]
    }
}

/// Returns the words that give the option with the short flags `letters`,
/// which end with the option's letter, the value `value`: the next word, the
/// rest of the flag's word, or the rest of the word after `=`.
fn short_option(letters: &str, value: &str, form: usize) -> Vec<String> {
    match form {
        0 => vec![format!("-{letters}"), value.to_owned()],
        1 => vec![format!("-{letters}{value}")],
        _ => vec![format!("-{letters}={value}")],
    }
}

/// Returns the words that pass `argument` on its own.
fn alone(argument: &Argument, picks: &mut Picks<'_>) -> Vec<String> {
    let long = argument.long;
    let short = argument.short.filter(|_| picks.chance());
    if !argument.takes_value {
        return vec![short.map_or_else(|| format!("--{long}"), |short| format!("-{short}"))];
    }
    let value = picks.name();
    match short {
        Some(short) => short_option(&short.to_string(), value, picks.below(3)),
        None if picks.chance() => vec![format!("--{long}"), value.to_owned()],
        None => vec![format!("--{long}={value}")],
    }
}

/// Returns the words that pass most of the arguments `in_scope`, each at
/// most once, with the words of each argument together. Boolean flags with
/// short names mostly join into one group, and the group mostly ends with an
/// option's letter.
fn typed(in_scope: &[&Argument], picks: &mut Picks<'_>) -> Vec<Vec<String>> {
    let mut chosen: Vec<&Argument> = in_scope
        .iter()
        .copied()
        .filter(|_| picks.likely())
        .collect();
    let mut passed = Vec::new();
    if picks.likely() {
        let mut letters = String::new();
        chosen.retain(|argument| match argument.short {
            Some(short) if !argument.takes_value => {
                letters.push(short);
                false
            }
            _ => true,
        });
        let last = chosen
            .iter()
            .position(|argument| argument.takes_value && argument.short.is_some())
            .filter(|_| picks.likely())
            .map(|position| chosen.remove(position));
        match last.and_then(|option| option.short) {
            Some(short) => {
                letters.push(short);
                passed.push(short_option(&letters, picks.name(), picks.below(3)));
            }
            None if letters.is_empty() => {}
            None => passed.push(vec![format!("-{letters}")]),
        }
    }
    passed.extend(chosen.into_iter().map(|argument| alone(argument, picks)));
    passed
}

/// Returns a line of words after the program name that names one path
/// through `root` and that clap should accept.
///
/// Before each subcommand word, which is the real name or the alias, the
/// line passes most of the arguments that the current command declares or
/// that a command above declares as global. A line that ends at a command
/// without subcommands sometimes holds one positional value among that
/// command's arguments, and sometimes ends with `--` and one word that
/// starts with a hyphen or equals a command's name.
fn line(root: &Node, picks: &mut Picks<'_>) -> Vec<String> {
    let mut words = Vec::new();
    let mut node = root;
    let mut in_scope: Vec<&Argument> = Vec::new();
    loop {
        in_scope.extend(&node.arguments);
        let mut passed = typed(&in_scope, picks);
        in_scope.retain(|argument| argument.global);
        if node.subcommands.is_empty() {
            match picks.below(4) {
                0 => {}
                1 => {
                    let place = picks.below(passed.len() + 1);
                    passed.insert(place, vec![picks.name().to_owned()]);
                }
                2 => {
                    let hyphened = ["--json", "-a", "--alpha=id"];
                    let word = hyphened[picks.below(hyphened.len())];
                    passed.push(vec!["--".to_owned(), word.to_owned()]);
                }
                _ => passed.push(vec!["--".to_owned(), picks.name().to_owned()]),
            }
        }
        words.extend(passed.into_iter().flatten());
        let Some(subcommand) = picks
            .below(node.subcommands.len() + 1)
            .checked_sub(1)
            .map(|chosen| &node.subcommands[chosen])
        else {
            return words;
        };
        let word = subcommand
            .alias
            .filter(|_| picks.chance())
            .unwrap_or(subcommand.name);
        words.push(word.to_owned());
        node = &subcommand.node;
    }
}

/// Returns the path of subcommand names that clap read into `matches`, and
/// the ids of the flags and options that clap read from the command line in
/// any command on that path. It leaves out the positional argument, which
/// `given` counts and never names. It also leaves out an id that clap holds in
/// the matches of a command that does not declare it, where asking for the
/// value's source panics; the matches of a declaring command hold the same
/// id.
fn read_by_clap(matches: &ArgMatches) -> (Vec<String>, BTreeSet<String>) {
    let mut path = Vec::new();
    let mut ids = BTreeSet::new();
    let mut current = matches;
    loop {
        ids.extend(
            current
                .ids()
                .filter(|id| *id != POSITIONAL)
                .filter(|id| current.try_contains_id(id.as_str()).is_ok())
                .filter(|id| current.value_source(id.as_str()) == Some(ValueSource::CommandLine))
                .map(ToString::to_string),
        );
        let Some((name, below)) = current.subcommand() else {
            return (path, ids);
        };
        path.push(name.to_owned());
        current = below;
    }
}

proptest! {
    #[test]
    fn the_walk_reads_the_path_the_flags_and_json_as_clap_does_from_a_line_that_clap_accepts(
        root in tree(),
        numbers in vec(any::<u8>(), 128),
    ) {
        let command = root.command("tool");
        let words = line(&root, &mut Picks { numbers: numbers.iter() });
        let args: Vec<OsString> = ["tool"]
            .into_iter()
            .chain(words.iter().map(String::as_str))
            .map(OsString::from)
            .collect();
        let matches = command.clone().try_get_matches_from(&args);
        prop_assert!(
            matches.is_ok(),
            "clap refuses {:?}: {}",
            words,
            matches.unwrap_err()
        );
        let (path, ids) = read_by_clap(&matches.unwrap());

        let walked = walk(&command, &args);
        let given = given(&command, &words);

        prop_assert_eq!(&walked.path, &path, "the path of walk for {:?}", words);
        prop_assert_eq!(&given.path, &path, "the path of given for {:?}", words);
        prop_assert_eq!(
            given.ids.iter().cloned().collect::<BTreeSet<_>>(),
            ids.clone(),
            "the ids of given for {:?}",
            words
        );
        prop_assert_eq!(walked.json, ids.contains("json"), "json for {:?}", words);
    }
}
