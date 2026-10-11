# ADR-0006. Property tests and fuzzing

- proptest checks the code segment rules, the predefined spellings and table rendering. The usage-log reader and the usage-record reduction arrive in a later release, and proptest will check the reduction and bolero will fuzz both once code for them exists.
- The fast lane runs the property tests briefly with one fixed seed (RUST-11). The nightly slow lane will fuzz for 10 minutes per target once a fuzz target exists.
- A fix adds a unit test holding the failing input.

Considered: cargo-fuzz, quickcheck.
