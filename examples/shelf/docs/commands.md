# Commands of `shelf`

This reference is generated from the help of `shelf`. Edit the help in the tool's code, and not this file.

## `shelf`

Browse a small catalogue of public-domain books

### Commands

- [`shelf books`](#shelf-books): Work with the books in the catalogue
- [`shelf commands`](#shelf-commands): Explore shelf's commands
- [`shelf schemas`](#shelf-schemas): Explore the JSON documents that shelf prints
- [`shelf self`](#shelf-self): Review shelf's own use

### Arguments

- `--json`: Print the result as a JSON document. The default prints text. Works after every subcommand.
- `--fields <FIELD>`: Print only these comma-separated fields, in this order. The default prints the compact columns. Takes more than one value. Works after every subcommand.
- `--long`: Add the long columns after the compact ones. The default prints only the compact columns. Works after every subcommand.
- `--full`: With --json, print every field of every row. The default prints only the fields of the columns. Works after every subcommand.
- `--no-input`: Fail instead of asking a question. Works after every subcommand.

## `shelf books`

Work with the books in the catalogue

### Commands

- [`shelf books list`](#shelf-books-list): List the books in the catalogue, in id order
- [`shelf books show`](#shelf-books-show): Show the books with the given ids, in the order given
- [`shelf books summarize`](#shelf-books-summarize): Describe the books with the given ids, or the whole catalogue, in one sentence
- [`shelf books remove`](#shelf-books-remove): Remove the books with the given ids from the shelf, so list and show skip them

## `shelf books list`

List the books in the catalogue, in id order

### Arguments

- `--limit <N>`: Print at most this many rows, up to 100. Default: `4`.
- `--all`: Print every row

### Examples

Print the first page of books.

```sh
shelf books list
```

Print every book.

```sh
shelf books list --all
```

### Columns

- `id`: The book's id, unique within the catalogue.
- `title`: The book's title.
- `author`: The book's author.
- `year`: The year the book was first published.

### Columns that `--long` adds

- `form`: The book's literary form.
- `pages`: The number of pages in a common edition.
- `reading`: The time it takes to read the book at two minutes a page.

Effects: read-only, closed-world.

## `shelf books show`

Show the books with the given ids, in the order given

### Arguments

- `<ID>`: The ids of the books to show, separated by spaces. Required, or give `--from` in its place. Takes more than one value.
- `--from <FILE>`: Read the ids from this file, one id per line

### Examples

Show books 2 and 5.

```sh
shelf books show 2 5
```

### Columns

- `id`: The book's id, unique within the catalogue.
- `title`: The book's title.
- `author`: The book's author.
- `year`: The year the book was first published.

### Columns that `--long` adds

- `form`: The book's literary form.
- `pages`: The number of pages in a common edition.
- `reading`: The time it takes to read the book at two minutes a page.

Effects: read-only, closed-world.

## `shelf books summarize`

Describe the books with the given ids, or the whole catalogue, in one sentence

### Arguments

- `<ID>`: The ids of the books to describe, separated by spaces; without ids, the whole catalogue. Takes more than one value.

### Examples

Summarize the whole catalogue.

```sh
shelf books summarize
```

Summarize books 2 and 5.

```sh
shelf books summarize 2 5
```

Effects: read-only, closed-world.

## `shelf books remove`

Remove the books with the given ids from the shelf, so list and show skip them

### Arguments

- `<ID>`: The ids of the books to remove, separated by spaces. Required. Takes more than one value.
- `--dry-run`: Report what the command would change, and change nothing
- `--yes`: Proceed without asking for confirmation

### Examples

Print the book that removing book 2 would remove.

```sh
shelf books remove 2 --dry-run
```

Remove book 2 without asking.

```sh
shelf books remove 2 --yes
```

### Columns

- `id`: The book's id, unique within the catalogue.
- `title`: The book's title.
- `author`: The book's author.
- `year`: The year the book was first published.

### Columns that `--long` adds

- `form`: The book's literary form.
- `pages`: The number of pages in a common edition.
- `reading`: The time it takes to read the book at two minutes a page.

Effects: destructive, idempotent, closed-world.

Writes: applies all items or none.

## `shelf commands`

Explore shelf's commands

### Commands

- [`shelf commands list`](#shelf-commands-list): List a branch of shelf's commands with their arguments, examples and columns

## `shelf commands list`

List a branch of shelf's commands with their arguments, examples and columns

### Arguments

- `<PATH>`: The names of the commands below the root that lead to the branch, such as `books list`. Takes more than one value.
- `--limit <N>`: Print at most this many rows. Default: `25`.
- `--all`: Print every row

### Examples

List every command.

```sh
shelf commands list
```

List the books command and the commands below it.

```sh
shelf commands list books
```

Print books list with its arguments, examples and columns as JSON.

```sh
shelf commands list books list --json --full
```

### Columns

- `path`: The command line that runs the command, such as `shelf books list`.
- `about`: The command's one-line description, or its full description when it has no one-line description.

Effects: read-only, closed-world.

## `shelf schemas`

Explore the JSON documents that shelf prints

### Commands

- [`shelf schemas list`](#shelf-schemas-list): List the JSON documents that shelf prints, with a description of each
- [`shelf schemas show`](#shelf-schemas-show): Print the JSON Schema of each document

## `shelf schemas list`

List the JSON documents that shelf prints, with a description of each

### Arguments

- `--limit <N>`: Print at most this many rows. Default: `25`.
- `--all`: Print every row

### Examples

List every document that shelf prints.

```sh
shelf schemas list
```

### Columns

- `id`: The document's id, such as `shelf/books-list/v1`, which `schemas show` takes.
- `kind`: output when a command prints the document with its schema id, unwrapped when a command prints it without the envelope that holds its schema id, and file when it is a file format that users write and no command prints.
- `form`: stream when a command prints the document as a stream, one document on each line as each row arrives, and document when a command prints it once or it is a file format.
- `description`: What the document holds, in one line.

### Columns that `--long` adds

- `commands`: The command lines that print the document. It is empty for the failure document, which every command prints when it fails with --json, and for a file format.

Effects: read-only, closed-world.

## `shelf schemas show`

Print the JSON Schema of each document

### Arguments

- `<ID>`: The ids of the documents that `schemas list` lists, separated by spaces. Required. Takes more than one value.

### Examples

Print the JSON Schema of the document that books list prints.

```sh
shelf schemas show shelf/books-list/v1
```

### Columns

- `id`: The document's id, such as `shelf/books-list/v1`.
- `schema`: The document's 2020-12 JSON Schema.

Effects: read-only, closed-world.

## `shelf self`

Review shelf's own use

### Commands

- [`shelf self usage`](#shelf-self-usage): Report the patterns in shelf's usage log where callers waste effort
- [`shelf self figures`](#shelf-self-figures): Work with the figures of each command in shelf's usage log

## `shelf self usage`

Report the patterns in shelf's usage log where callers waste effort

### Arguments

- `--since <DURATION>`: How far back to read the usage log, such as 12h, 7d or 2w. Default: `7d`.
- `--log <FILE>`: Read this usage log instead of the tool's own

### Examples

Report the findings of the last 7 days.

```sh
shelf self usage
```

Report the findings of the last day.

```sh
shelf self usage --since 1d
```

### Columns

- `finding`: What the pattern is.
- `runs`: The runs that show the pattern.
- `command`: The command path below the tool's name, such as `books show`, or the tool's name for its root command.
- `suggestion`: What the pattern suggests, or the figure that raised it.

Effects: read-only, closed-world.

## `shelf self figures`

Work with the figures of each command in shelf's usage log

### Commands

- [`shelf self figures list`](#shelf-self-figures-list): List the figures of each command in shelf's usage log

## `shelf self figures list`

List the figures of each command in shelf's usage log

### Arguments

- `--since <DURATION>`: How far back to read the usage log, such as 12h, 7d or 2w. Default: `7d`.
- `--log <FILE>`: Read this usage log instead of the tool's own
- `--limit <N>`: Print at most this many rows. Default: `25`.
- `--all`: Print every row

### Examples

Print the figures of each command over the last 7 days.

```sh
shelf self figures list
```

Print the figures of the commands in a copy of the usage log.

```sh
shelf self figures list --log usage.jsonl
```

### Columns

- `command`: The command path below the tool's name, such as `books show`, or the tool's name for its root command.
- `runs`: The runs that did not print a help page.
- `help_pages`: The runs that printed the command's help page.
- `help_before`: The help pages that each run's session read since its last run that printed no help page, summed over the runs.
- `failed`: The runs that failed.
- `p95_out_bytes`: The 95th percentile of the bytes that the runs wrote to standard output, or null when no run printed a result.

### Columns that `--long` adds

- `refused`: The runs whose command line was refused.
- `p95_ms`: The 95th percentile of the runs' wall time in milliseconds, or null when every run printed help.
- `typed_full`: The runs that typed `--full`.
- `typed_all`: The runs that typed `--all`.
- `typed_long`: The runs that typed `--long`.

Effects: read-only, closed-world.
