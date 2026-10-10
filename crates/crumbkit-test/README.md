# crumbkit-test

crumbkit-test checks a tool built with crumbkit from an ordinary `#[test]`.

crumbkit is in design and is not published yet. This crate has nine checks.
Each check collects every problem it finds and fails the test with one
problem on each line.

- `check_codes` takes the tool's `Spelling` and the `CODES` list of each of
  its error types, and spells every code as `finish` prints it. The test
  fails when two different codes spell the same, or when a spelled code is
  empty or holds a character other than printable ASCII, `!` to `~`. A code
  that two lists share counts once.

```rust
use crumbkit::{Code, Spelling};

const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");
const TIMED_OUT: Code = Code::new("shelf", "network", "timed_out");

const BOOK_CODES: &[&Code] = &[&NOT_FOUND];
const NETWORK_CODES: &[&Code] = &[&TIMED_OUT];

crumbkit_test::check_codes(Spelling::COLONS, &[BOOK_CODES, NETWORK_CODES]);
```

Four checks take the tool's clap command, as `Cli::command()` returns it,
and a list of exemptions. Each one walks the command and every subcommand
below it, including hidden ones, and works on any node, so a crate can
check its own subtree. An exemption names a command by its path
below the command passed in, such as `"books list"`, and skips that command
and everything below it for that check only. An exemption that names no
command fails the test.

- `check_names` fails on every second name that help shows: a visible alias
  of a command or an argument, a visible short-flag or long-flag alias, and
  a command's short or long flag name, such as `-B` from
  `Command::short_flag`. Hidden aliases pass, so a renamed command or flag
  can keep its old name for a while. An argument may have one short form
  beside its long name, such as `-l` with `--limit`.
- `check_grammar` takes the tool's verbs beside `list` and `show`, and fails
  unless every command has the form `<tool> <resource> <verb>`: a command
  without subcommands is one of the verbs, and a command with subcommands is
  a resource named in lowercase words joined by dashes, such as
  `reading-lists`. It also fails on a `--dry-run` argument, a name that
  crumbkit reserves for its write verbs, and on a command named or aliased
  `help` at any level, a word that crumbkit reserves for asking for help.
- `check_listings` fails on a `list` command without the `--limit` and
  `--all` flags that flattening crumbkit's `Page` gives it, and on a `show`
  command without a required positional argument that takes 1 to 100 values.
- `check_filters` fails on an option that accepts many values unless it
  accepts both repeats and commas, as `--author a --author b` and
  `--author a,b`, with one value per use. It fails on such an option when
  the tool set its `num_args` to a maximum above 1, such as
  `num_args(1..=3)`, because clap then reads the words after the option as
  more values, so `list --author tolkien dragons` takes `dragons` as a
  second author. Review checks that an option that takes one value refuses
  a second, because clap has no documented getter for the settings that
  turn that refusal off.

```rust
use clap::{Arg, ArgAction, Args, Command};
use crumbkit::{Output, Page};

let shelf = Output::augment_args(Command::new("shelf"))
    .subcommand(
        Command::new("books")
            .subcommand(Page::<25>::augment_args(Command::new("list")))
            .subcommand(
                Command::new("show").arg(
                    Arg::new("ids")
                        .required(true)
                        .num_args(1..=100)
                        .action(ArgAction::Append),
                ),
            ),
    )
    .subcommand(Command::new("schema"));

crumbkit_test::check_names(&shelf, &[]);
crumbkit_test::check_grammar(&shelf, &[], &["schema"]);
crumbkit_test::check_listings(&shelf, &[]);
crumbkit_test::check_filters(&shelf, &[]);
```

A tool that derives its parser passes `&Cli::command()` in the same way.

- `check_guide` takes the tool's clap command and the `Guide` it passes to
  `crumbkit::parse`, and no exemptions. It fails on a group whose
  path names no command or a command without subcommands, on a goal that
  names a command its group lacks, on a visible subcommand of a group that
  no goal names, and on a subcommand that two goals name. A hidden
  subcommand passes whether or not a goal names it. It also fails on a
  topic name that is not lowercase words joined by dashes, on a topic name
  that a root command's name or alias hides, on a topic without text, and
  on two topics with one name. A topic with an empty summary passes. It
  fails on a leaf whose path names no command or a command with
  subcommands, on two leaves with one path, and on an example that clap
  refuses, with clap's error kind and the example's command line. An
  example that asks for `--help` or `--version` counts as refused. It fails
  on a visible command without subcommands, the root included when it has
  none, that no leaf with at least one example names. It fails on a leaf
  column whose name is not a field of the row type, and on a leaf column
  without a description, naming the field whose doc comment is missing.

```rust
use clap::Command;
use crumbkit::help::{Example, Goal, Group, Guide, Leaf, Topic};

const GUIDE: Guide = Guide::new()
    .groups(&[Group::new(
        &[],
        &[Goal::new("Browse the catalogue", &["books"])],
    )])
    .leaves(&[Leaf::new(&["books", "list"])
        .examples(&[Example::new(&[], "Print the books")])])
    .topics(&[Topic::new(
        "catalogue",
        "Where the books come from",
        "The catalogue holds six books, built into the binary.\n",
    )]);

let shelf = Command::new("shelf")
    .subcommand(Command::new("books").subcommand(Command::new("list")));

crumbkit_test::check_guide(&shelf, &GUIDE);
```

Two checks read the help pages that crumbkit prints. Each takes the tool's
clap command and its `Guide`, renders the pages with `crumbkit::parse_to`
on a copy of the tree, and visits the root and every visible command below
it. A hidden command and everything below it are skipped, because help
never shows them.

- `check_help_sizes` takes a `Budgets` and fails on a `-h` page whose
  plain text has more bytes than its budget. The default budgets are 1,500
  bytes for the root, 800 for a group, which is a command below the root
  with subcommands, and 1,200 for a leaf, which is a command below the root
  without subcommands. `Budgets::new()` holds the defaults, `.root`,
  `.group` and `.leaf` lower a whole class, and `.nodes` lowers single
  commands, named by their path below the root, such as `"books list"`. A
  command's own budget wins over its class budget. The check fails on a
  budget above the default, because a tool can lower a budget and cannot
  raise one, and on a command budget whose path names no command or a
  hidden one. `--help` has no budget.
- `check_help_goldens` takes a directory and compares the `-h` and `--help`
  page of each visited command with a golden file there, named after the
  command's path and the flag: `shelf-h.txt` and `shelf-help.txt` for the
  root, and `shelf-books-list-h.txt` and `shelf-books-list-help.txt` for
  `shelf books list`. It fails on a missing file, a file whose content
  differs, and a stale file, which is a `.txt` file that no page produces,
  so the directory holds only help golden files. With
  `SNAPSHOTS=overwrite`, the check writes and rewrites the files and
  deletes each stale one. A relative directory names a directory in the
  package that runs the test, because cargo runs tests from the package's
  manifest directory.

```rust
use clap::Command;
use crumbkit::help::{Example, Guide, Leaf};
use crumbkit_test::Budgets;

const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "list"])
    .examples(&[Example::new(&[], "Print the books")])]);

let shelf = Command::new("shelf")
    .about("Browse a small catalogue")
    .subcommand(Command::new("books").subcommand(Command::new("list")));

crumbkit_test::check_help_sizes(&shelf, &GUIDE, &Budgets::new());
crumbkit_test::check_help_sizes(
    &shelf,
    &GUIDE,
    &Budgets::new().leaf(1_000).nodes(&[("books list", 600)]),
);
```

```rust,no_run
use clap::Command;
use crumbkit::help::Guide;

let shelf = Command::new("shelf").subcommand(Command::new("books"));

crumbkit_test::check_help_goldens(&shelf, &Guide::new(), "tests/golden/help");
```

- `check_examples` takes the tool's clap command, its `Guide` and an
  `Examples`, and runs every example of every leaf against the tool's built
  binary, one run at a time, with the leaf's path and the example's words
  as separate arguments. It compares each run's standard output with a
  golden file named after the tool, the path and the words, such as
  `shelf-books-list.txt` and `shelf-books-list-all.txt`. It fails on a run
  that exits with a code other than 0 or writes to standard error, also
  with `SNAPSHOTS=overwrite`, on a binary that cannot start, on two
  examples with one file name, on a missing file, a file whose content
  differs and a stale file, so the directory holds only example golden
  files. It skips a leaf whose path names no command and runs the examples
  of hidden commands. Each run has a cleared environment, the variables of
  `.env`, and fresh XDG directories that the check deletes afterwards.
  `.exempt` skips the examples of each named command and everything below
  it, and an exemption that names no command fails the check.

`Examples::new` takes the path of the built binary and the golden
directory. cargo sets `CARGO_BIN_EXE_<name>` only for the integration tests
and benchmarks of the binary's own package, so a tool calls the check from
one of those. An integration test can import only from a library, so that
package needs a library target that exports the clap command and the
guide, and the binary calls that library. cargo runs those tests from the
package's manifest directory, and the binary starts there too, so a
relative directory or `.env` value resolves from there.

```rust,no_run
use clap::Command;
use crumbkit::help::{Example, Guide, Leaf};
use crumbkit_test::Examples;

const GUIDE: Guide = Guide::new().leaves(&[
    Leaf::new(&["books", "list"]).examples(&[Example::new(&["--all"], "Print every book")]),
    Leaf::new(&["books", "delete"]).examples(&[Example::new(&["2"], "Delete book 2")]),
]);

// In `tests/examples.rs`, the binary is `env!("CARGO_BIN_EXE_shelf")`.
const EXAMPLES: Examples = Examples::new("/path/to/shelf", "tests/golden/examples")
    .env(&[("SHELF_DATA", "tests/fixtures/books.json")])
    .exempt(&["books delete"]);

let shelf = Command::new("shelf").subcommand(
    Command::new("books")
        .subcommand(Command::new("list"))
        .subcommand(Command::new("delete")),
);

crumbkit_test::check_examples(&shelf, &GUIDE, &EXAMPLES);
```

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.
