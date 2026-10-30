# crumbkit

Conventions for Rust command-line tools that people and agents can drill into:
one command grammar, compact output, a help tree that reveals one level at a
time, failures with stable codes and a next step, and a local usage log that
shows where callers waste effort.

crumbkit gives you a few types and traits that work with your own clap types and
your own `main`, and you adopt only the parts you want.

## Crates

The eight crates are published on crates.io at version 0.1, and their API
reference is on [docs.rs](https://docs.rs/crumbkit). A tool starts with
crumbkit itself:

```sh
cargo add crumbkit
```

- [crumbkit](crates/crumbkit/README.md) holds the command grammar, the
  output, the help tree and the failures.
- [crumbkit-derive](crates/crumbkit-derive/README.md) derives `Failure`,
  behind crumbkit's `derive` feature.
- [crumbkit-test](crates/crumbkit-test/README.md) checks a tool against the
  conventions from a `#[test]`.
- [crumbkit-aliases](crates/crumbkit-aliases/README.md) expands a person's
  own command aliases.
- [crumbkit-usage](crates/crumbkit-usage/README.md) keeps the local usage
  log.
- [crumbkit-child](crates/crumbkit-child/README.md) hands the terminal to
  another program.
- [crumbkit-markdown](crates/crumbkit-markdown/README.md) writes a Markdown
  reference of every command.
- [cargo-crumbkit](crates/cargo-crumbkit/README.md) is the
  `cargo crumbkit` subcommand, which weighs a tool's help and writes its
  reference.

The crates follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
and share one version. Before 1.0, a minor release, such as 0.2, may change
the API, and the [changelog](CHANGELOG.md) names each breaking change. Each
release pull request from 0.1.1 on runs cargo-semver-checks against the last
published version. A patch release, such as 0.1.1, makes no breaking change.

## Licence

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
