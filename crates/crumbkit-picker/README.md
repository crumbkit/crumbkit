# crumbkit-picker

crumbkit-picker gives a tool built with crumbkit a picker: when a person
leaves out a required argument, the tool shows the argument's choices as a
numbered list and runs the command with the one the person picks. It needs
only the standard library. crumbkit is in design and is not published yet.

## Adding it

A tool adds crumbkit-picker beside crumbkit, from this repository until the
first release:

```toml
[dependencies]
crumbkit-picker = { git = "https://github.com/crumbkit/crumbkit" }
```

The `main` branch can change without notice before the first release.

## Example

A tool sets the picker in its guide, names the source of each argument's
choices on the argument's leaf, and flattens `NoInput` into its root
arguments, so a script or an agent can say that nobody can answer:

```rust
use clap::{CommandFactory, Parser, Subcommand};
use crumbkit::help::{Guide, Leaf};
use crumbkit::pick::{Choice, Choices};
use crumbkit::{Error, NoInput, Spelling};
use crumbkit_picker::Menu;

/// Browse a small catalogue.
#[derive(Parser)]
#[command(name = "shelf")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[command(flatten)]
    no_input: NoInput,
}

#[derive(Subcommand)]
enum Commands {
    /// Show a book.
    Show {
        /// The book to show.
        #[arg(long)]
        id: u32,
    },
}

fn books() -> Result<Vec<Choice>, Error> {
    Ok(vec![
        Choice::new("1".to_owned(), "Dune".to_owned()),
        Choice::new("2".to_owned(), "Emma".to_owned()),
    ])
}

static MENU: Menu = Menu::new();

const GUIDE: Guide = Guide::new()
    .picker(&MENU)
    .leaves(&[Leaf::new(&["show"]).choices(&[Choices::new("id", books)])]);

let parsed = crumbkit::parse_from::<Cli, _, _>(
    Cli::command(),
    &GUIDE,
    Spelling::COLONS,
    ["shelf", "show", "--id", "2"],
);
assert!(parsed.is_ok());
```

When `shelf show` runs at a terminal without `--id`, the menu shows the
books and reads the person's answer:

```text
shelf show needs ID: The book to show
  1. Dune
  2. Emma
Type a number to pick, text to narrow the list, or nothing to cancel: 2
Running: shelf show --id=2
```

The person types a number to pick, or several numbers separated by spaces
when the argument takes several values. Other text narrows the list to the
choices that contain it. An empty line or the end of input cancels, and the
tool then refuses the command line as it would without a picker. The menu
shows 20 choices at once unless the tool sets another number with
`Menu::rows`.

`crumbkit::parse` asks only when standard input and standard error are
terminals, no coding agent runs the tool, and the caller did not pass
`--no-input`. Otherwise it refuses the command line, so a script never waits
for an answer. An argument with possible values offers them and needs no
source. `crumbkit::pick::from_rows` turns the rows that a list command
prints into choices.

## Using another picker

A tool can wrap any prompt library in its own type that implements
`crumbkit::pick::Picker`, and set that type in its guide in place of
`Menu`. The picker receives the question and the process's standard input
and standard error, and returns the positions of the picked choices:

```rust
use std::io::{self, BufRead, Write};

use crumbkit::pick::{Answer, Picker, Question};

/// A picker that asks with the tool's own prompt function.
struct Prompt;

/// Stands in for a prompt library's function that asks the person to pick
/// one of `labels` and returns its position, or `None` when they cancel.
fn select(title: &str, labels: &[&str], output: &mut dyn Write) -> io::Result<Option<usize>> {
    writeln!(output, "{title}: {}", labels.join(", "))?;
    Ok(labels.first().map(|_| 0))
}

impl Picker for Prompt {
    fn pick(
        &self,
        question: &Question<'_>,
        _input: &mut dyn BufRead,
        output: &mut dyn Write,
    ) -> io::Result<Answer> {
        let labels: Vec<&str> = question.choices().iter().map(|choice| choice.label()).collect();
        let title = format!("{} needs {}", question.command(), question.name());
        Ok(match select(&title, &labels, output)? {
            Some(position) => Answer::Picked(vec![position]),
            None => Answer::Cancelled,
        })
    }
}

static PROMPT: Prompt = Prompt;
let guide = crumbkit::help::Guide::new().picker(&PROMPT);
assert!(guide.get_picker().is_some());
```

[ADR-0043] records why crumbkit asks for a missing argument and when it
never asks.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.

[ADR-0043]: ../../docs/decisions/api/effects/0043-pickers-and-interactive-input.md
