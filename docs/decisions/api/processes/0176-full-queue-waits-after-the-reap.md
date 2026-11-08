# ADR-0176. A full line queue waits half a drain grace after the reap

- Date: 2026-11-08
- Status: accepted
- Supersedes: ADR-0172, which it amends
- Missed: ADR-0172 did not decide what a full line queue does after the child has exited by itself and the caller no longer reads, and it tied the run's time bound to a caller that reads its lines.

## Context

ADR-0172 decided that a full line queue makes the run's reader wait, and
that it drops each line only once the run begins to stop the child. A child
that exits by itself while its queue is full and its caller has stopped
reading therefore held the run for as long as the caller kept the queue,
and the run's time bound held only for a caller that reads its lines.

## Decision

- After the run has reaped the child, a full queue waits for the caller
  until half of `DRAIN_GRACE` has passed, and then drops each line. A
  caller that keeps reading receives every line that it can read within
  that time.
- The reads of the pipes continue until `DRAIN_GRACE` after the reap. The
  reader keeps reading while the queue drops lines, so the pipe is read to
  its end in the second half of the grace and `Lines::lost` is the
  stream's last line when it is.
- A run that begins to stop the child still drops each line from a full
  queue at once.
- The run's time bound no longer depends on a caller that reads its lines.

## Consequences

A caller that stopped reading holds the run for at most `DRAIN_GRACE` after
the reap. `Lines::lost` is not the stream's last line when a process
outside the child's group holds the pipe and goes on writing, when the
reader is too slow to reach the end of the pipe in the second half of the
grace, or when the stream went over its limit. `Lines::lost` is `None` when
a line was queued after the last drop, so `None` does not show that no line
was dropped.

Considered: one deadline for the queue and the reads, which stops the
reader as the queue starts to drop lines and leaves `Lines::lost` as a
fragment from the middle of the stream; dropping lines as soon as the
child exits, which loses output for a caller that is still reading.
