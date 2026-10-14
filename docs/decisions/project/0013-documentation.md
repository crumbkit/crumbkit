# ADR-0013. Documentation

- Date: 2026-10-14
- Status: accepted

## Context

From 2026-10-07, rustdoc held the API reference, docs.rs published it, and
the standards and records were Markdown files in `docs/`. Each kind of fact
was meant to have one home, because two copies of a fact drift apart and a
reader cannot tell which copy is right. In practice the decision records
held most of crumbkit's behaviour mixed with the reasons for it, and the
rustdoc repeated much of it. Contributors edited a record before changing a
choice, so the records were both the history and the current specification.

## Decision

- The pages in `docs/` and the rustdoc became the authoritative statement
  of what crumbkit is and does. The contract of one public item lives in
  its rustdoc. Behaviour that spans several items, and every convention
  crumbkit enforces on a tool, lives on a page under `docs/behaviour/`. How
  the repository is built, checked and released lives under
  `docs/project/`. Each fact has one home, and every other place links to
  it.
- Decision records hold the reasons and the history (ADR-0017). A page
  cites a record only to say why, in a `Why:` line.
- Exact output, such as help pages, footers and failure lines, stays in
  golden files and constants, and docs include or link to them.
- Each crate's README holds what the crate is, how to add it, one doctested
  example, and links, and it is also the crate's rustdoc front page.
  `CHANGELOG.md` follows Keep a Changelog, and each entry names one change
  and links to the rustdoc or page that describes it. DOCS-5 limits both.
- The getting-started tutorial stayed a Markdown page whose blocks a test
  compares with the files of the `tutorial` example package.
- A branch that adds or changes an Unreleased changelog entry either
  updates every doc the change affects or records an obligation to update
  them, as a trailer on one of its commits. A `Docs-Swept` trailer names
  the swept obligations, and a `Docs-Owed` trailer states the change, from
  what to what, and the docs it affects (DOCS-7).

## Consequences

- Behaviour that the records described got page text under
  `docs/behaviour/` and `docs/project/`, and rustdoc that cited a record
  for behaviour links to the page instead.
- A reader of `docs/` reads Markdown on GitHub, because 0.1 has no
  documentation site. `docs/project/documentation.md` describes the homes
  of facts, the READMEs, the tutorial and the planned site.
- A behaviour change that leaves docs untrue carries a `Docs-Owed` trailer
  in its history instead of going unnoticed.

Considered: cargo-rdme, rustdoc theming, a Node site generator, describing
each behaviour in full in the README, the changelog and the record, links
from the READMEs to docs.rs, which fail until a crate is published, an
mdBook site for 0.1, the tutorial as a rustdoc module with doctests, a
tutorial whose blocks are retyped without a test, behaviour in rustdoc
only, behaviour pages included into rustdoc for docs.rs, an obligation list
kept in the repository, records as the specification edited in place.
