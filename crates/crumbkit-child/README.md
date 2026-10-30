# crumbkit-child

crumbkit-child lets a command of a crumbkit tool hand the terminal to
another program, such as an editor, a pager or a wrapped tool, as the
command's whole response. The child reads and writes the terminal directly,
Ctrl-C reaches only the child, and the tool returns the child's exit code.

## Adding it

A tool adds crumbkit-child beside crumbkit:

```toml
[dependencies]
crumbkit-child = "0.1"
```

## Example

A command that opens an editor calls [`hand_over`] last, in place of
`crumbkit::finish`, and returns the exit code that it gives:

```rust,no_run
use std::process::{Command, ExitCode};

use crumbkit::Output;

fn edit(output: Output, path: &str) -> ExitCode {
    let mut editor = Command::new("vi");
    editor.arg(path);
    crumbkit_child::hand_over(output, editor)
}
```

The child's output is text that the terminal shows, so `hand_over` refuses
`--json` and `--fields` before the child starts. A tool that wants a
child's JSON runs the child itself and reads its output.

## Where to read more

- [`hand_over`] describes the hand-over, its signals and its failures.
- [`codes`] lists the failure codes, which a tool's tests can name.
- The [child processes docs] describe what a hand-over does for a tool,
  and the [run observer docs] what it reports. [ADR-0042], [ADR-0058] and
  [ADR-0095] record why.

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.

[`hand_over`]: src/hand_over.rs
[`codes`]: src/codes.rs
[ADR-0042]: ../../docs/decisions/api/output/0042-child-processes.md
[ADR-0058]: ../../docs/decisions/api/output/0058-child-crate.md
[ADR-0095]: ../../docs/decisions/api/usage/0095-text-hand-over-own-signals-and-menu.md
[child processes docs]: ../../docs/behaviour/output/child-processes.md
[run observer docs]: ../../docs/behaviour/run-observer/README.md
