# Checks

crumbkit-test checks a tool against crumbkit's conventions from ordinary
`#[test]` functions. This page lists what each check enforces on a tool and
where its full contract lives. Each check's rustdoc lists every problem it
finds.

## How every check reports

Each check runs on its own, collects every problem it finds and then panics,
as a failed test assertion does, with all of them in one message. A tool can
therefore turn the checks on one at a time. A check that takes opt-outs
accepts `&[]`, or a list of `Allow` values that each name one rule, one
command and a reason, and the check fails on an opt-out that names no
command or excuses no problem. [Opt-outs](../adoption/opt-outs.md) describes
the rules and their ids.

Why: ADR-0025, ADR-0036.

## Checks that read the command tree

These checks read a `clap::Command` without running the tool, so a tool that
parses with clap alone can run them. They read the unbuilt tree, so they see
no arguments on a command that the tool gives to `Command::defer`.

| Check | What it enforces |
| --- | --- |
| [`check_names`](../../../crates/crumbkit-test/src/names.rs) | Help shows one name for every visible command and argument: no visible alias and no flag name on a command. |
| [`check_grammar`](../../../crates/crumbkit-test/src/grammar.rs) | Every command is a resource or a verb in the form `<tool> <resource> <verb>`, resources are dashed lowercase words, and no command below the root is named `help`. |
| [`check_effects`](../../../crates/crumbkit-test/src/effects.rs) | Every visible leaf declares its effects, and `--dry-run`, `--yes`, `--no-input` and `--expect` sit where crumbkit puts them. |
| [`check_listings`](../../../crates/crumbkit-test/src/listings.rs) | Every `list` command has `--limit` and `--all`, and every `show` command takes 1 to 100 ids. |
| [`check_filters`](../../../crates/crumbkit-test/src/filters.rs) | An option that takes many values accepts them both repeated and separated by commas, with one value per use. |
| [`check_guide`](../../../crates/crumbkit-test/src/guide.rs) | The guide's goal headings, leaves, columns and topics match the command tree, and each visible leaf has an example that clap accepts. |
| [`check_exit_codes`](../../../crates/crumbkit-test/src/exits.rs) | The guide's declared exit codes avoid 0, 1, 2 and 126 to 255, and each code and rename is declared once. |
| [`check_codes`](../../../crates/crumbkit-test/src/codes.rs) | No two failure codes spell the same text, and each code is printable ASCII without spaces. |
| [`check_document_ids`](../../../crates/crumbkit-test/src/documents.rs) | Every leaf's document has crumbkit's envelope. |
| [`check_mounts`](../../../crates/crumbkit-test/src/documents.rs) | Each of crumbkit's own commands that a tool mounts sits at its fixed path. |
| [`check_schema_goldens`](../../../crates/crumbkit-test/src/schemas.rs) | The JSON Schema of each document the tool prints matches its golden file. |
| [`check_opt_outs`](../../../crates/crumbkit-test/src/opt_outs.rs) | The report of the tool's opt-outs, budgets and customisations matches `opt-outs.txt` and `opt-outs.json`. |

The help page checks, `check_help_sizes`, `check_help_goldens` and
`check_path_weights`, are described in [Help checks](../help/checks.md).

The tree checks walk the tree as the tool built it, without
`Command::build`, so they never see clap's generated `help` subcommand or
its `--help` and `--version` flags. Every function that takes a
`clap::Command` works on any node of the tree, so a crate or a team can
apply the checks to its own subtree.

`check_guide` takes no opt-outs, because a tool that wants clap's list for a
command leaves that command out of its guide. The help checks take no
opt-outs either, because they take budgets. Each other tree check takes the
tool's opt-outs.

Why: ADR-0015, ADR-0021, ADR-0025, ADR-0036.

## Reasons behind the tree rules

- `check_names` fails on every second name that help shows. Help is what
  agents read and copy, so each visible alias costs help tokens and teaches
  a second spelling. A hidden alias costs neither, and lets a retired name
  keep working for a while or a common wrong guess parse.
- `check_grammar` checks only a name's form, so review checks that each
  resource is plural. A leaf directly under the root passes only with an
  opt-out of `Rule::RootLeaf`, because such a leaf adds a line to the root
  help, its name can collide with a resource that the tool adds later, and
  an agent cannot guess its siblings from the noun and verb pattern.
- `check_filters` requires one value per use, because an option that takes
  several values per use swallows the positional words after it. Review
  checks that an option that takes one value refuses a second, because clap
  has no documented getter for `Command::args_override_self` or for an
  `Arg::overrides_with` that names the argument itself.

Why: ADR-0015.

## Trying the checks with clap alone

`check_listings`, `check_filters`, and `check_names`, `check_grammar` and
the help checks with `Pages::clap` run on a tool that parses with clap
alone, so a tool can try crumbkit-test before it adopts crumbkit's runtime.
Such a tool needs no crumbkit dependency of its own. `check_names` and
`check_grammar` read the tool's guide from the `Pages` they take, and
`Pages::clap` carries no guide, so a guided tool passes `Pages::rendered` or
`Pages::binary` with its guide.

[`Pages::clap`](../../../crates/crumbkit-test/src/pages.rs) renders each
help page in memory as clap prints it to a pipe, without crumbkit's lines
and without a guide. clap's `wrap_help` wraps such a page at the test's
terminal width unless the tool fixes `term_width`, and clap has no public
getter for that width, so crumbkit-test cannot fix it for the tool.
`Pages::rendered` also renders in memory and shares this limit.
`Pages::binary` runs the binary with a cleared environment and its output
on a pipe, so clap wraps its pages at its fixed width of 100 columns. A tool
without examples budgets its paths with `PathBudgets::help_only`, which
reads no golden files, and `check_path_weights` fails when the guide of its
pages gives a command examples, because their output would go uncounted.

crumbkit-test is one crate that depends on crumbkit. The dependency reaches
only the tool's tests, and a feature that left crumbkit out would need
`cfg` attributes across most of the crate's modules to save a few crates.

Why: ADR-0015.

## Checks that run the built binary

These checks run the tool's built binary, so a tool calls them from an
integration test of the binary's own package. [End-to-end
checks](end-to-end.md) describes how each run is isolated and timed.

| Check | What it enforces |
| --- | --- |
| [`check_examples`](../../../crates/crumbkit-test/src/examples.rs) | Every example in the guide's leaves exits with code 0 and prints the output in its golden files. |
| [`check_output_budget`](../../../crates/crumbkit-test/src/output_budget.rs) | Each `--json` example, and each text example of a headed leaf, keeps the output budget's contract when its budget is half the bytes it prints. |
| [`check_command_reference`](../../../crates/crumbkit-test/src/reference.rs) | The checked-in Markdown command reference matches the one written from the binary's `commands list`. |
| [`check_twin`](../../../crates/crumbkit-test/src/twins.rs) | A refused command line fails with its code, and a line that differs from it by one word succeeds. |
| [`check_twins`](../../../crates/crumbkit-test/src/twins/coverage.rs) | Every declared code has a twin, an untwinned opt-out or an entry in the unprovoked list. |
| [`check_recoveries`](../../../crates/crumbkit-test/src/recoveries.rs) | Every recovery that a failure prints parses, leads somewhere new, and every declared failure offers a next step. |

[Twins](end-to-end.md#twins) and [Recovery chains](recovery-chains.md)
describe the last three in more detail. [Golden files](golden-files.md) lists
the golden files that the checks keep.

Why: ADR-0005, ADR-0053.
