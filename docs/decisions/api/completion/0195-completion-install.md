# ADR-0195. Completion install

- Date: 2026-11-10
- Status: accepted

## Context

The `complete` feature answered a shell's completion request (ADR-0048),
but nothing installed the registration. A person had to find the right
line and the right file for their shell, so most crumbkit tools offered no
completion in practice.

## Decision

- The `complete` feature adds `crumbkit::completion` with `install`,
  `check` and `print`, shaped as `schemas list` is: a clap arguments type
  and a function whose result the tool passes to `finish`. Each function
  takes the function that builds the tool's command, as `complete` does,
  so the script names the same command that answers a Tab press.
- The shell is `--shell`, a clap value list of clap_complete's five shells,
  or the file stem of `SHELL`. An unset or unknown `SHELL` fails with
  `crumbkit::shell::unset` or `crumbkit::shell::unknown`, whose hint names
  `--shell`.
- `install` writes fish's file under `XDG_CONFIG_HOME`, bash's under
  `XDG_DATA_HOME` and zsh's in a directory of `FPATH` inside the home
  directory, or in `~/.zfunc` with a note that gives the `fpath` line. It
  edits no startup file, and elvish and PowerShell fail with
  `crumbkit::shell::no_directory`, because neither loads completions from a
  directory.
- The script runs the tool by the name that the person types, in place of
  the path that `COMPLETE=<shell> <tool>` writes, so an installed file
  follows the binary on `PATH` and a golden file holds no machine's path.
- A comment line that names the tool marks install's file, after zsh's
  `#compdef` line. Install replaces only a marked file, follows a link only
  to a marked file, and otherwise skips with the reason unless `--force`,
  which replaces the file or the link itself and never removes a directory.
- The zsh file ends with a call of the function that it defines, because
  zsh runs an autoloaded file as the completion function on the first Tab
  press, and without the call that press offers nothing. `print` leaves the
  call out, because a person sources its script.
- crumbkit's new `files` module holds the home directory from `HOME` or
  `USERPROFILE`, the write through a new temporary file and a rename, and
  the display of a path. crumbkit-skills uses it in place of its own copies,
  because crumbkit-skills depends on crumbkit and the reverse would be a
  cycle.
- `check` lists only a missing, differing or unreadable file and exits with
  1 when there is one. A link to a file that install did not write is
  `linked` and no problem, as in `skills check`.

## Consequences

The plan named a zsh directory on `fpath` that the person can write; install
chooses only directories inside the home directory, because the standard
library cannot tell whether a directory is writable and a system directory
would surprise the person. `docs/behaviour/parsing/completion.md` describes
the current behaviour.

Considered: the binary's path in the script, a `&mut Run` in place of the command function, editing `~/.zshrc`, a failure for zsh without an `fpath` directory, a shared module in crumbkit-skills, copying the write and home code, failure codes under a `completion` area, an `access(2)` check through rustix for writable `fpath` directories, `print` with the zsh call.
