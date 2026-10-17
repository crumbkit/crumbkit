# Getting started

This page takes a small clap tool to crumbkit in four steps. Each step is a
`pantry` binary in [examples/tutorial](../examples/tutorial), which the
workspace builds and tests. A test in that package fails when a block of
Rust on this page differs from the file that the link before it names, so
every block here compiles.

A tool adds crumbkit as the crate README's
[Adding it](../crates/crumbkit/README.md#adding-it) section shows, with
serde's and schemars' `derive` features. Step 4 also adds crumbkit-test as
a dev-dependency, as [its README](../crates/crumbkit-test/README.md) shows.

## The starting point

[`start.rs`](../examples/tutorial/src/bin/start.rs) is a plain clap tool.
`pantry jars list` prints the jars in the pantry, and `--shelf` keeps the
jars on one shelf:

```rust
fn main() {
    let cli = Cli::parse();
    match cli.resource {
        Resource::Jars {
            command: JarsCommand::List(args),
        } => {
            for jar in jars() {
                if args.shelf.as_deref().is_none_or(|shelf| jar.shelf == shelf) {
                    println!("{}\t{}\t{}", jar.id, jar.name, jar.shelf);
                }
            }
        }
    }
}
```

## 1. Parse and finish

In [`step_1.rs`](../examples/tutorial/src/bin/step_1.rs), the handler
returns its rows, and crumbkit prints them as a table or as JSON. A row
type derives `Serialize` and `JsonSchema`, and implements `Row` to name the
columns that the table shows. A `Document` names the JSON document that
the command prints, and a `Spelling` says how the tool prints a failure
code:

```rust
#[derive(Serialize, JsonSchema)]
struct Jar {
    /// The jar's id.
    id: u32,
    /// What the jar holds.
    name: &'static str,
    /// The shelf the jar stands on.
    shelf: &'static str,
}

impl Row for Jar {
    const COLUMNS: &'static [&'static str] = &["id", "name", "shelf"];
}

/// The document that `pantry jars list` prints.
const JARS_LIST: Document<Listing<Jar>> = Document::new(
    "pantry",
    "jars-list",
    1,
    "One page of the jars in the pantry.",
);

/// How pantry spells a failure code.
const SPELLING: Spelling = Spelling::COLONS;
```

The root flattens `ViewFlags`, which adds `--json`, `--fields`, `--long` and
`--full`. The list command flattens `Page`, which adds `--limit` and
`--all`:

```rust
struct JarsList {
    /// List only the jars on this shelf
    #[arg(long)]
    shelf: Option<String>,

    /// How many jars to print.
    #[command(flatten)]
    page: Page<20>,
}
```

The handler returns a `Listing`, which holds one page of rows and the
number of rows that match:

```rust
fn list(args: &JarsList) -> Listing<Jar> {
    let jars = jars()
        .into_iter()
        .filter(|jar| args.shelf.as_deref().is_none_or(|shelf| jar.shelf == shelf))
        .collect();
    args.page.first_of(jars).described("jars")
}
```

`main` calls `crumbkit::parse` in place of `Cli::parse`. `parse` prints help
and refusals itself and returns a `Stop` with the exit code, so `main`
returns that code. On success `parse` returns the parsed `Cli` and an
`Output` that holds the command tree that `parse` built, and `finish` prints
the result through that `Output` and returns the exit code:

```rust
fn main() -> ExitCode {
    let parsed = crumbkit::parse::<Cli>(Cli::command(), &Guide::new(), SPELLING);
    let (cli, mut output) = match parsed {
        Ok(parsed) => parsed,
        Err(stop) => return stop.code(),
    };
    match cli.resource {
        Resource::Jars {
            command: JarsCommand::List(args),
        } => {
            let result: Result<_, crumbkit::Error> = Ok(list(&args));
            crumbkit::finish(&mut output, SPELLING, JARS_LIST, result)
        }
    }
}
```

`pantry jars list --limit 2` now ends with the footer
`Showing 2 of 3 jars. --all shows every one.` on standard error.
[ADR-0015] says why a tool keeps its own `main`, and the [output docs]
describe how a result prints.

## 2. A failure with a code and a recovery

In [`step_2.rs`](../examples/tutorial/src/bin/step_2.rs), a shelf without
jars is a failure. The failure type implements `Failure` beside
`std::error::Error`. Its `Code` is stable, so a caller can match on it, and
its `Recovery` names the command to run next:

```rust
/// The code of a shelf that holds no jar.
const SHELF_NOT_FOUND: Code = Code::new("pantry", "shelves", "not_found");

/// No jar stands on the shelf that the caller named.
#[derive(Debug)]
struct ShelfNotFound(String);

impl fmt::Display for ShelfNotFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "no jar stands on the shelf {}", self.0)
    }
}

impl std::error::Error for ShelfNotFound {}

impl Failure for ShelfNotFound {
    fn code(&self) -> &'static Code {
        &SHELF_NOT_FOUND
    }

    fn recovery(&self) -> Recovery {
        Recovery::Run(Call::new(&["jars", "list"]))
    }
}
```

The handler returns the failure, and `main` passes its result to `finish`
unchanged:

```rust
fn list(args: &JarsList) -> Result<Listing<Jar>, ShelfNotFound> {
    let jars: Vec<Jar> = jars()
        .into_iter()
        .filter(|jar| args.shelf.as_deref().is_none_or(|shelf| jar.shelf == shelf))
        .collect();
    if let (Some(shelf), true) = (&args.shelf, jars.is_empty()) {
        return Err(ShelfNotFound(shelf.clone()));
    }
    Ok(args.page.first_of(jars).described("jars"))
}
```

`pantry jars list --shelf middle` exits with 1 and prints the error, a
`help:` line that tells the caller to run `pantry jars list`, and the code
`pantry::shelves::not_found`. With `--json`, it prints the failure as a
JSON document. [ADR-0001] says why every failure carries a code and a next
step.

## 3. A guide

In [`step_3/main.rs`](../examples/tutorial/src/bin/step_3/main.rs), a
`Guide` gives `jars list` a leaf: examples for its help page, its effects,
and the document it prints:

```rust
const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["jars", "list"])
    .examples(&[
        Example::new(&[], "Print the first page of jars"),
        Example::new(&["--shelf", "top"], "Print the jars on the top shelf"),
    ])
    .effects(Effects::read_only())
    .document(JARS_LIST)]);
```

`main` passes the guide to `parse` in place of `Guide::new()`:

```rust
    let parsed = crumbkit::parse::<Cli>(Cli::command(), &GUIDE, SPELLING);
```

`pantry jars list -h` now lists the examples and the line
`Effects: read-only, closed-world.`, and `--help` describes each column
with its field's doc comment. The [examples docs] cover examples and
columns, and the [effects docs] cover effects. A guide also takes goal
headings and help topics ([help docs]).

## 4. Convention tests

The step 3 file declares a test module:

```rust
#[cfg(test)]
mod tests;
```

[`step_3/tests.rs`](../examples/tutorial/src/bin/step_3/tests.rs) calls
crumbkit-test's checks from ordinary tests. Each check walks the command
tree and fails with every problem it finds. `Pages::rendered(&GUIDE)` gives
the names and grammar checks pantry's guide:

```rust
#[test]
fn every_command_and_argument_has_one_name() {
    crumbkit_test::check_names(&Cli::command(), Pages::rendered(&GUIDE), ALLOW);
}

#[test]
fn every_command_is_a_resource_followed_by_a_verb() {
    crumbkit_test::check_grammar(&Cli::command(), Pages::rendered(&GUIDE), &[], ALLOW);
}
```

The file also runs `check_effects`, `check_listings`, `check_guide` and
`check_codes`. With no opt-outs, `check_names` fails, because help shows the
visible alias `ls` as a second name for `jars list`. pantry could make the
alias hidden, which passes the check and keeps `ls` working. pantry keeps it
visible, and opts out of the rule for that command with a reason:

```rust
const ALLOW: &[Allow] = &[Allow::new(
    Rule::OneName,
    "jars list",
    "Help has shown `ls` since the first release, and people type it.",
)];
```

Every check that takes opt-outs reads the same list. A check fails on an
opt-out that names no command or excuses no problem, so an opt-out goes
away when its command does. `check_opt_outs` writes each opt-out with its
reason and its cost to an agent into a golden file. The [opt-outs docs]
describe opt-outs, and [ADR-0025] records why the checks exist.

## 5. Where to go next

- A `show` command that takes many ids returns a [`Batch`], which holds a
  row or a failure for each id, and [`fetch_each`] runs the lookups
  ([output docs]).
- The output budget caps the bytes that one run prints ([output budget docs]).
- A command that changes something declares its effects, and takes
  [`DryRun`] and [`Yes`] ([effects docs]).
- The `complete` feature answers a shell's completion request
  ([completion docs]).
- The crumbkit-usage crate keeps a local log of the shape of each run
  ([usage log docs]).
- crumbkit-test's [README](../crates/crumbkit-test/README.md) lists the
  other checks, such as help sizes, golden help pages, examples and
  recoveries.
- [shelf](../examples/shelf/src/cli.rs) is a fuller tool that uses most of
  crumbkit.

[`Batch`]: ../crates/crumbkit-core/src/batch.rs
[`fetch_each`]: ../crates/crumbkit/src/fetch.rs
[`DryRun`]: ../crates/crumbkit/src/output/dry_run.rs
[`Yes`]: ../crates/crumbkit/src/output/confirm.rs
[ADR-0001]: decisions/api/failures/0001-error-model.md
[ADR-0015]: decisions/api/adoption/0015-lightweight-adoption.md
[ADR-0025]: decisions/api/help/0025-help-checks.md
[output docs]: behaviour/output/README.md
[examples docs]: behaviour/help/examples.md
[effects docs]: behaviour/effects/README.md
[help docs]: behaviour/help/README.md
[opt-outs docs]: behaviour/adoption/opt-outs.md
[output budget docs]: behaviour/output/budget/README.md
[completion docs]: behaviour/parsing/completion.md
[usage log docs]: behaviour/usage-log/README.md
