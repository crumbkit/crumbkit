# ADR-0008. Performance budgets

- The fast lane will fail a pull request that goes over a limit on gungraun's instruction or allocation counts, which stay the same from run to run on shared CI machines, once benchmarks for them exist. Each limit will start at the measured count plus 10 percent.
- Help pages stay within 1,500 bytes for the root, 800 for a group and 1,200 for a leaf.
- The slow lane will time whole shelf runs against a 10 ms budget and fail above 30 ms once code for the timing exists.

Considered: criterion, divan, counting allocators.
