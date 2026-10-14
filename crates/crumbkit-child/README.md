# crumbkit-child

crumbkit-child lets a command of a crumbkit tool hand the terminal to
another program, such as an editor, a pager or a wrapped tool, as the
command's whole response. The child reads and writes the terminal directly,
Ctrl-C reaches only the child, and the tool returns the child's exit code.
crumbkit is in design and is not published yet.

## Adding it

A tool adds crumbkit-child beside crumbkit, from this repository until the
first release:

```toml
[dependencies]
crumbkit-child = { git = "https://github.com/crumbkit/crumbkit" }
```

The `main` branch can change without notice before the first release.

## Example

A command that opens an editor calls [`hand_over`] last, in place of
`crumbkit::finish`, and returns the exit code that it gives:

```rust,no_run
use std::process::{Command, ExitCode};

use crumbkit::{Output, Spelling};

fn edit(output: &mut Output, path: &str) -> ExitCode {
    let mut editor = Command::new("vi");
    editor.arg(path);
    crumbkit_child::hand_over(output, Spelling::COLONS, editor)
}
```

A child that prints a typed JSON document when asked goes through
[`hand_over_json`] with a [`Json`] constant, so `--json` prints the child's
document inside crumbkit's envelope.

## Where to read more

- [`hand_over`], [`hand_over_json`] and [`Json`] describe the hand-over, its
  signals and its failures.
- [`codes`] lists the failure codes, which a tool's tests can name.
- The [child processes docs] describe what a hand-over does for a tool,
  and the [run observer docs] what it reports. [ADR-0042] and [ADR-0058]
  record why.

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.

[`hand_over`]: src/hand_over.rs
[`hand_over_json`]: src/hand_over.rs
[`Json`]: src/hand_over.rs
[`codes`]: src/codes.rs
[ADR-0042]: ../../docs/decisions/api/output/0042-child-processes.md
[ADR-0058]: ../../docs/decisions/api/output/0058-child-crate.md
[child processes docs]: ../../docs/behaviour/output/child-processes.md
[run observer docs]: ../../docs/behaviour/run-observer/README.md
