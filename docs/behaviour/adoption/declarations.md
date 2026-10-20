# Declarations

This page covers the names that a tool declares in place of crumbkit's: its
codes, its exit codes, the spelling of its commands and the other seams.
crumbkit enforces behaviour and lets a tool declare its own names, so a
declaration needs no reason and no opt-out.

## Behaviour and names

Behaviour is what an agent relies on across tools, such as when a failure
happens, its recovery, the output budget, paging, confirmation and one
meaning for each exit code. A name is a word the tool's own users already
read, such as a failure code or an exit code. A tool that declares its own
name keeps crumbkit's behaviour. A departure from behaviour stays an
[opt-out](opt-outs.md) with a stated reason. A new seam needs a name that
the existing seams cannot declare.

Why: ADR-0056.

## The seams

- `Guide::codes` lists [`Rename`](../../../crates/crumbkit/src/help/rename.rs)
  values, each of which prints the tool's code in place of one of crumbkit's
  in text, in JSON, in the usage log and in the mark of a cut result.
- `Leaf::codes` lists renames for the runs of one command. A run of that
  command reads the leaf's renames first and the guide's for every other
  code ([codes per command](#codes-per-command)).
- `Guide::spelling` sets the
  [`Spelling`](../../../crates/crumbkit/src/code.rs) that every code
  prints in, such as segments joined with `.`. A guide that sets none
  prints codes with `Spelling::COLONS`, so a tool with colon codes declares
  nothing
  ([codes and their spelling](../failures/README.md#codes-and-their-spelling)).
- `Guide::exits` may list one of crumbkit's codes or the tool's rename of it,
  so a refused command line, an over-budget failure or a version conflict
  exits with the tool's own code ([exit codes](../failures/exit-codes.md)).
- `Leaf::passes_on_exit` declares a leaf that passes on a program's exit
  code, which help prints ([child processes](../output/child-processes.md)).
- `crumbkit::complete_with` declares the name of the completion variable,
  which the registration script and the usage log then use
  ([completion](../parsing/completion.md)).
- `Leaf::refusal_alternatives` and `Guide::refusal_alternatives` declare a
  function that builds calls from a refusal of a command line. The first
  call that renders and parses becomes the refusal's recovery in place of
  `-h`, and the rest follow `-h` among the alternatives, except for a
  `--limit` above its bound
  ([refused command lines](../failures/README.md#refused-command-lines)). No
  check reaches these calls before a run, because the function needs a
  refusal, so a tool provokes the refusal in a `check_recoveries` start to
  see them.
- A document whose name is not its command's path keeps a name that scripts
  already read ([schemas](../schemas/README.md#document-ids)).
- `Guide::dashed_forms` lets a command keep a name in another style
  ([dashed forms](#dashed-forms)).

A tool that declares nothing prints crumbkit's codes and exits with
crumbkit's defaults.

Each check reads through the seams. `Guide::printed` takes a command path
and returns the code that a run of that command prints for one of
crumbkit's codes, and `Guide::failure_exit` takes the same path and returns
the exit code a failure ends the run with, so `check_exit_codes`,
`check_twins`, `check_recoveries` and `check_output_budget` expect what the
run prints. A path that no leaf with renames names, such as `&[]`, gives the
guide's renames alone.

Why: ADR-0056, ADR-0060, ADR-0001, ADR-0086, ADR-0016, ADR-0035, ADR-0081.

## Codes per command

`Leaf::codes` gives one command its own codes for crumbkit's failures, such
as a command that prints `ledger::rows::moved` for a version conflict while
the rest of the tool prints `ledger::rows::stale`. `parse` finds the leaf of
the parsed command and keeps its renames in the `Output` that it returns, so
`finish`, `stream`, the usage log and the trace print and record the leaf's
code. The leaf's renames come first, and the guide's renames cover every
code that the leaf does not rename. An exit that lists a leaf's code covers
that failure only in runs of that command.

A leaf cannot rename `<tool>::cli::usage`. crumbkit refuses a command line
before it knows which command the line names, so the refusal prints the
guide's usage code for every command. `Leaf::codes` is a `const fn` that
panics on such a rename, so a `const` guide with one fails to compile.

`Output::new` takes the argv and finds the leaf in the same way, so a tool
that parses with clap alone prints the leaf's renames too.
`RuntimeLeaf` has no `codes` setting, and a runtime leaf prints the guide's
renames.

`check_exit_codes` reports a crumbkit code that one list of renames names
twice, and a tool code that renames of two different crumbkit codes print,
across the guide and every leaf, because a caller reads a code as one
failure. It also checks the exits once for each leaf with renames, and
reports two exits that list codes that a run of that command prints as one
code.

Why: ADR-0060, ADR-0076.

## Dashed forms

`Guide::dashed_forms` declares that a command named in another style, such
as `reading_lists` or `ReadingLists`, keeps that name in help, completion and
the usage log and also answers to its dashed form, such as `reading-lists`.
An agent that types crumbkit's spelling then reaches it. The declaration is
opt-in, because a tool that does not want a second spelling of its commands
keeps clap's refusal of every unknown word. Help, the guide and completion
candidates show only the tool's own name, because a second visible spelling
costs help tokens as a visible alias does.

The declaration lives on the guide, and `parse`, `complete`,
`complete_with`, `complete_with_words`, `complete_to` and `command_path` all
take the guide, so every entry point reads the same declaration. `parse`
rewrites each typed dashed form once
([parsing](../parsing/README.md#dashed-forms-of-command-names)).

Without the declaration, `check_grammar` fails every resource whose name is
not lowercase dashed words unless the tool opts out of `Rule::ResourceName`
with a reason. With it, `check_grammar` fails only a resource whose name maps
to no dashed form, such as `2books`, or whose form is `help` or is taken by
another command beside it, because the agent's spelling then reaches
nothing, and such a resource still needs the opt-out. Verbs follow the same
mapping, and a verb whose form another command takes has no opt-out, because
the tool declares its verbs.

Why: ADR-0015, ADR-0056, ADR-0036.

## The customisations section

crumbkit-test's `check_opt_outs` lists each declaration after the opt-outs,
under the customisations, with no reason and no cost, so a reviewer sees in
one place how far a tool has moved from crumbkit's defaults. The section
lists the guide's spelling when it prints crumbkit's usage code other than
`Spelling::COLONS` does (as a `code-spelling` customisation), each code
rename of the guide and of each leaf, each exit that lists one of crumbkit's
codes or a rename of one, each document whose name is not its command's path
(as a `document-name` customisation beside the name that its command's path
gives), each leaf that passes on an exit code, each leaf that declares
refusal alternatives, the guide's refusal function once for every run, and
each leaf whose row type declares `Row::LINE` (as a `text-line`
customisation that names the form of the line), and each leaf whose row
type declares `Row::CHILDREN` (as a `children` customisation that names the
field). A leaf's code rename, and an
exit that lists a code that only a leaf renames, apply to that leaf's
command line. The dashed-forms declaration is listed once, followed by each
command named in another style as a `resource-name` or a `verb-name` with
the dashed form it also answers to.

The section is left out when the tool declares nothing, and the JSON holds it
as a `customisations` array beside `data`. The report does not list the
completion variable, because `complete_with` sets it for the process and the
guide does not hold it.

Why: ADR-0056, ADR-0060, ADR-0036, ADR-0035, ADR-0064, ADR-0081, ADR-0086,
ADR-0120.
