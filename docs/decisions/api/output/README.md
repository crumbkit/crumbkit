# ADR-0031. Output

These records decide how a tool prints a command's result: the compact text table, the JSON document, the columns that a row type names, the failures, batches and listings that print beside the rows, the failure forms that a tool sets in place of crumbkit's failure document, the streams that print rows as they arrive, the output budget that limits the bytes of one run, the child processes that a command hands the terminal to, and the terminal detection that no result depends on.

## Children

- [ADR-0041. Streams](0041-streams.md)
- [ADR-0042. Child processes](0042-child-processes.md)
- [ADR-0045. Failure forms](0045-failure-forms.md)
- [ADR-0046. Output budget](0046-output-budget.md)
- [ADR-0049. Terminal detection](0049-terminal-detection.md)
- [ADR-0050. Listings](listings/README.md)
- [ADR-0051. Result shapes](results/README.md)
