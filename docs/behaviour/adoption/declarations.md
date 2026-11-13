# Declarations

This page covers the names that a tool declares in place of crumbkit's: its exit
codes, its document names and the other seams. crumbkit enforces behaviour and
lets a tool declare its own names, so a declaration needs no reason and no
opt-out.

## Behaviour and names

Behaviour is what an agent relies on across tools, such as when a failure
happens, its recovery, the output budget, paging, confirmation and one meaning
for each exit code. A name is a word the tool's own users already read, such as
an exit code or a document name. A tool that declares its own name keeps
crumbkit's behaviour. A departure from behaviour stays an [opt-out](opt-outs.md)
with a stated reason. A new seam needs a name that the existing seams cannot
declare.

Why: ADR-0056.

## The seams

- `Failures::exits` may list one of crumbkit's codes, so a refused command line,
  an over-budget failure or a version conflict exits with the tool's own code
  ([exit codes](../failures/exit-codes.md)).
- `Leaf::passes_on_exit` declares a leaf that passes on a program's exit
  code, which help prints ([child processes](../output/child-processes.md)).
- `Leaf::prints_no_result` declares a leaf whose command prints no result
  on standard output, such as a server that runs until it is interrupted.
  The leaf offers no view flag and help prints its `Output:` line
  ([result shapes](../output/shapes/README.md#a-command-that-prints-no-result)).
- `crumbkit::complete_with`, or `crumbkit::declare_completion_variable` for
  a tool with its own completion handler, declares the name of the
  completion variable,
  which the registration script and the usage log then use
  ([completion](../parsing/completion.md)).
- `Leaf::refusal_alternatives` and `Failures::refusal_alternatives` declare a
  function that builds calls from a refusal of a command line. The first
  call that renders and parses becomes the refusal's recovery in place of
  `-h`, and the rest follow `-h` among the alternatives, except for a
  `--limit` above its bound
  ([refused command lines](../failures/README.md#refused-command-lines)). No
  check reaches these calls before a run, because the function needs a
  refusal, so a tool provokes the refusal in a `check_recoveries` start to
  see them.
- `Leaf::refusal_hint` and `Failures::refusal_hint` declare a function that
  returns a sentence for a refusal of a command line, such as telling the
  caller to put an unknown flag after `--`. The sentence becomes the
  refusal's recovery as a hint before any call, and `-h` and the calls
  follow it among the alternatives, except for a `--limit` above its bound
  ([refused command lines](../failures/README.md#refused-command-lines)).
- A document whose name is not its command's path keeps a name that scripts
    already read ([schemas](../schemas/README.md#document-ids)).

A tool that declares nothing exits with crumbkit's defaults.

Each check reads through the seams. `Tool::failure_exit` takes a failure's code
and returns the exit code that the failure ends the run with, so
`check_exit_codes`, `check_twins`, `check_recoveries` and `check_output_budget`
expect what the run exits with.

Why: ADR-0056, ADR-0001, ADR-0086, ADR-0016, ADR-0035, ADR-0081, ADR-0123,
ADR-0143, ADR-0186.

## Root verbs

A tool whose commands sit directly under the root on purpose declares them
once as root verbs, with one reason for the whole list. Unlike the other
declarations it takes a reason, because a root verb departs from the
`<tool> <resource> <verb>` grammar for good.

- `Help::root_verbs(verbs, reason)` is the recommended declaration. Root
  help then lists the verbs under a `Verbs` heading
  ([goal headings](../help/headings/README.md#root-verbs)).
- [`Allow::root_verbs(verbs, reason)`](../../../crates/crumbkit-test/src/allow.rs)
  in the opt-out list is the alternative for a tool that does not want the
  heading. Every check except `check_grammar` and `check_opt_outs` ignores
  it.

[`check_grammar`](../../../crates/crumbkit-test/src/grammar.rs) reads both:

- each declared root verb passes the root-leaf rule without an opt-out,
  and a root leaf that is not declared still needs an opt-out from
  `Rule::RootLeaf`;
- a resource at any depth whose name or alias is a root verb's name or
  alias fails, and this rule has no opt-out;
- a declared verb that names no visible command without subcommands
  directly under the root fails as a stale opt-out does, and so does a
  declaration without a verb;
- a declaration that names one verb twice fails, because root help would
  list it twice;
- a tool that declares root verbs more than once, on the tool record and with
  `Allow::root_verbs` or with `Allow::root_verbs` twice, fails, because root
  help lists only the tool record's verbs and one reason covers the choice.

The root's `-h` budget still covers the heading and each root verb's line
([help checks](../help/checks.md)).

Why: ADR-0165.

## The declarations section

crumbkit-test's `check_opt_outs` lists each declaration after the opt-outs,
under the declarations, with no reason and no cost, so a reviewer sees in one
place how far a tool has moved from crumbkit's defaults. The section lists each
exit that lists one of crumbkit's codes, each document whose name is not its
command's path (as a `document-name` declaration beside the name that its
command's path gives), each leaf that passes on an exit code, each leaf that
declares refusal alternatives (as `refusal-alternatives`) or a refusal hint (as
`refusal-hint`), the tool record's refusal function and hint function once each
for every run, and each leaf whose row type declares `Row::LINE` (as a
`text-line` declaration that names the form of the line), and each leaf whose
row type declares `Row::CHILDREN` (as a `children` declaration that names the
field and quotes any `Children::help`), and each example that needs a
precondition (as an `example-precondition` declaration that names the example's
command line and the names it needs). The root verbs follow as one `root-verbs`
declaration for each declaration, which names each verb's command line and gives
the tool's reason on a `Reason:` line. When a tool declares root verbs, the line
that counts the declarations no longer says that they need no reason.

The section is left out when the tool declares nothing, and the JSON holds it
as a `declarations` array beside `data`. The report does not list the
completion variable, because `complete_with` or `declare_completion_variable`
sets it for the process and the tool record does not hold it.

Why: ADR-0056, ADR-0036, ADR-0035, ADR-0064, ADR-0081, ADR-0086, ADR-0120,
ADR-0123, ADR-0163, ADR-0165, ADR-0186.
