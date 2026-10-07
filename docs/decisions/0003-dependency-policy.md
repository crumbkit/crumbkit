# 0003. Dependencies and MSRV

- A dependency needs active maintainers, judged case by case, a compatible `rust-version` and licence, a measured weight, and no async runtime or C build. `Cargo.toml` states its reason and sets `default-features = false`.
- The MSRV is 1.89. crumbkit raises it only when a change needs it, never above the newest stable release that is at least six months old.
- crumbkit supports Linux and macOS.

Considered: tabled, comfy-table, ctrlc, dirs, blake3, jiff.
