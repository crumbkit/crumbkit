# Documentation

Each kind of fact has one home, and every other place links to it, because
two copies of a fact drift apart and a reader cannot tell which copy is
right. This page names each home.

## Homes of facts

| Fact | Home |
| --- | --- |
| How an item behaves | The rustdoc on that item or its module. A cookbook will join it in rustdoc once it is written. The rustdoc of a derive macro holds the reference for its attributes and the code it writes for an example type. docs.rs publishes it for each crate with every feature on, and labels each item that a feature adds. |
| How crumbkit behaves for a tool, where rustdoc has no single item to hold it | A page under [`docs/behaviour/`](../behaviour/README.md), which links to the rustdoc items instead of restating them. |
| How the repository is built, checked, tested and released | A page under `docs/project/`. |
| The rules that code and docs follow | [`docs/standards/`](../standards/README.md), where each rule has an ID and names its gate. |
| Why a choice was made | Its [decision record](../decisions/README.md), which is frozen once it is on `main` (DOCS-6). Each section of a page ends with a `Why:` line that cites the records behind it. |
| What changed | `CHANGELOG.md`, which follows Keep a Changelog. |
| Exact output, such as help pages, footers and failure lines | Golden files and constants. |

A record names a number only when the number is the decision itself, such
as a budget. Docs include a golden file with `include_str!` or link to it,
and do not retype a block of output. A sentence may quote a single line of
output as an example.

Why: ADR-0013.

## Changelog entries and crate READMEs

Each changelog entry names one change and the item it touches, and links to
the item's rustdoc or page instead of describing the behaviour.

Each crate's README holds what the crate is, how to add it, one doctested
example, and links to the item docs and records. The README is also the
crate's rustdoc front page through `include_str!`. Its links are Markdown
reference links to files in the repository, which work on GitHub. The
crate's `lib.rs` defines the same labels as rustdoc paths before it
includes the README, and rustdoc uses those, because Markdown uses the
first definition of a label.

DOCS-5 sets a word limit for each README and each changelog entry. When a
README or an entry goes over its limit, the detail moves into the rustdoc
or the docs, and the README or entry links to it.

Why: ADR-0013.

## Keeping docs current

`docs/` and the rustdoc state what crumbkit does now. A branch that changes
behaviour updates every page and rustdoc item the change affects. When it
cannot, it records an obligation to update them. DOCS-7 makes this
mechanical: a branch that adds or changes an Unreleased changelog entry
must have a commit whose message ends with one of these trailers.

```text
Docs-Owed: json-layout; from: one JSON object per line; to: indented JSON; docs: docs/behaviour/output/json-layout.md
Docs-Swept: json-layout
```

`Docs-Owed` gives the obligation a kebab-case id, the old and new
behaviour, and the paths of the docs that still describe the old
behaviour. `Docs-Swept` names one or more obligations whose docs the
commit updated. A branch that updates its docs in the same change uses
`Docs-Swept` with an id of its own.

A decision record never changes to match new behaviour. A changed decision
gets a new record whose `Supersedes` line names the old one, and the old
record's Status line says it was superseded (ADR-0017). A new record that
amends a record dated less than 7 days before it also has a `- Missed:`
line that says what the earlier record missed (DOCS-8).

Why: ADR-0013, ADR-0017, ADR-0074.

## The tutorial

The getting-started tutorial is [`docs/tutorial.md`](../tutorial.md), and
the crate README and `docs/README.md` link to it. It is outside the crate
README, because a README holds one example within its word limit. Each step is a whole
binary in the `tutorial` example package, and the page shows parts of each
file. A test in that package fails when a block of Rust on the page differs
from the file that the link before it names, so the page shows only code
that compiles. The page is Markdown, because rustdoc would add a public
module to crumbkit to hold it and GitHub would show the hidden lines of its
doctests.

Why: ADR-0013.

## Generated references and the site

`cargo crumbkit docs` writes a tool's command reference, one Markdown file
built from its `commands list` document, as
[Command reference](../behaviour/help/reference.md) describes. crumbkit has
no documentation site for 0.1. A planned export will write an adopter's
golden examples, failure codes, schemas and weights as Markdown pages and a
JSON manifest with its own schema, and a built-in static site will be
rendered from it, and shelf's site will be its showcase.

Why: ADR-0013, ADR-0044.
