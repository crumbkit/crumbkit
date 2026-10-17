# Recovery chains

[`check_recoveries`](../../../crates/crumbkit-test/src/recoveries.rs) runs
command lines that fail and follows the recoveries that their failures
print, so an agent that follows a recovery reaches somewhere new. This page
states the rules of the check. The rustdoc of `check_recoveries`,
[`Recoveries`](../../../crates/crumbkit-test/src/recoveries.rs) and
[`Failing`](../../../crates/crumbkit-test/src/recoveries.rs) gives the API.

## Starts and steps

The tool's test lists the command lines that fail, each with the code it
fails with. The check follows each `run` recovery and alternative that the
failures print, up to 3 steps after the start by default, which
`Recoveries::steps` changes. A run that prints several failures, such as a
batch, starts one branch for each. A start passes when any of its failures
carries its code, because a batch reports every id that failed.

A step fails the check in these cases:

- The parser refuses the step.
- The step fails with the code of the failure it follows, or of any earlier
  failure of its chain, because an agent that follows the recovery would
  meet a failure it has already seen. An opt-out from `Rule::SameCode`,
  id `recoveries/same-code`, names the command that such a step runs, and
  marks a step that a person or an agent must complete by hand first.

A failure with one of the tool's declared codes that offers no `run`,
`retry` or `hint` step fails the check, at the start and after any step,
because an agent that reads it has nothing to act on. A `none` step with a
reason counts as no next step. An opt-out from `Rule::NextStep`, id
`recoveries/next-step`, that names the failing command declares the
exception that the [failure model](../failures/README.md) allows.

A failure from a run that writes crumbkit's warning about a recovery that
does not render is not reported again as a dead end. A code that the tool
does not declare and crumbkit does not own ends its chain, and the report
counts such codes. crumbkit owns its own codes, the codes that the guide
prints in place of them, and the codes of its add-on crates, such as crumbkit-child's
`<tool>::child::failed` and crumbkit-aliases's
`<tool>::aliases::unreadable`. The tool's output names the tool in each of
these codes, so the check follows a code in the tool's component that the
tool does not declare only when it is one of these codes spelled with the
tool's name. Any other undeclared code in the tool's component fails the
check, such as `shelf::books::lost` beside `shelf::books::not_found`, or
`shelf::cache::stale` and `shelf::cli::bogus`. A test of crumbkit-test
compares its list of the add-on crates' codes with the codes that
crumbkit-child and crumbkit-aliases declare.

The check finds the component of a printed code from the text the
spelling writes before a code's area. For each component of a declared
code, it spells a code of that component and takes the text before the
area, such as `shelf::` in `Spelling::COLONS` or `shelf.` in a custom
spelling that joins the segments with `.`. A printed code that begins with
that text, and has more text after it, is in that component. The check
takes no such text from a custom spelling that writes the area first,
writes the reason before the area, changes the area's letters, or leaves
out the component's name, such as `E:books:not_found`. It also takes none
when the text ends in a lowercase letter, a digit or `_`, as in
`shelf_books_not_found` or `shelfbooksnot_found`, because `shelf_` would
also begin a code of a component `shelf_cache`. The check then counts every
undeclared code of that component as one it does not follow.

Every declared code is provoked by a chain or listed in `Unprovoked` with
the reason no test can provoke it. A tool passes the same list to
[`check_twins`](end-to-end.md#twins).

Why: ADR-0053, ADR-0001, ADR-0079.

## Runs of a chain

The runs of one chain share fresh XDG directories, so a step sees what the
earlier runs wrote, such as a confirmation that a remove needs. Each start
and step runs in the working directory, standard input and XDG fixture that
[End-to-end checks](end-to-end.md#isolation-of-each-run) describes.

`Failing::env` sets or removes environment variables for every run of the
chain, for a failure that the environment provokes. Its entries win over
`Examples::env` and the fresh XDG directories, and the report names the
start with them, such as `tool books list (with DEPTH=1)`.

Why: ADR-0053, ADR-0005.

## Placeholders

A placeholder in a step, such as `<ID>...` or `<path>`, takes the first
value of three sources:

1. the chain's own `Failing::values`;
2. a field of the failure that offers the step;
3. the shared `Recoveries::values`.

Each chain can therefore fill `<path>` with the path its own failure named.

- A table value matches the placeholder's name exactly, case included,
  because a tool may write its placeholders in either case.
- A field matches with capitals lowered and `-` read as `_`, because field
  names are always lowercase and clap writes its placeholders in capitals.
- A null, list or object field fills no placeholder, because an agent cannot
  pass JSON as one word of a command line. The next source fills it, or the
  check reports the placeholder without a value.
- A placeholder can sit anywhere in a word, so `<plugin>.<capability>`
  fills two placeholders and keeps the `.`. A `<` that starts no
  placeholder stays literal text.
- The fields come with the failure that the check reads: the `fields` of
  crumbkit's failure document. The code, steps and fields therefore always
  belong to one failure.

Each placeholder that a field fills is a finding of `Rule::FieldValue`, id
`recoveries/field-value`, and the report shows it under the step. An agent
that follows such a recovery must copy the value itself, while a recovery
built with `Call::value` carries it. A tool that cannot carry the value
opts out with an `Allow` that names the command whose failure printed the
recovery. A start's own value that wins over a field is no finding.

Why: ADR-0053.

## Steps that take every value of a field

A step that runs with every value of a list field, such as the `all` step
of a batch's [over-budget failure](../output/budget/README.md), runs once
with every value of the field, each as its own word, in place of the first
word that is one placeholder, such as `<ID>...`. That fill is no finding,
because the step names the field its values come from. A field that is
missing or holds no value fails the check.

When the run with every value fails again with the same code and its
failure's field holds fewer values than the run was given, the step is no
`Rule::SameCode` finding, because the run answered some keys and the chain
makes progress. A run that leaves as many values as it was given is a
finding, because the agent learns nothing new.

Why: ADR-0053, ADR-0046.

## Reading a step's words

The check reads a step's words with its own `split`, which reads single and
double quotes as a POSIX shell does and refuses a `$` or a backtick between
double quotes. A tool therefore need not re-quote its hints with
`help::quote`. `split` is private to crumbkit-test, because only the check
reads a recovery's words, and crumbkit offers only `help::quote`.

Why: ADR-0053.

## The golden file

The check pins the chains, codes and outcomes in the golden file
`recoveries.txt`, without messages, so a changed recovery shows in review.
[Golden files](golden-files.md) describes how it is updated.

Why: ADR-0053, ADR-0004.
