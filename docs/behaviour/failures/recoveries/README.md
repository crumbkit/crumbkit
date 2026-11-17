# Recoveries and alternatives

Every failure states a [`Recovery`](../../../../crates/crumbkit/src/failure.rs).
A `Recovery::Run` call that the tool's command tree cannot render prints as
`None` with crumbkit's reason, so crumbkit never advertises a command the tool
hid. A `Recovery::Run` call can carry concrete values with `Call::value`, so a
failure can suggest the exact command that repairs it. Rendering refuses a
value that the tool would refuse before it runs: a value for a flag, a value
outside the argument's possible values, and a positional value that clap would
read as a flag.

`Recovery::Terminal` names a command that a person must run in a terminal,
such as one that opens an editor or asks questions. Text prints it as
``run `shelf login` in a terminal``, and JSON holds
`{"kind":"terminal","call":"shelf login"}`, so a program that reads the
failure can hand the step to a person and not run it itself. A call that the
tool's command tree cannot render behaves as a `Recovery::Run` call that
cannot render. The call must be concrete, because a terminal runs exactly the
call that prints, so a call with a placeholder from `Call::arg` or `Call::all`
behaves as a call that cannot render. `Call::value` gives an argument a
concrete value, and `Call::flag` adds a flag that takes no value as set, such
as `--password-auth`, so a terminal call can hold both. A flag added with
`Call::arg` is a placeholder.

`Recovery::None` with a reason is the exception for a failure that no step
can repair. A tool declares each such failure with one of its declared codes
through an opt-out from `check_recoveries`'s `Rule::NextStep`
([recovery chains](../../checks/recovery-chains.md)).

A failure may offer alternative repairs through `Failure::alternatives`, such
as setting a token variable beside logging in. An alternative that offers no
step is left out. When the recovery offers no step, the first alternative
that offers one takes its place, so the output never says that there is no
next step and then names one. In text, each alternative prints on its own
`help:` line after the recovery's.

`Recovery::program` names a command of another program, such as
``Recovery::program(GIT, ["status"])`` with `const GIT: Program =
Program::new("git");`. Text prints it as ``run `git status` ``, with each
argument quoted as `Step::run` quotes it, and JSON holds
`{"kind":"program","program":"git","args":["status"],"run":"git status"}`, so
a program that reads the failure can tell a command of another tool from
advice. `run` is the quoted line, so a reader does not quote it again. An
argument with a control or format character makes the recovery print as `None`
with the reason that the suggested command is not available, and a debug
build warns. crumbkit does not check that the program exists or accepts the
arguments, and `check_recoveries` reports the step without running it
([recovery chains](../../checks/recovery-chains.md)).

Why: ADR-0001, ADR-0252.
