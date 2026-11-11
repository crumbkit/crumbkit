# ADR-0202. RUST-8 counts code lines

- Date: 2026-11-11
- Status: accepted
- Supersedes: ADR-0010, which it amends

## Context

RUST-8 limited every `.rs` file to 600 lines, and `cargo xtask check`
counted every line of the file. Comments made up 35% of the source lines
and blank lines 7%, so a well-documented module reached the limit at about
350 lines of code. The limit therefore pushed authors to split small
modules or to write fewer comments. Three feature commits moved `impl`
blocks of 50 to 75 lines into new files for this reason.

## Decision

- RUST-8 limits the code lines of a `.rs` file, and the limit stays 600.
- A code line is a line that holds at least one character other than
  whitespace and whose first such characters are not `//`. Blank lines,
  line comments and doc comments are not counted. A line of code with a
  trailing comment is counted.
- The check reads the start of each line and does not parse Rust. It counts
  every line of a block comment as code, and it skips a line of a
  multi-line string literal that starts with `//`. No `.rs` file held
  either case on this date.

## Consequences

Documentation no longer uses up the limit. The largest file had 497 code
lines on this date, so no file failed under the new count. A file can reach
about 1,000 lines in all, which a person or an agent still reads whole. The
refusal line says what the number counts. A file that uses a block comment
reaches the limit sooner than its code warrants, and a tokenizer replaces
the count once that happens.

Considered: keeping the count of every line, and a tokenizer that also
skips block comments.
