# ADR-0156. Hyphens in code segments

- Date: 2026-10-25
- Status: accepted
- Supersedes: ADR-0001, which it amends

## Context

A code segment was one or more lowercase ASCII letters, digits and
underscores, and a failure's field names followed the same rule. A tool
that hosts plugins reports a plugin's failure under a code whose component
is the plugin's name, which the tool learns at run time and passes to
`Code::runtime`. Plugin names, package names and command names often hold a
hyphen, such as `book-club`. `Code::runtime` refused such a component, so
the tool fell back to a generic code of its own and the caller lost the
plugin's code. The alternative was to rename the plugin or to spell its
name with an underscore in the code, so the code no longer matched the name
that the caller knows.

The failure document's schema already accepts any printable ASCII code,
and `check_codes` already accepts a hyphen, so only `Code::new` and
`Code::runtime` refused one.

## Decision

- Every segment of a code, the component, the area and the reason, is one
  or more words joined by single hyphens, where each word is one or more
  lowercase ASCII letters, digits and underscores. `book-club::gate::unmet`
  and `shelf::reading-list::not_found` are codes.
- A hyphen never leads a segment, so a code in the colon spelling never
  starts with `-` and never reads as a flag when a caller passes it as an
  argument. A hyphen never trails a segment and never follows another
  hyphen, so every hyphen joins two words.
- `Code::new` and `Code::runtime` apply the one rule, so a code that a tool
  declares and a code that it learns at run time follow the same grammar.
- A field name keeps the old rule of lowercase ASCII letters, digits and
  underscores, because the failure document's schema promises every reader
  that rule for the keys of `fields`.
- `ToolName` still spells crumbkit's own codes with `_` in place of each
  `-` of the tool's name, such as `my_tool::cli::usage`. The code component
  is also the start of the tool's environment variables, which cannot hold
  a `-`, and the codes that crumbkit prints for an existing tool do not
  change.

## Consequences

A tool passes a hyphenated plugin name to `Code::runtime` as it is, and its
caller sees the plugin's own code. A tool that matched codes with a pattern
of letters, digits, underscores and colons alone needs to add `-`. A tool
named `my-tool` that declares its own codes with the component `my-tool`
prints two components: `my-tool::…` for its own codes and `my_tool::…` for
crumbkit's.

Considered: a hyphen anywhere in a segment, including at either end; a
hyphen in the component alone; a hyphen in field names too; spelling
crumbkit's own codes for `my-tool` as `my-tool::cli::usage`.
