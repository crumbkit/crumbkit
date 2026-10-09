# crumbkit-test

crumbkit-test checks a tool built with crumbkit from an ordinary `#[test]`.

crumbkit is in design and is not published yet. This crate has six checks.
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
  crumbkit reserves for its write verbs.
- `check_listings` fails on a `list` command without the `--limit` and
  `--all` flags that flattening crumbkit's `Page` gives it, and on a `show`
  command without a required positional argument that takes 1 to 100 values.
- `check_filters` fails on an option that accepts many values unless it
  accepts both repeats and commas, as `--author a --author b` and
  `--author a,b`. Review checks that an option that takes one value refuses
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
  `crumbkit::help::extend`, and no exemptions. It fails on a group whose
  path names no command or a command without subcommands, on a goal that
  names a command its group lacks, on a visible subcommand of a group that
  no goal names, and on a subcommand that two goals name. A hidden
  subcommand passes whether or not a goal names it.

```rust
use clap::Command;
use crumbkit::help::{Goal, Group, Guide};

const GUIDE: Guide = Guide::new().groups(&[Group::new(
    &[],
    &[Goal::new("Browse the catalogue", &["books"])],
)]);

let shelf = Command::new("shelf")
    .subcommand(Command::new("books").subcommand(Command::new("list")));

crumbkit_test::check_guide(&shelf, &GUIDE);
```

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.
