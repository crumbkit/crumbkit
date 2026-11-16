# ADR-0224. Usage log

- Date: 2026-11-14
- Status: accepted

## Decision

These records decided how the usage log writes its file on the user's
machine: where the log and its key live, how a run locks, appends and
trims the log, and how a run writes the key.

## Children

- [ADR-0029. Usage log](0029-usage-log.md)
- [ADR-0223. The usage key is written under the usage lock](0223-the-usage-key-is-written-under-the-usage-lock.md)
- [ADR-0232. The usage record carries lock wait and context](0232-the-usage-record-carries-lock-wait-and-context.md)
