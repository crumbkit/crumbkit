# ADR-0023. Help command and topics

- [`parse`](../../../../crates/crumbkit/src/parse.rs) answers the word `help` itself and turns off clap's generated `help` subcommand, so clap's `help` line leaves every `Commands:` list and `Command::build` no longer copies the tree into a generated help command.
- `parse` answers `help` at every level, as clap's `help` did, because a refused request for help gives the caller nothing. It answers before clap parses, so the tool's own type never sees the word, and a tool's own command named `help` still takes the word.
- `<tool> help` lists the tool's topics, and `<tool> help <TOPIC>` prints one. Topics belong to the root.
- Every other `help` command line becomes the same command line with `--help`, and an unknown word gets clap's usual refusal with its suggestions.
- A tool declares its topics in its `Guide`, each a [`Topic`](../../../../crates/crumbkit/src/help/topic.rs) of a name, a one-line summary and plain text. Topic text is a literal or comes from `include_str!`. `parse` reads the topics from the guide, so the tree holds no topics and no run copies a topic's text.
- crumbkit-test's [`check_guide`](../../../../crates/crumbkit-test/src/guide.rs) checks the topics, and [`check_grammar`](../../../../crates/crumbkit-test/src/grammar.rs) reserves the word `help` for asking for help.
- The topic list and a topic's text print as plain text, also on a terminal. The root's help names the topics on one line.

Considered: the hidden `help` command carrying the topics, clap's `unstable-ext` extension slot, a deferred builder per leaf, a `Help` variant in the tool's enum, `help` only at the root, crumbkit writing its own message for an unknown word, topic files read at runtime, a Topics section with summaries on the root page, keeping clap's `help` for tools without topics.
