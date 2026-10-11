# ADR-0006. Property tests and fuzzing

- proptest checks the code segment rules, the predefined spellings, table rendering and the reduction of usage records, which must report the same figures and findings for any interleaving of sessions in the log and for a log read in two parts. The usage-log reader's bolero fuzz target arrives with the PR that sets up the nightly fuzz lane.
- The fast lane runs the property tests briefly with the fixed seed of RUST-11, so a CI run generates the same cases on every machine. proptest writes no regression files, so the unit test that RUST-11 asks a fix to add keeps the failing input. The nightly slow lane, which that PR sets up, fuzzes each target for 10 minutes.

Considered: cargo-fuzz, quickcheck.
