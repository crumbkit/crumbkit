# Changelog

This file records the notable changes to the published crumbkit crates. The
format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
the crates follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `Row` marks a type that prints as one table row and names its compact
  columns and the columns that `--long` adds.
- `Document` is the typed id of a JSON document, such as
  `shelf/books-list/v1`, and `Document::new` rejects a malformed id at compile
  time.
- `Output` adds the global flags `--json`, `--long`, `--full` and `--fields`
  to a tool's clap parser.
- `finish` takes the tool's `Spelling` and the handler's `Result`, and
  returns the exit code. It prints an `Ok` result as a text table or as a
  JSON document with `schema` and `data`. It prints an `Err` as a failure on
  standard error, with an `error:` line, a `cause:` line for each error in
  its source chain, and `help:`, `code:` and `url:` lines, and with `--json`
  as one `crumbkit/failure/v1` document on standard output. A failure exits
  with code 1. A `Recovery::Run` call that does not resolve, or that has no
  command to resolve against, prints as `None` with the reason `the
  suggested command is not available`. `finish_to` does the same into a
  `Sink` over any two writers.
- `Page` adds the flags `--limit <N>` and `--all` to one command. Its const
  parameter sets the default limit, which is 25 unless the command names
  another. `--limit` parses as a `NonZeroUsize`, so clap refuses 0.
- `Listing` holds one page of rows and the total number of rows that matched.
  `Listing::new` takes a page and a total that the tool counted, and raises
  a total smaller than the page to the number of rows.
  `Listing::from_all` keeps the first page of rows that the tool already
  holds in memory.
- A listing's JSON document holds the total under `total`, beside `data`.
  When a listing shows fewer rows than its total, the text table ends with a
  footer such as `Showing 4 of 6 books. --all shows every one.`
- `Call` names a command by its path from the root and the clap ids of the
  arguments to show, and `Call::render` turns it into text, such as
  `shelf books show <ID>...`, against the `clap::Command` the tool built at
  runtime. `CallError` names the path segment or argument that is missing or
  hidden.
- `Output::set_command` keeps the command tree that the tool parsed against,
  so `finish` can render calls.
- `Listing::next` gives a listing a next step. Its text output ends with a
  `Next:` line after the table and any footer, such as `Next: shelf books
  show <ID>... takes up to 100 ids in one call.` The line is left out when no
  command is set or the call does not resolve, and the JSON document is
  unchanged.
- `Failure` extends `std::error::Error` with a stable `Code`, a `Recovery`
  and an optional url, and `Recovery` names the caller's next step: `Run` a
  `Call`, `Retry`, a `Hint`, or `None` with a reason.
- `Code` holds a failure code's component, area and reason, and
  `Code::new` rejects a malformed segment at compile time. `Spelling` turns a
  code into text, with `Spelling::COLONS`, `DOTS`, `SLASHES` and
  `Spelling::custom`.
- `Error` holds any `Failure`, so `?` converts each of a tool's error types
  into one type. `Error` implements `std::error::Error`, so a tool's `main`
  can pass it on to `Box<dyn std::error::Error>`. `AsFailure` is the sealed
  bound through which `finish` accepts a `Failure` or an `Error`.
- crumbkit-test's `check_codes` fails a test, naming every problem, when two
  of a tool's codes spell the same, or when a spelled code is empty or holds
  a character other than printable ASCII, `!` to `~`.
- crumbkit-test's `check_names`, `check_grammar`, `check_listings` and
  `check_filters` take a tool's `clap::Command`, or any subcommand of it,
  and fail a test, naming every problem, with one line for each problem that
  names the command by its path. `check_names` fails on a visible alias of a
  command or an argument and on a command's short or long flag name, and
  passes hidden aliases. `check_grammar` fails unless every command is a
  resource followed by `list`, `show` or a verb the tool passes, a resource
  is named in lowercase words joined by single dashes, and no argument is
  named `--dry-run`. `check_listings` fails on a `list` command without
  `Page`'s `--limit` and `--all`, and on a `show` command without a required
  positional argument that takes 1 to 100 values. `check_filters` fails on
  an option that accepts many values unless it accepts both repeats and
  commas, and leaves to review the rule that an option that takes one value
  refuses a second, because clap has no documented getter for the settings
  that turn that refusal off. Each check takes a list of exempt command
  paths, such as `"books list"`, and fails on an exemption that names no
  command.
- `Batch` holds the rows for the ids that worked and the failures for the
  ids that did not, and `finish` takes it as a result. In text, the rows
  print as a table on standard output and each failure prints on standard
  error in the form a failed command prints, and a batch with no rows
  prints nothing on standard output. With `--json`, one document holds the
  rows under `data` and the failures under `failures`, which is present
  even when empty, and each entry has the fields of the
  `crumbkit/failure/v1` document without `schema`. A batch with at least
  one failure exits with code 1, also when the reader closed the pipe
  early.
- `Rows::failures` returns a batch's failures as `&dyn Failure`, and `None`
  for every other result.
- `fetch_each` runs a per-id lookup on at most a given number of scoped
  threads and returns one result per id in the order of the ids. With one
  worker, one id or none, it runs every lookup in the calling thread.
- `crates/crumbkit/schemas/failure-v1.json` holds the JSON Schema of the
  `crumbkit/failure/v1` document.
- The `miette` feature, off by default, adds an optional dependency on
  miette 7.6.0 with no miette features enabled.
- With the `miette` feature, `Failure::diagnostic` returns the failure as a
  `miette::Diagnostic`, and `None` by default. `finish` prints each label of
  that diagnostic as an `at:` line after the causes, followed by the source
  line and a caret underline, or by nothing when the span cannot be read.
  With `--json`, the failure document and each batch failure hold the labels
  under `labels`, which is left out when there are none.
- The `crumbkit/failure/v1` schema lists the optional `labels` property.
- With the `miette` feature, a `miette::Report` converts into
  `crumbkit::Error` with the code `crumbkit::report::unchecked`. The report's
  own code follows the message in parentheses, the report's help becomes a
  `Recovery::Hint`, and a report without help has a `Recovery::None`.
- `parse` parses the process's command line against a tool's `Command`
  with the tool's `help::Guide`, which is `&Guide::new()` for a tool without
  one, and returns the tool's parsed type and that command, which `main`
  passes to `Output::set_command`, or the exit code after it printed help,
  the version or a refusal. `parse_from` takes the arguments, and
  `parse_to` also takes a `Sink`. `parse` never calls `process::exit`.
- A command line that clap refuses prints as a failure with the code
  `crumbkit::cli::usage`, clap's message on one line, and a recovery that
  runs the `-h` of the deepest command the caller typed, such as
  `shelf books show -h`, and exits with code 2. A command line that stops at
  a command with subcommands prints ``the command `shelf books` needs a
  subcommand``. With `--json` before any `--`, the refusal prints as one
  `crumbkit/failure/v1` document on standard output.
- A `--fields` name that the row type lacks prints as a refusal with the
  code `crumbkit::cli::usage`, the message
  `--fields names an unknown field 'pages'`, and the hint
  `use the valid fields: author, form, id, title, year`, and exits with
  code 2.
- Through `parse`, `-h` names the root's global flags on one line, such as
  ``Global flags: --json, --fields <FIELD>, --long, --full. `shelf books
  list --help` describes them.``, and `--help` lists them in full. The help
  of a command with a visible subcommand ends with a line such as
  `Next: shelf books <COMMAND> -h`. Help and the version keep clap's colours
  only when standard output is a terminal.
- A `Guide` gives a group's help goal headings that the tool declares,
  such as `Read` and `Change`, in place of clap's `Commands:` list, in `-h`
  and `--help`. `Guide`, `Group` and `Goal` are built with `const fn`s. A
  visible subcommand that no goal names is listed under a final `Commands`
  heading, and a command that the guide does not name keeps clap's list.
  `parse` applies the goal headings to the groups that the typed words pass
  through, and `crumbkit::help::extend` applies them to a tool's whole
  command tree, for a tool that parses with plain clap and for tests.
- crumbkit enables clap's `help` feature, so a tool built with crumbkit
  always has `-h` and `--help`.
- `parse` answers `help` in place of clap's generated `help` subcommand,
  which no longer appears in any `Commands:` list. `shelf help books show`
  and `shelf books help show` print what `shelf books show --help` prints,
  and `shelf books help` prints what `shelf books --help` prints. An unknown
  word such as `shelf books help shwo` is refused with clap's message and
  the recovery `shelf books -h`.
- `Guide::topics` gives a tool help topics, each a `Topic` with a name, a
  one-line summary and plain text, which `parse` reads from the guide.
  `shelf help` lists the topics and `shelf help catalogue` prints one, and
  the root's `-h` and `--help` gain a line such as
  ``Topics: catalogue. `shelf help <TOPIC>` prints one.``
- crumbkit-test's `check_guide` fails a test, naming every problem, on a
  guide group whose path names no command or a command without
  subcommands, on a goal that names a command its group lacks, on a visible
  subcommand that no goal names, and on a subcommand that two goals name.
  It also fails on a topic name that is not lowercase words joined by
  dashes, on a topic name that a root command's name or alias hides, on a
  topic without text, and on two topics with one name.
- crumbkit-test's `check_grammar` fails on a command named or aliased `help`
  at any level, because crumbkit reserves that word for asking for help.
