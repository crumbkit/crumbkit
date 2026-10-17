# ADR-0084. A batch's answers follow one budget rule in text and JSON

- Date: 2026-10-18
- Status: accepted
- Supersedes: ADR-0046, ADR-0062, ADR-0068 and ADR-0081, which it amends
- Missed: ADR-0046 and ADR-0081 said that every batch prints its first answer, while ADR-0062 refused each answer of a headed batch's text that passed the budget alone, so the text and the JSON of one batch printed different answers and failures. ADR-0068 let `Room::has_room` return false before the first row when the empty JSON document already reached the budget, so a handler fetched no key. None of these records compared the two forms of one run.

## Context

A headed batch with the keys A, B and C, each larger than the budget,
printed in three shapes. With `--json` it printed A and one failure that
named B and C. Its text, when the handler fetched every key, printed A
when the document always prints its first answer, then refused B and C
with one failure each, whose help said that no flag makes the answer
smaller. At a budget smaller than the empty JSON document, `Room` stopped
the handler before it fetched A, so the JSON printed no answer and named
all three keys, although the text at the same budget printed A. Rerunning
with B alone starts with B as its first answer, so the help that no flag
makes B smaller was wrong.

A batch without `Document::always_print_first` printed a first answer of
any size with `--json`, and refused it in a headed batch's text. A single
bytes result without the mark is refused when it passes the budget.

The message also said "1 bytes" at a budget of 1.

## Decision

- A batch whose document is marked with `Document::always_print_first`
  prints its first answer whole at any budget, in text and with `--json`.
- A batch whose document is not marked refuses its first answer when that
  answer passes the budget alone. Alone means the bytes that the run
  prints for that one row: its header and bytes in a headed batch's text,
  the table or the line of that row in other text, and the document that
  holds that row and no failures with `--json`. The refused answer gets its
  own over-budget failure under its key, among the batch's own failures in
  the order of the keys. The failure of a table row asks for fewer fields
  with `--fields`, as the failure of a single row does. The failure of a
  headed answer has no next step, because the answer prints whole whatever
  the fields. Every later answer and every unfetched key goes into the one
  over-budget failure, whose recovery runs the command with them.
- A first answer that fits alone prints. From the first later answer that
  does not fit, alone or beside the answers before it, that answer, every
  later answer and every unfetched key are left out and named in the one
  failure. No later answer gets a failure of its own.
- Text and JSON follow this one rule. The JSON form is larger, because
  each row carries its field names and the failure counts against the
  budget, so at the same budget the JSON form can refuse a first answer
  that the text prints, or leave out answers that the text prints.
- `Room::has_room` returns true until the batch holds a row, so a handler
  fetches the first answer whatever the budget and `finish` decides on it.
- The cut line and the over-budget messages say "1 byte" at a budget of 1.
- crumbkit-test's `check_output_budget` accepts a run that exits with code
  0 past the budget only when it prints one answer and its document is
  marked with `always_print_first`. A run of an unmarked document must
  print within the budget whatever its exit code.

## Consequences

A run with the keys left out starts with the first of them, so the run
recovery holds for every key that it names. An unmarked batch whose first
answer passes the budget alone now prints no row with `--json`, where it
printed that answer before. An unmarked one-key command that shows a file
refuses a file larger than the budget in both forms, as a single bytes
result does. A tool that wants a large first answer printed marks its
document.

Considered: printing the first answer of every batch whole, with or
without the mark. That option was rejected because a one-key `show FILE`
with an unmarked document would print a file of any size, while a single
bytes result of the same file is refused. Also considered: a failure of its own
for each later answer too large alone, which would need the JSON writer to
skip rows in the middle of a batch and would tell the reader that no flag
makes an answer smaller when a rerun with that key prints it.
