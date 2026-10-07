# 0008. Performance budgets

- The fast lane gates on gungraun instruction and allocation counts, each limit starting at the measured count plus 10 percent.
- Help pages stay within 1,500 bytes for the root, 800 for a group and 1,200 for a leaf.
- The slow lane times whole shelf runs against a 10 ms budget and fails above 30 ms.

Considered: criterion, divan, counting allocators.
