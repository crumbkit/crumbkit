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

`Capture::Lines` delivers a stream line by line through a bounded queue
from [`line_queue`] while the child runs, which the caller reads from
another thread.

On Unix the child leads a new process group. On Windows it runs in a job
object, and a stop ends the job at once with no graceful step. A tool that
is killed outright or crashes leaves the job's processes running on
Windows, and leaves the group running on Unix.

The off-by-default `forward` feature, on Unix only, stops every live child
group when the tool receives SIGINT, SIGTERM or SIGHUP, runs the tool's
clean-ups, and then ends the tool as the signal would have. A tool that
also keeps crumbkit-usage's log starts it with
`start_under(forward::clean_up)` before it registers any other clean-up, so
the forwarder alone ends the tool and the log's line is its first clean-up.

## Where to read more

- [`run`] and [`Run`] describe the one-shot run, its limits, deadline,
  cancel and scope, and [`Exit`] how it ended. `Finished::status` gives
  the child's own status, which a stopped run has too.
- [`Escalation`] lists the steps that stop the child, and [`forward`]
  forwards the tool's signals.
- The [processes docs] describe the current behaviour, and [ADR-0171],
  [ADR-0172] and [ADR-0174] record why.

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.

[`run`]: src/run.rs
[`Run`]: src/run.rs
[`line_queue`]: src/lines.rs
[`forward`]: src/forward.rs
[`Escalation`]: src/escalation.rs
[`Exit`]: src/exit.rs
[ADR-0171]: ../../docs/decisions/api/processes/0171-the-process-crate.md
[ADR-0172]: ../../docs/decisions/api/processes/0172-process-crate-additions.md
[ADR-0174]: ../../docs/decisions/api/processes/0174-stopped-run-keeps-child-status.md
[processes docs]: ../../docs/behaviour/processes.md
