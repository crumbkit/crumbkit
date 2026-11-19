# Guides for usage findings

This file holds one row for each finding that a tool's `self usage`
reports. The "Suggest guides from usage" job of cargo crumbkit's skill
reads it.

- **Agents were probably trying to** says what the runs suggest the agent
  wanted, and which figure or run shows it.
- **Guide text** is a template for one line of the tool's `SKILL.md` or a
  help topic. Replace `<bin>`, `<command>` and the other placeholders with
  the tool's real names and figures.
- **Tool change** says what change to the tool would remove the cause.
- **Affordance** names the rows of `AFFORDANCES.md` that the tool change
  would use.
- **The guide alone is enough when** says when the guide text is the
  whole fix and the tool needs no change.

| Finding | Agents were probably trying to | Guide text | Tool change | Affordance | The guide alone is enough when |
| --- | --- | --- | --- | --- | --- |
| `one id at a time` | Look up several items, without knowing that the command takes many ids. | "`<bin> <command>` takes many ids in one call; pass every id at once." | Give the leaf an example with several ids. When the command takes one id, make it take many and report each id's failure in its row. | `Batch`, `fetch_each`, `Leaf::examples` | The command already takes many ids and its help shows an example with several. |
| `identical repeat` | Get a result they did not see or trust. The run before the repeat often printed nothing (`out_bytes` 0) or took long (`ms`). | For a slow command: "`<bin> <command>` takes up to <p95> s and prints nothing until it ends; wait for it to exit instead of running it again." For a silent one: "`<bin> <command>` prints nothing when it succeeds; exit 0 means it worked." | Show progress while the command waits, or print a sentence result on success. | `Run::progress_allowed`, `stream`, `Prose` | The command already prints a result and shows progress, and the repeats come from sessions that checked state again on purpose. |
| `help loop` | Find the command for a goal that no help page named. The help pages before the first run, and the command the session ran after them, show the goal. | "To <goal>, run `<bin> <command> <ARGS>`." | Add a goal heading that names the goal, an example on the leaf, or a topic for a concept that spans commands. | `Group`, `Goal`, `Leaf::examples`, `Topic` | The command the session reached already has an example for the goal, and only the route to it was missing. |
| `usage error` | Use a flag or argument the command lacks, often a name from another tool or an older version. The row's nearest flag and the flags in `self runs list` show which. | "`<bin> <command>` takes `--<nearest> <VALUE>` for <purpose>." Name the missing flag in prose only, because the skill check refuses a code span with a flag the tool lacks. | Add the flag when agents name a real need, add a hidden alias for a renamed flag, or give the refusal a hint that names the right line. | `check_names`, `Leaf::refusal_hint`, `Leaf::refusal_alternatives` | The flag the agents typed has no meaning for this tool. |
| `retry after failure` | Get past a failure that looked temporary, or that printed no next step. The row's code and the command the session ran after the retry show what repaired it. | "When `<bin> <command>` fails with `<code>`, <repair> before running it again." | Give the failure a typed recovery that names the repair. Offer a retry only when the next run starts clean. | `Recovery::after`, `Recovery::Retry`, `Recovery::program` | The repair depends on the caller's environment, such as a login or a network, which no recovery of the tool can name. |
| `short page` | See every row after the default page cut the listing short. | "`<bin> <command>` shows <N> rows by default; add `--all` when you need every row." | Raise the default page, add a filter so the agent asks for fewer rows, or describe the cut in the listing's summary. | `Page<N, MOST>`, `Listing::described` | Most callers need only the default page, and the sessions that need more are a minority. |
| `heavy output` | Read one fact from a large result. The `p95_out_bytes` figure gives the size. | "`<bin> <command>` prints about <p95> KB; use `--fields <NAMES>` or a filter to print only what you need." | Move wide columns behind `--long`, add a filter, or set an output budget. | `LONG_COLUMNS`, `Row::in_default_text`, `tool::Output::budget` | The command already has a flag or filter that narrows its output. |
| `slow command` | Wait for work that takes long. The `p95_ms` figure gives the time. | "`<bin> <command>` takes up to <p95> s; run it once and wait for it to exit." | Show progress, run lookups in parallel, or stop fetching rows that will not print. | `Run::progress_allowed`, `fetch_each`, `Run::room` | The work is slow by nature and the command already shows progress. |
| `mostly waiting` | Wait on another system or on a lock that another run holds. The `lock_ms` column of `self runs list` shows lock waits. | "`<bin> <command>` waits on <system or lock>; a second run at the same time makes both wait longer." | Report what the run waits on, and give the wait a deadline that names what is late. | `Facts`, `Extension`, `RunOptions::deadline`, `RunOptions::overdue` | The wait is in another system that the tool cannot speed up, and the tool already reports it. |
