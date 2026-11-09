# ADR-0186. Three seams without a user removed

- Date: 2026-11-09
- Status: accepted
- Supersedes: ADR-0060, and ADR-0001, ADR-0015, ADR-0046, ADR-0056, ADR-0076, ADR-0079, ADR-0081, ADR-0085, ADR-0086, ADR-0088, ADR-0100, ADR-0136, ADR-0177, ADR-0180 and ADR-0185, which it amends
- Missed: ADR-0177 exempted a declared code that a `Rename` prints from the twin check, and it did not ask whether any tool declared a `Rename`. ADR-0180 gave renames, the spelling of codes and dashed forms a place on the new records, and it did not ask whether any tool used them. ADR-0185 justified whole-check skips by saying that the report lists each skip, and it did not consider that the report's own check could be skipped. It also gave `check_codes` a setter without asking whether that check could still fail.

## Context

crumbkit let a tool replace three of its conventions.

- `Failures::codes` and `Leaf::codes` held `Rename` values that printed a
  tool's code in place of one of crumbkit's, and `Tool::printed` returned the
  code that a command's run printed.
- `Failures::spelling` and `Spelling::custom` printed every code in a form that
  the tool chose.
- `Tool::dashed_forms` let a command named in another style also answer to its
  dashed form, and `dashed_form`, `dashed_name` and `is_dashed` were public
  helpers for it.

No tool uses any of these features. Each one still adds parameters, fields and
branches to how a code prints, how a command line parses and how a check reads
a tool record. The completion functions and `command_path`, for example, took
the tool record only to read its dashed forms.

While the project's own tools are crumbkit's only users, the API is judged only
on being the best we know how to make. A feature without a user is removed, and
no deprecated alias stays.

## Decision

- `Rename`, `Failures::codes`, `Leaf::codes`, `Tool::printed` and their getters
  are removed. A tool's failures print crumbkit's own code with the tool's name
  as the component. `Tool::failure_exit` takes only the code, because a command
  can no longer change what a code prints.
- `Spelling::custom`, `Failures::spelling` and its getter are removed. Every
  code prints in `Spelling::COLONS`. The `Spelling` type, `Spelling::COLONS`,
  `Spelling::for_tool`, `Spelling::spell` and `Spelling::split` stay, because
  tools call them.
- `Tool::dashed_forms`, `Tool::get_dashed_forms`, `dashed_form`, `dashed_name`
  and `is_dashed` are removed from crumbkit. A command answers only to its own names and aliases. crumbkit-test keeps a
  private rule for the lowercase dashed names that `check_grammar` and the topic
  check require.
- `complete`, `complete_with`, `complete_with_words`, `complete_to` and
  `command_path` no longer take the tool record, because they read only the
  removed settings from it.
- `check_codes`, `Check::Codes` and `Suite::codes` are removed. `Code::new` and
  `Code::runtime` accept only lowercase ASCII letters, digits, `_` and `-` in a
  segment, so with `::` as the one separator every code passes the check's
  rules. The `CODES` lists stay, because `check_twins` and `check_recoveries`
  read them.
- `Suite::skip` panics on `Check::OptOuts`, because that check is what lists
  every skip and departure.
- `check_opt_outs` no longer lists the `code-spelling`, `code-rename`,
  `dashed-forms`, `resource-name` and `verb-name` declarations. The report keeps
  the id `opt-outs/v2`, because no published release has printed that id.
- The usage report accepts the characters of a code printed with `::`, and it
  refuses a code that holds `.` or `/`.

ADR-0060 is superseded whole. The other records named above keep their
decisions except where they describe these features.

## Consequences

A tool cannot print its own code for one of crumbkit's failures, choose another
separator for its codes, or give a command a second dashed name. A tool that
lists one of crumbkit's codes in an `Exit` still gets its own exit code for that
failure. A resource whose name is not lowercase dashed words needs an opt-out
of `Rule::ResourceName` or a new name, and a verb with such a name needs a new
name.

The features had no user and can be restored from git history. A tool that
needs one again brings it back through a new record.

Considered: deprecating the items for one release, which no user needs; hiding
them with `#[doc(hidden)]`, which keeps their cost in the code and the tests;
keeping `check_codes` for tools that already call it, which keeps a check that
cannot fail; keeping a public `is_dashed` for `examples/reading`, which defines
its own.
