# ADR-0006. Property tests and fuzzing

- proptest checks the code segment rules, the predefined spellings and table rendering. The usage-log reader and the usage-record reduction arrive in a later release, and proptest will check the reduction and bolero will fuzz both once code for them exists.
- The fast lane runs the property tests briefly with the fixed seed of RUST-11, so a CI run generates the same cases on every machine. proptest writes no regression files, so the unit test that RUST-11 asks a fix to add keeps the failing input. The nightly slow lane will fuzz for 10 minutes per target once a fuzz target exists.

Considered: cargo-fuzz, quickcheck.
