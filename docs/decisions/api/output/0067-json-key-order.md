# ADR-0067. JSON key order

- Date: 2026-10-17
- Status: amended by ADR-0073

## Context

crumbkit printed the keys of each JSON object in the order that its value
serialized them: the envelope's type key first, then the data key and
crumbkit's own names, and a row's fields in their declaration order. Some
tools already printed documents whose every object held its keys in sorted
order, and whose readers compared the bytes. Such a tool could not move
`total`, `truncated` or a summary field, and it hand-wrote a `Serialize`
for a row that flattens `Bytes`, so that `base64` printed before `blob` and
`text` printed last.

## Decision

- A tool set `Guide::json_keys(KeyOrder::Sorted)`. `KeyOrder` was a
  `#[non_exhaustive]` enum with only `Sorted`, because a variant for the
  default would only name it.
- The order covered every object at every depth of every JSON document and
  JSON line that a run printed on standard output: the envelope, each row,
  a summary, each failure, the document of a failure form, each line of a
  stream, and the JSON lines of a text output. Sorting only the envelope
  could not reorder the fields of a row that flattens `Bytes`.
- Keys sorted by Rust's `str` comparison, which compares Unicode code
  points, because `serde_json` already sorts by it and crumbkit then needs
  no comparison of its own.
- crumbkit serialized a sorted document into a `serde_json::Value`, sorted
  each object with `Value::sort_all_objects`, and wrote the value in the
  guide's layout. `sort_all_objects` sorts whether or not another crate
  turns on serde_json's `preserve_order` feature.
- The order was a declaration (ADR-0056), because it changed where keys
  printed and kept every key and value, so it cost an agent nothing and
  took no reason. `check_opt_outs` listed it as the `json-keys`
  customisation of every run.

## Consequences

A tool whose readers expect sorted keys keeps every part of crumbkit's
envelope and deletes its hand-written `Serialize`. A sorted document no
longer starts with its type key. crumbkit builds one `Value` tree for each
sorted document that it writes or measures. A sorted document that holds an
integer outside the ranges of `i64` and `u64` fails as a failed write, and
of two equal keys in one object only the last prints. A tool that does not
set the order prints as before.

Considered: sorting the envelope only, the order of RFC 8785, which
compares UTF-16 code units, a serializer that sorts while it writes, an
opt-out with a reason, a per-document setting.
