# ADR-0218. A tool declares its completion variable without answering a request

- Date: 2026-11-13
- Status: accepted
- Supersedes: ADR-0056, which it amends

## Context

ADR-0056 let a tool declare its completion variable through `complete_with`, and `complete_with_words` declares it the same way. Each call sets the process's declared name and also answers the request when the variable holds a shell's name. A tool that answers completion requests with its own handler, such as one that completes paths that clap_complete cannot, calls `complete_with_words` only when its variable asks for nothing. In a run whose variable is set, such as `SHELF_COMPLETE=fish shelf completion install`, that tool declares nothing, so `completion install`, `completion check` and `completion print` write a registration that sets `COMPLETE`, and the installed file completes nothing.

`complete_with(variable, command)` takes the variable before the command, and `complete_to(sink, command, variable, shell, args, current_dir)` takes the command before the variable. Changing that order after 0.5.0 would be a second breaking change.

## Decision

- `crumbkit::declare_completion_variable(variable: &'static str)`, behind the `complete` feature, sets the process's completion variable and does nothing else. It reads no environment, builds no command, answers no request and returns nothing.
- The first declaration in the process wins, whether it comes from `declare_completion_variable`, `complete_with` or `complete_with_words`, and a later declaration with another name changes nothing, as before. `events::observe`, `completion install`, `completion check` and `completion print` read the declared name as before.
- The function sits at the crate root beside `complete`, `complete_with`, `complete_with_words` and `complete_to`, because the `complete` module is private and those functions are exported at the root.
- `complete_to` takes the variable before the command: `complete_to(sink, variable, command, shell, args, current_dir)`, so its first two arguments after the sink follow `complete_with(variable, command)`.

## Consequences

A tool with its own completion handler calls `declare_completion_variable` first in `main` and gets a registration that sets its variable in every run, including a run whose variable is set. A caller of `complete_to` swaps its second and third arguments. ADR-0002 exempts the `complete` feature from stability, and the CHANGELOG still marks the order change breaking. crumbkit does not check that the declared name is one every shell accepts, as `complete_with` does not.

Considered: a public `crumbkit::complete::declare`, which needs a new public module; a return value that says whether the declaration won, which no adopter reads, since `completion print` shows the name that won.
