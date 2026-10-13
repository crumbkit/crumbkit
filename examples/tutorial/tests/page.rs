//! The test that keeps `docs/tutorial.md` in step with the code of this
//! package, so every block of Rust that the page shows compiles.
//!
//! Each Rust block on the page belongs to the file that the nearest link
//! before it names under `examples/tutorial/`, and the test requires that
//! file to hold the block word for word.

use std::fs;
use std::path::Path;

/// The directory of the tutorial page, which its links start from.
const DOCS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs");

/// The prefix of a link from the page to a file of this package.
const LINK: &str = "](../examples/tutorial/";

/// One block of Rust on the page and the file that the link before it
/// names, relative to `docs/`.
struct Block {
    /// The file that must hold the block, such as
    /// `../examples/tutorial/src/bin/step_1.rs`.
    file: String,
    /// The block's lines, each ending with a line break.
    code: String,
    /// The page's line number of the block's opening fence.
    line: usize,
}

/// Returns the Rust blocks of `page`, each with the file that the last link
/// to this package before it names.
///
/// RUST-10: this reads only what the page uses: a fence of three backticks
/// followed by `rust`, and a link that sits on one line. A Markdown parser
/// such as pulldown-cmark replaces it if the page needs nested or indented
/// fences.
fn blocks(page: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut file = String::new();
    let mut open: Option<Block> = None;
    for (index, text) in page.lines().enumerate() {
        if let Some(block) = open.as_mut() {
            if text == "```" {
                blocks.extend(open.take());
            } else {
                block.code.push_str(text);
                block.code.push('\n');
            }
            continue;
        }
        if text == "```rust" {
            open = Some(Block {
                file: file.clone(),
                code: String::new(),
                line: index + 1,
            });
            continue;
        }
        if let Some(start) = text.rfind(LINK) {
            let target = &text[start + 2..];
            if let Some(end) = target.find(')') {
                target[..end].clone_into(&mut file);
            }
        }
    }
    blocks
}

#[test]
fn every_rust_block_on_the_page_is_part_of_the_file_linked_before_it() {
    let page = fs::read_to_string(Path::new(DOCS).join("tutorial.md")).unwrap();
    let blocks = blocks(&page);
    assert!(!blocks.is_empty(), "docs/tutorial.md shows no Rust block");
    let mut problems = Vec::new();
    for block in &blocks {
        if block.file.is_empty() {
            problems.push(format!(
                "line {}: no link to a file of examples/tutorial comes before the block",
                block.line
            ));
            continue;
        }
        match fs::read_to_string(Path::new(DOCS).join(&block.file)) {
            Ok(source) if source.contains(&block.code) => {}
            Ok(_) => problems.push(format!(
                "line {}: {} does not hold the block word for word",
                block.line, block.file
            )),
            Err(error) => problems.push(format!(
                "line {}: {} cannot be read: {error}",
                block.line, block.file
            )),
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
