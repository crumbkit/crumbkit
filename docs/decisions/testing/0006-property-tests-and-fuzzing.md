# ADR-0006. Property tests and fuzzing

- proptest checks the usage-record reduction, the failure-code grammar and table rendering. bolero fuzzes the usage-log reader and the usage-record reduction.
- The fast lane runs them briefly with fixed seeds, and the nightly slow lane fuzzes for 10 minutes per target.
- A fix adds a unit test holding the failing input.

Considered: cargo-fuzz, quickcheck.
