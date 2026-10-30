# crumbkit-markdown

crumbkit-markdown writes a Markdown reference of every command of a tool
built with crumbkit. It reads the document that the tool's
`commands list --all --json --full` prints, so the reference says what the
tool's help says.

## Adding it

Most tools use crumbkit-markdown through two other crates.
`cargo crumbkit docs --bin NAME` writes the reference to
`docs/commands.md` in the binary's package, and crumbkit-test's
`check_command_reference` fails a test when that file differs from a fresh
reference. A program that renders the reference itself adds the crate:

```toml
[dependencies]
crumbkit-markdown = "0.1"
```

## Example

[`render`] takes the bytes of the document and returns a [`Reference`]:

```rust
let document = br#"{
  "schema": "shelf/commands-list/v1",
  "data": [{
    "path": "shelf", "about": "Browse a small catalogue", "long_about": null,
    "arguments": [], "examples": [], "columns": [], "form": null,
    "line": null, "children": null, "exits": [], "effects": null
  }],
  "total": 1
}"#;

let reference = crumbkit_markdown::render(document).unwrap();
assert_eq!(reference.commands(), 1);
assert!(reference.text().starts_with("# Commands of `shelf`\n"));
assert!(reference.text().contains("\n## `shelf`\n\nBrowse a small catalogue\n"));
```

The reference has one section for each command, in the order of the
document. A section holds the command's description, links to its
subcommands, its arguments, examples and columns, the effects, stream,
text and children lines that its help prints, and its exit codes. Each heading's anchor is the
one GitHub gives it. The reference escapes the markup characters in help
text and keeps its code spans.

[`render`] returns a [`RenderError`] when the input is not a full
`<tool>/commands-list/v1` document, or when two headings would have one
anchor. The [command reference docs] describe what the reference holds,
and [ADR-0044] records why it reads the document.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.

[`render`]: src/render.rs
[`Reference`]: src/render.rs
[`RenderError`]: src/error.rs
[ADR-0044]: ../../docs/decisions/api/help/0044-command-reference.md
[command reference docs]: ../../docs/behaviour/help/reference.md
