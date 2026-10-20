# ADR-0031. Output

- Date: 2026-10-11
- Status: accepted

## Decision

These records decided how a tool prints a command's result: the compact
text table, the JSON document, the columns that a row type names, the
failures, batches and listings that print beside the rows, the streams
that print rows as they arrive, the output budget that limits the bytes
of one run, the child processes that a command hands the terminal to, the
terminal detection that no result depends on, and the layout of a JSON
document.

## Children

- [ADR-0041. Streams](0041-streams.md)
- [ADR-0042. Child processes](0042-child-processes.md)
- [ADR-0092. Output budget](budget/README.md)
- [ADR-0049. Terminal detection](0049-terminal-detection.md)
- [ADR-0058. The child crate](0058-child-crate.md)
- [ADR-0077. Notes on prose and batch results](0077-notes-on-prose-and-batches.md)
- [ADR-0093. The view flags are parsed once into a View](0093-the-view-flags-are-parsed-once-into-a-view.md)
- [ADR-0108. An age cell](0108-an-age-cell.md)
- [ADR-0050. Listings](listings/README.md)
- [ADR-0087. Uncounted listings omit totals](uncounted/README.md)
- [ADR-0098. JSON documents](json/README.md)
- [ADR-0091. One row in the singular](0091-one-row-in-the-singular.md)
- [ADR-0051. Result shapes](results/README.md)
