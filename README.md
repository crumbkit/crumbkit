# crumbkit

Conventions for Rust command-line tools that people and agents can drill into:
one command grammar, compact output, a help tree that reveals one level at a
time, failures with stable codes and a next step, and a local usage log that
shows where callers waste effort.

crumbkit gives you a few types and traits that work with your own clap types and
your own `main`, and you adopt only the parts you want.

crumbkit is in design. Nothing is published yet.

## Licence

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
