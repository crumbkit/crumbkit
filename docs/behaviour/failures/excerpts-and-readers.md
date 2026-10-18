# Source excerpts and failure readers

This page covers the source excerpts that the `miette` feature draws in a
failure's text, and how crumbkit-test's checks read the failures that a
failed run printed.

## Source excerpts

With the `miette` feature, each label of a failure's diagnostic prints an
`at:` line, then the lines its span covers with one line before and after,
drawn by crumbkit, so a reader who cannot open the source sees what the label
means. Carets mark the span on its first and last line. The `code:` line
stays last, so `| tail` keeps it.

- The carets measure terminal columns with unicode-width, so they line up
  after wide characters such as CJK text. The `at:` column counts
  characters, as an editor does.
- A span over more than 7 lines prints its first 3 and last 3 lines and one
  row that counts the lines left out. A row that would count one line prints
  that line.
- Labels that follow each other and whose lines overlap or touch print as one
  block: their `at:` lines, then each source line once, with one underline
  row per label that ends with the label's text, so no line prints twice.

Why: ADR-0016, ADR-0001.

## Reading failures back

crumbkit-test's `check_twin`, `check_recoveries` and `check_examples` read a
failed run through one function. It tries the tool's reader first, then
crumbkit's failure document, then its `code:` line, and returns every failure
that the first of them reads.

- A tool that prints failures in a shape of its own implements
  crumbkit-test's
  [`FailureReader`](../../../crates/crumbkit-test/src/failures.rs) and sets
  it with `Examples::failure_reader`. A reader that returns no failure
  leaves the run to crumbkit's failure document and `code:` line.
- crumbkit's failure document is the last `<tool>/failure/v1` document on
  standard output, or every entry of the `failures` of the last document
  that has any, as a batch prints them.
- A failed example's problem names the code and steps that the check read.

`check_recoveries` follows the `Run` recoveries that it reads. When the
reader reads no failure from a run that failed, and the run prints no
failure document and no `code:` line, the check stops with one problem. The
problem says that the tool's reader reads the failure, or names the root in
an opt-out of `Rule::Recoveries`, id `recoveries/follow`. The first
unreadable run stops the check, because every later run would fail the same
way.

Why: ADR-0045, ADR-0088.
