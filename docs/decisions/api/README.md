# ADR-0018. API

- Date: 2026-10-08
- Status: accepted

## Decision

These records decided what an adopter writes and what their tool's
callers see: the error model and failure forms, the public API and its
compatibility promises, the clap-free core crate, JSON documents and the
commands that serve their schemas, how a tool adopts crumbkit and opts out
of its conventions, shell completion, the result formats, help and the
checks of a tool's help pages, command effects and the questions that pick
missing values, personal aliases, the run observer, and the usage log with
its trace context.

## Children

- [ADR-0002. Public API and semver](0002-public-api-and-semver.md)
- [ADR-0027. Aliases](0027-aliases.md)
- [ADR-0048. Shell completion](0048-completion.md)
- [ADR-0057. Run observer](0057-run-observer.md)
- [ADR-0075. Several run observers](0075-several-observers.md)
- [ADR-0079. The tool's name in place of crumbkit's](0079-tool-names.md)
- [ADR-0081. Seams for one adopter removed](0081-seams-for-one-adopter-removed.md)
- [ADR-0085. Public items without a caller removed](0085-public-items-without-a-caller-removed.md)
- [ADR-0054. Adoption](adoption/README.md)
- [ADR-0055. Failures](failures/README.md)
- [ADR-0022. Help](help/README.md)
- [ADR-0038. Effects](effects/README.md)
- [ADR-0040. Schemas](schemas/README.md)
- [ADR-0028. Usage](usage/README.md)
- [ADR-0031. Output](output/README.md)
- [ADR-0099. The types that need no clap](core/README.md)
