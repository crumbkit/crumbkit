# ADR-0003. Dependencies and MSRV

- A dependency needs active maintainers, judged case by case, a compatible `rust-version` and licence, a measured weight, and no async runtime or C build. `Cargo.toml` states its reason and sets `default-features = false`, and a published crate enables only the features its own code uses, so that binaries choose features such as clap's colour.
- The default build depends only on clap, serde, serde_json and schemars. crumbkit enables clap's `derive`, `string` and `help` features, because its own code uses each. With `help`, every tool built with crumbkit has `-h` and `--help`, which crumbkit's refusals tell the caller to run, and goal headings can set a help template (ADR-0021). The usage log and its dependencies (hmac, sha2, getrandom, etcetera, humantime and signal-hook) sit behind the `usage-log` cargo feature, and miette sits behind the `miette` feature. That feature enables miette with no miette features, because crumbkit draws the source line and its underline itself, so miette brings only cfg-if and unicode-width.
- The MSRV is 1.89. crumbkit raises it only when a change needs it, never above the newest stable release that is at least six months old.
- crumbkit supports Linux and macOS.

Considered: tabled, comfy-table, ctrlc, dirs, blake3, jiff, the usage log in the default build, miette's `derive` and `fancy` features.
