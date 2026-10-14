# ADR-0015. Lightweight adoption

- Date: 2026-10-07
- Status: accepted

## Context

crumbkit wants tools to take up its conventions, and a tool with an
existing clap command tree and `main` will not rewrite them to do so. Each
concept and dependency that crumbkit asks a tool to take is a reason not
to adopt it. The conventions also had to cover what agents read: help,
command names, recoveries and refusals.

## Decision

- Each part of crumbkit worked on its own and followed an existing
  convention, so a tool keeps its clap derive types and its `main` and
  adopts only the parts it wants. `Failure` extends `std::error::Error`,
  `Output` is flattened with clap, results derive `Serialize` and
  `JsonSchema`, and each crumbkit-test check is a free function called
  from an ordinary `#[test]`.
- crumbkit's argument types clear the command's about, because clap
  otherwise copies the type's rustdoc into the help of a command that has
  no doc comment.
- crumbkit offered no `run` one-liner, because a hand-wired `main` is a
  few lines, a `run` could not call crumbkit-aliases, and a `run` that
  started the usage log would let feature unification start the log in
  tools that never asked for it.
- `parse` returned the exit code instead of exiting, so every run ends by
  returning from `main` and tests run in memory.
- Help, `Next:` lines and recoveries were rendered from the command tree
  that the tool builds at run time, which may hide commands, and the tool
  passes that tree to `Output::set_command` itself. A forgotten call
  shows as a debug-build warning that crumbkit-test's checks fail on,
  because the alternatives that make the call impossible to forget change
  every handler. `finish` builds only the branch that a call names, so the
  work of a run grows with the command typed. A hidden command or flag
  prints only where the caller typed it.
- crumbkit-test's tree checks were free functions that take a
  `clap::Command` and plain settings and report every problem at once.
  Several run on a tool that parses with clap alone, so a tool can try
  crumbkit-test before adopting crumbkit's runtime.
- `check_names` failed on every second name that help shows, because each
  visible alias costs help tokens and teaches a second spelling. A hidden
  alias costs neither.
- `check_grammar` required `<tool> <resource> <verb>`, so an agent can
  guess a command's siblings. A leaf under the root needs an opt-out with
  a reason.
- `Guide::dashed_forms` let a command named in another style also answer
  to its dashed form, so an agent that types crumbkit's spelling reaches
  it and the tool keeps the name its users know. It is opt-in, because
  some tools want clap's refusal of every unknown word. `parse` rewrites
  the typed word before anything reads it, because adding a hidden alias
  to every command would make each run's work grow with the tree. The
  declaration sits on the guide, so every entry point reads the same
  setting.
- `check_listings` and `check_filters` pinned the flags of `list` and
  `show` and the form of multi-value options, because an option that takes
  several values per use swallows the positional words after it.
- crumbkit's own commands sat at fixed paths, so an agent runs them by
  the same words in every tool.
- The first segment of a code names the component that owns it, so a
  crate keeps its codes in any binary that includes it.
- `fetch_each` ran per-id lookups on a bounded number of threads, with no
  default worker count, for tools whose backend has no batch lookup.

## Consequences

Every function that takes a `clap::Command` works on any node, so a team
can apply crumbkit to its own subtree. `docs/behaviour/adoption/README.md`
and `docs/behaviour/checks/README.md` describe the current adoption steps
and checks.

Considered: a hand-written `clap::Args` implementation for each argument type, crumbkit's rendered pages for a tool that parses with clap alone, a switch on `Pages::rendered` that drops crumbkit's lines, a fixed width of 100 columns for clap's pages, an `Option<Guide>` argument on the checks that read only part of a guide, a crumbkit feature or a separate crate for the checks that need no guide, an `App` runner that owns dispatch, a `crumbkit::run` one-liner, a required derive macro, rendering from the compile-time tree, a trait with a provided `show_many`, an async runtime for concurrent lookups, a default worker count for `fetch_each`, builder-style tree checks, one grammar check with per-rule exemptions, no short flags, only `-h` and `-V` as short flags, a fixed verb list with exemptions, a plural check, a hidden getter for `args_override_self`, a hidden clap alias for each command's dashed form added when the tree is built, an opt-out with a reason for every name in another style, dashed forms on for every tool, the dashed-forms setting carried on the clap `Command`, a separate dashed-forms argument to `complete_with`, a parse probe with the option given twice, no aliases at all, hidden ones included, a trait that finds `Output` inside the tool's type, `finish` taking `Parsed`, a warning about a missing `set_command` in release builds, `parse` taking `&mut Command`, `parse` returning `Output` beside the tool's type, `process::exit` inside `parse`, building the whole tree in `set_command`, a lock inside `Output` that builds the tree on first read.
