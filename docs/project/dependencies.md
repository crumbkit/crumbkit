# Dependencies and platforms

This page states what a dependency must meet, what each crumbkit feature
adds, the MSRV and the supported platforms. `Cargo.toml` names each
dependency, feature and dev-dependency with its reason.

## Admitting a dependency

A dependency needs active maintainers, judged case by case, a compatible
`rust-version` and licence, a measured weight, and no async runtime or C
build. `Cargo.toml` states its reason and sets `default-features = false`.
gungraun is the exception, because it exports its benchmark API only under
its default feature. A published crate enables only the features its own
code uses, so a binary chooses features such as clap's colour.

Why: ADR-0003.

## The default build

The default build of crumbkit depends on clap, serde, serde_json, schemars
and is-ai-agent. The failure, document, call and row types need only serde,
serde_json and schemars, and crumbkit holds them beside the parsing and
output that need clap (ADR-0097).

crumbkit enables two clap features:

- `help`, so every tool built with crumbkit has `-h` and `--help`, which
  crumbkit's refusals tell the caller to run.
- `error-context`, so a refusal reads the refused argument, value and
  subcommand from clap's error for its fields. clap's `suggestions` feature
  already turns it on. It added 33,456 bytes to shelf's stripped release
  binary on aarch64 macOS when it was measured: 2,494,792 bytes with it and
  2,461,336 without.

is-ai-agent has no dependencies of its own. crumbkit reads it only when a
question is about to be asked, so a tool does not ask an agent a question
it cannot answer.

Why: ADR-0003, ADR-0001, ADR-0097, ADR-0043.

## Features and separate crates

| Feature or crate | Adds |
| --- | --- |
| `derive` | crumbkit-derive, a procedural macro built on syn 3, quote and proc-macro2, which clap_derive already brings into the lock file with the same syn features. The feature is off by default, so a tool that depends on crumbkit alone and writes its `Failure` impls by hand builds no syn. crumbkit-child and crumbkit-aliases turn the feature on for their own failures, so a tool that uses either builds crumbkit-derive. |
| `miette` | miette with no features, because crumbkit draws the source line and its underline itself, and unicode-width, which miette already depends on, so carets line up after wide characters. The width tables added 16,976 bytes to shelf's stripped release binary on aarch64 macOS. |
| `tracing` | The tracing crate with no features. It has no build script, compiles in about 0.3 s and added about 18 KB to shelf's stripped release binary. |
| `color` | clap's `color` feature, which adds anstream and lets `parse_to` write help with clap's styles to a styled sink. The feature is off by default, so a binary chooses clap's colour. shelf already enables clap's `color`, and crumbkit's feature added 16 bytes to its stripped release binary on aarch64 macOS: 3,265,728 bytes with it and 3,265,712 without. |
| `complete` | clap_complete with its `unstable-dynamic` feature, which brings shlex and is_executable and turns on clap's `unstable-ext`. It added 102,192 bytes to shelf's stripped release binary on aarch64 macOS. |
| crumbkit-aliases | toml. The crate is separate, so only a tool with personal aliases builds toml. |
| crumbkit-child | signal-hook with no features, which crumbkit-usage also uses, so a tool can hand the terminal to a child while it ignores Ctrl-C. The crate is separate, so only a tool that hands the terminal over builds it. |
| crumbkit-usage | The usage log's dependencies: siphasher, etcetera, humantime and signal-hook, and getrandom on Windows, which proptest already brings into the lock file, because Windows has no `/dev/urandom`. The crate is separate, so only a tool that keeps the log builds them. |
| cargo-crumbkit | etcetera, which crumbkit-usage already uses, so `weigh --usage` finds a tool's log with the same calls that wrote it. |

Why: ADR-0003, ADR-0001, ADR-0016, ADR-0027, ADR-0029, ADR-0030, ADR-0033,
ADR-0042, ADR-0043, ADR-0048, ADR-0102.

## MSRV and toolchain

The workspace's `rust-version` is the MSRV of RUST-9. crumbkit raises it
only when a change needs it, and never above the newest stable release that
is at least six months old. `rust-toolchain.toml` pins the stable toolchain
that builds and lints the workspace, so a new clippy release never fails an
unrelated change.

Why: ADR-0003, ADR-0009.

## Platforms

- Linux and macOS are tier 1. Every pull request runs the full lane on
  Linux, and each push to `main` runs the tests on macOS.
- Windows on `x86_64-pc-windows-msvc` is tier 2. The scheduled lane builds,
  lints and tests the workspace there each night that `main` changed
  (CI-4). A failure opens or comments on the scheduled issue, and no release
  waits for it.
- Other targets are untested, and crumbkit-usage refuses to build
  on a target that is neither Unix nor Windows.

Windows at tier 2 has these limits:

- `help::quote` quotes for a POSIX shell, so an example line pasted into
  PowerShell or `cmd` may need other quoting.
- The usage log's directory and files inherit the access control list of
  `%LOCALAPPDATA%` in place of Unix modes.
- Closing the console, logging off and shutting down end a tool with no
  usage line and end a tool beside its child. Ctrl-Break ends a tool with no
  usage line.

Why: ADR-0003, ADR-0011, ADR-0024, ADR-0029, ADR-0042.
