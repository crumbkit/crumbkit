# ADR-0170. Processes

- Date: 2026-11-05
- Status: accepted

## Decision

These records decided how a tool runs another program whose output it
reads: how the program starts, how its output is read and limited, how it
is stopped and reaped, and what differs between Unix and Windows. A child
that takes over the terminal is a hand-over, which ADR-0042 and ADR-0058
decided.

## Children

- [ADR-0171. The process crate](0171-the-process-crate.md)
- [ADR-0172. What the process crate adds to its first decision](0172-process-crate-additions.md)
- [ADR-0173. How the process crate waits on a stopped child and signals a moved one](0173-stopped-child-wait-and-moved-child-signal.md)
- [ADR-0174. A run that stopped its child keeps the child's status](0174-stopped-run-keeps-child-status.md)
- [ADR-0176. A full line queue waits half a drain grace after the reap](0176-full-queue-waits-after-the-reap.md)
- [ADR-0178. The process crate's types have names of their own](0178-process-type-names.md)
- [ADR-0179. The forwarder ends the tool and runs the usage line](0179-the-forwarder-ends-the-tool-and-runs-the-usage-line.md)
- [ADR-0187. A run's hook wraps the start of its child](0187-a-hook-wraps-the-start-of-the-child.md)
- [ADR-0188. A run takes a file as input, copies output and inherits output](0188-a-run-takes-a-file-copies-output-and-inherits-output.md)
- [ADR-0189. A reap waits until the child's group is empty](0189-a-reap-empties-the-group.md)
