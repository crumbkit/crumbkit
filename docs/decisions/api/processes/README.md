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
