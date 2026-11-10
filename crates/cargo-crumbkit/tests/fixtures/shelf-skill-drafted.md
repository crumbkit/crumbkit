---
name: shelf
# crumbkit:generated description
description: "Browse a small catalogue of public-domain books"
# /crumbkit:generated description
---

# shelf

<!-- Write here when an agent should use shelf and which command fits which job. cargo crumbkit skill draft rewrites only the sections between crumbkit:generated markers. -->

<!-- crumbkit:generated jobs -->
## Jobs

| Job | Command | What it does |
| --- | --- | --- |
| Browse the catalogue | `shelf books` | Work with the books in the catalogue |
| Explore the commands | `shelf commands` | Explore shelf's commands |
| Explore the commands | `shelf schemas` | Explore the JSON documents that shelf prints |
| Review how shelf is used | `shelf self` | Review shelf's own use |
<!-- /crumbkit:generated jobs -->

<!-- crumbkit:generated usage -->
## Most-used commands

The commands that callers run most, counting a failed or retried run twice, with their most common flags.

- `shelf schemas show`: Print the JSON Schema of each document
- `shelf books list [--limit N] [--all] [--json]`: List the books in the catalogue, in id order
- `shelf books show [--json]`: Show the books with the given ids, in the order given
- `shelf books summarize`: Describe the books with the given ids, or the whole catalogue, in one sentence
- `shelf self runs export [--after RUN]`: Print the records of shelf's usage log after a run, oldest first
<!-- /crumbkit:generated usage -->
