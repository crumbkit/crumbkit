# ADR-0013. Documentation

- rustdoc holds the API reference. A tutorial and a cookbook will join it in rustdoc once they are written. An mdBook site built from `docs/` will hold the standards and these records once code for it exists. Until then they are Markdown files in `docs/`.
- Every terminal sample will be an included golden file. The READMEs and rustdoc still write their terminal samples inline.
- Each README is short and doctested.
- After 0.1, `cargo crumbkit docs` exports an adopter's help pages, golden examples, failure codes, schemas and weights as Markdown pages plus a JSON manifest with its own schema, for any static site builder.
- Its built-in static site will be rendered from that export, and shelf's site will be its showcase.

Considered: cargo-rdme, rustdoc theming, a Node site generator.
