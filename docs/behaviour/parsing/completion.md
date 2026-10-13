# Shell completion

This page covers how a tool answers a shell's completion request through
crumbkit's `complete` cargo feature. The rustdoc of
[`complete`](../../../crates/crumbkit/src/complete.rs) gives the registration
line for each shell and describes `complete_with`, `complete_with_words` and
`complete_to`.

## Answering a request

The `complete` feature answers a shell's completion request through
clap_complete's dynamic completion. A shell asks by running the tool with the
completion variable set to the shell's name, once to register and once per
Tab press. The variable is `COMPLETE` unless the tool declares its own name
with `crumbkit::complete_with`, such as one its users already register, and
the registration script then sets that name
([declarations](../adoption/declarations.md)). The feature is off by default,
so the default build has no completion code and no clap_complete.

A tool's `main` calls `crumbkit::complete` or `complete_with` first, before
`usage::start`, and passes the function that builds the command it parses
against. crumbkit calls the function only for a completion request, so an
ordinary run builds its command once and no run clones the tree. A tool that
hides commands at run time does so inside that function, so completion
offers what parsing accepts. A tool whose completers depend on an earlier
word on the line, such as path completers under the directory that a global
`-C <DIR>` chooses, calls `complete_with_words`.

`complete` never exits the process. The answer goes through crumbkit's
`Sink` whole or not at all. A completion never prints a failure into the
shell, because the shell shows standard error over the person's prompt. An
unknown shell, a command line that clap_complete cannot complete and a failed
write give exit code 1 and write nothing.

Why: ADR-0048, ADR-0003, ADR-0056.

## The completion variable

The completion variable stays set for the rest of the run, because removing
an environment variable needs unsafe code, which RUST-2 forbids. A custom
completer that runs another clap_complete tool as a child removes the
variable from the child's environment itself. `complete_with` runs before the
guide is built, so it records the declared name in a process-wide
`OnceLock`, and the usage log reads the name from there and falls back to
`COMPLETE`.

The usage log installs no recorder for a run whose completion variable asks
for completion, by the same rule that `complete` reads, so a Tab press
writes no line even when a tool starts the log first or calls clap_complete
directly ([usage log](../usage-log/README.md)).

Why: ADR-0048, ADR-0029.

## Hidden names

The candidates follow clap_complete's rule for hidden items. A hidden
subcommand, flag or alias is left out whenever a visible one matches the
typed word, and a word that matches only hidden names completes to them. So
`shelf books ls` completes when `ls` is a hidden alias of `list`, and
`shelf al` completes to the hidden `aliases`. This is the one place where
crumbkit prints a hidden name that the caller typed only in part. Completion
candidates show only the tool's own command names and never a declared
dashed form ([parsing](README.md#dashed-forms-of-command-names)).

Why: ADR-0048, ADR-0015.

## Stability and tests

clap_complete's dynamic completion sits behind its `unstable-dynamic`
feature, which clap leaves outside its semver promise. crumbkit requires
`~4.6.11`, which allows 4.6 patch releases only, and a crumbkit minor release
may change what the `complete` feature offers and how a shell registers it
([public API](../adoption/README.md)).

crumbkit has no help topic for completion, because completion serves a
person at a shell and a topic would add a name to every root help page that
agents read. A tool tells its users the registration line. crumbkit's tests
complete shelf's tree and check that hidden commands and aliases are left
out, and crumbkit-test has no completion check for tools.

Why: ADR-0048, ADR-0002.
