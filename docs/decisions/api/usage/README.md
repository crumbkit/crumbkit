# ADR-0028. Usage

- Date: 2026-10-10
- Status: accepted

## Decision

These records decided what a tool built with crumbkit records about its
own runs with the crumbkit-usage crate: the usage log, which appends one line per run to a file on the
user's machine, the record that each line holds, what a tool adds to it,
the W3C trace context that joins a run to the trace of its caller and its
children, and the report that `self usage` reads from the log.

## Children

- [ADR-0029. Usage log](0029-usage-log.md)
- [ADR-0030. Trace context](0030-trace-context.md)
- [ADR-0034. Usage report](0034-usage-report.md)
- [ADR-0095. crumbkit-child hands over text, the usage log records its own signals, and `Menu` lives in crumbkit](0095-text-hand-over-own-signals-and-menu.md)
- [ADR-0144. A tool adds value-free facts of its own to the usage record](0144-a-tool-adds-value-free-facts-to-the-usage-record.md)
- [ADR-0145. `self usage` finds slow commands that mostly wait](0145-self-usage-finds-slow-commands-that-mostly-wait.md)
