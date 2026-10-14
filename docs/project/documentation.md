# Documentation

Each kind of fact has one home, and every other place links to it, because
two copies of a fact drift apart and a reader cannot tell which copy is
right. This page names each home.

## Homes of facts

| Fact | Home |
| --- | --- |
| How an item behaves | The rustdoc on that item or its module. The rustdoc of a derive macro holds the reference for its attributes and the code it writes for an example type. docs.rs publishes it for each crate with every feature on, and labels each item that a feature adds. |
| How crumbkit behaves for a tool, where rustdoc has no single item to hold it | A page under [`docs/behaviour/`](../behaviour/README.md), which links to the rustdoc items instead of restating them. |
| How the repository is built, checked, tested and released | A page under `docs/project/`. |
| The rules that code and docs follow | [`docs/standards/`](../standards/README.md), where each rule has an ID and names its gate. |
| Why a choice was made | Its [decision record](../decisions/README.md). Each section of a page ends with a `Why:` line that cites the records behind it. |
| What changed | `CHANGELOG.md`, which follows Keep a Changelog. |
| Exact output, such as help pages, footers and failure lines | Golden files and constants. |

A record names a number only when the number is the decision itself, such
as a budget. Docs include a golden file with `include_str!` or link to it,
and do not retype a block of output. A sentence may quote a single line of
output as an example.

Why: ADR-0013.

## Changelog entries and crate READMEs

Each changelog entry names one change and the item it touches, and links to
the item's rustdoc or record instead of describing the behaviour.

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

## The tutorial

The getting-started tutorial is [`docs/tutorial.md`](../tutorial.md), and
the crate README and `docs/README.md` link to it. Each step is a whole
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
rendered from it.

Why: ADR-0013, ADR-0044.
