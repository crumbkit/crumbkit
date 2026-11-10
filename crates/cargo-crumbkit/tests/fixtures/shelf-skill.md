---
name: shelf
description: Browse a small catalogue of books with shelf.
---

# shelf

Use `shelf books list` to page through the catalogue, and
`shelf books show ID` to read one book. `shelf books remove ID --dry-run`
shows what a removal would do before `shelf books remove ID --yes` does it.

```sh
shelf books list --limit 5
shelf books show 1 2 --json
shelf --json books list --all
shelf schemas show <id>
shelf self usage --since 7d
shelf commands list books -h
```

`shelf books list [OPTIONS]` lists the books, and `shelf COMMAND -h` prints
a command's help.
