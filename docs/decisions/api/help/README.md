# ADR-0022. Help

- Date: 2026-10-09
- Status: accepted

## Decision

These records decided what a tool's help shows and how a caller reaches
it: the lines crumbkit adds to clap's help pages, goal headings, the
message of a refused command line, crumbkit's own `help` command, the
help topics that a tool declares, the examples and column descriptions of
leaf commands, the export of a branch of commands with their arguments,
the Markdown reference of every command, the checks of a tool's help
pages, the exit codes that a tool declares, and the measure of the bytes
that each path through a tool's help costs.

## Children

- [ADR-0021. Help pages](0021-help-pages.md)
- [ADR-0023. Help command and topics](0023-help-command-and-topics.md)
- [ADR-0024. Leaf examples and columns](0024-leaf-examples-and-columns.md)
- [ADR-0025. Help checks](0025-help-checks.md)
- [ADR-0026. Branch export](0026-branch-export.md)
- [ADR-0033. Help weight](0033-weigh.md)
- [ADR-0037. Exit codes](0037-exit-codes.md)
- [ADR-0044. Command reference](0044-command-reference.md)
- [ADR-0101. tool help prints the root's help](0101-help-prints-the-root-help.md)
- [ADR-0102. Styled sinks and compact groups](0102-styled-sinks-and-compact-groups.md)
- [ADR-0103. Exit ranges and signal codes](0103-exit-ranges-and-signal-codes.md)
- [ADR-0104. An opt-out for the example rule of check_guide](0104-examples-opt-out.md)
