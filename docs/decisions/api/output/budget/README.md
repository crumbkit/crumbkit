# ADR-0092. Output budget

- Date: 2026-10-19
- Status: accepted

## Decision

These records decided the output budget that limits the bytes of one run:
its size and how a guide or a variable lowers it, the cut line of a result
that does not fit, and the failures that name each answer of a batch that
it leaves out.

## Children

- [ADR-0046. Output budget](0046-output-budget.md)
- [ADR-0068. One failure for each answer left out](0068-left-out-per-key.md)
- [ADR-0069. Output budget floor](0069-output-budget-floor.md)
- [ADR-0070. Every answer left out keeps its failure](0070-left-out-keys-always-answered.md)
