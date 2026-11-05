# crumbkit-process

crumbkit-process runs another program whose output a tool reads. It starts
the child so that the child's descendants stop with it, reads standard
output and standard error within byte limits and a deadline, stops the
child gracefully before it kills it, and reaps it without signalling a
process id that another process may have taken.

The crate depends on neither crumbkit nor crumbkit-child, because it prints
nothing and parses no command line.

## Adding it

```toml
[dependencies]
crumbkit-process = "0.1"
```

## Example

A tool builds the command, states what it keeps of each stream and how
long the child may run, and reads how the child ended:

```rust
use std::process::Command;
use std::time::{Duration, Instant};

use crumbkit_process::{Capture, Escalation, Exit, OverLimit, Run, RunError, Signal};

fn version(mut tool: Command) -> Result<Option<Vec<u8>>, RunError> {
    tool.arg("--version");
    let settings = Run::new()
        .stdout(Capture::Whole { limit: 64 * 1024, over: OverLimit::Stop })
        .stderr(Capture::Tail { limit: 4096 })
        .deadline(Instant::now() + Duration::from_secs(10))
        .escalation(Escalation::default().step(Signal::Terminate, Duration::from_secs(2)));
    let finished = crumbkit_process::run(tool, &settings)?;
    Ok(match finished.exit() {
        Exit::Code(0) => Some(finished.into_output().0.into_bytes()),
        _ => None,
    })
}
```

On Unix the child leads a new process group. On Windows it runs in a job
object, and a stop ends the job at once with no graceful step. A tool that
is killed outright or crashes leaves the job's processes running on
Windows, and leaves the group running on Unix.

## Where to read more

- [`run`] and [`Run`] describe the one-shot run, its limits, deadline,
  cancel and scope, and [`Exit`] how it ended.
- [`Group`] starts, signals and reaps a child and its descendants, and
  [`Escalation`] lists the steps that stop it.
- The [processes docs] describe the current behaviour, and [ADR-0171]
  records why.

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.

[`run`]: src/run.rs
[`Run`]: src/run.rs
[`Group`]: src/group.rs
[`Escalation`]: src/escalation.rs
[`Exit`]: src/exit.rs
[ADR-0171]: ../../docs/decisions/api/processes/0171-the-process-crate.md
[processes docs]: ../../docs/behaviour/processes.md
