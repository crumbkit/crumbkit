# ADR-0013. Documentation

- rustdoc holds the API reference, tutorial and cookbook. An mdBook site in `docs/` holds the standards and these records.
- Every terminal sample is an included golden file.
- Each README is short and doctested.
- After 0.1, `cargo crumbkit docs` exports an adopter's help pages, golden examples, failure codes, schemas and weights as Markdown pages plus a JSON manifest with its own schema, for any static site builder.
- Its built-in static site is rendered from that export, and shelf's site is its showcase.

Considered: cargo-rdme, rustdoc theming, a Node site generator.
