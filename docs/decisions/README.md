# ADR-0017. Decision records

- Each record states a decision as crumbkit holds it now, in one to three sentences per point, and ends with a one-line list of the alternatives considered. A changed decision is edited in place, because git history keeps the old version.
- Records form a tree. Every directory's `README.md` is itself a record that says which decisions belong under it and lists its children. When a directory nears the 5,000-word limit of ADR-0010, its records split by topic into child directories, and each new directory's `README.md` takes the next free number.
- A record's number is global and never reused, and moving a record keeps its number. Text anywhere in the repository cites a record as `ADR-NNNN`.

Considered: a flat directory, numbers local to each directory, citing records by file path.

## Children

- [ADR-0018. API](api/README.md)
- [ADR-0019. Testing](testing/README.md)
- [ADR-0020. Project](project/README.md)
