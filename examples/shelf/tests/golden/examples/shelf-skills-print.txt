---
name: shelf
description: Browse shelf's small catalogue of public-domain books. Use it to list, show, summarize or remove books.
---

# shelf

shelf browses a built-in catalogue of six public-domain books.

- `shelf books list --all` prints every book with its id.
- `shelf books show ID` prints the books with those ids.
- `shelf books summarize` describes the whole catalogue in one sentence.
- `shelf books remove ID --dry-run` prints what a removal would remove.

Add `--json` to any command for a JSON document, and run
`shelf schemas show shelf/books-list/v1` for its schema.
