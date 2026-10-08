//! The record listing check. Every decision record except the root is linked
//! from the `README.md` of the node above it, and every file link in a node
//! record names a file that exists.

use std::collections::{BTreeMap, BTreeSet};

use super::adr_records::is_record;
use super::{File, Violation};

#[cfg(test)]
mod tests;

/// The file name of every node record.
const NODE: &str = "README.md";

/// The node record at the top of the tree, which no node lists.
const ROOT: &str = "docs/decisions/README.md";

/// Reports records that the node above them does not list, and node links to
/// files that do not exist.
pub(super) fn check(files: &[File]) -> Vec<Violation> {
    let paths: BTreeSet<&str> = files.iter().map(|file| file.path.as_str()).collect();
    let links_by_node: BTreeMap<&str, Vec<&str>> = files
        .iter()
        .filter(|file| is_record(&file.path) && file.path.ends_with(NODE))
        .map(|file| (file.path.as_str(), links(&file.contents).collect()))
        .collect();

    let mut violations = Vec::new();
    for file in files {
        if !is_record(&file.path) || file.path == ROOT {
            continue;
        }
        let (node, link) = listing(&file.path);
        let listed = links_by_node
            .get(node.as_str())
            .is_some_and(|links| links.contains(&link.as_str()));
        if !listed {
            violations.push(Violation::UnlistedRecord {
                path: file.path.clone(),
                node,
            });
        }
    }

    for (node, links) in &links_by_node {
        for target in links {
            if !paths.contains(resolve(node, target).as_str()) {
                violations.push(Violation::BrokenLink {
                    path: (*node).to_owned(),
                    target: (*target).to_owned(),
                });
            }
        }
    }
    violations
}

/// Returns the node record that should list the record at `path`, and the
/// link that node should use. A record's node is the `README.md` beside it,
/// and a node's own node is the `README.md` one directory up.
fn listing(path: &str) -> (String, String) {
    let (directory, name) = path.rsplit_once('/').unwrap_or(("", path));
    if name == NODE {
        let (parent, child) = directory.rsplit_once('/').unwrap_or(("", directory));
        (format!("{parent}/{NODE}"), format!("{child}/{NODE}"))
    } else {
        (format!("{directory}/{NODE}"), name.to_owned())
    }
}

/// Returns the file part of every Markdown link target in `contents`,
/// leaving out web links and links within the page.
fn links(contents: &str) -> impl Iterator<Item = &str> {
    contents
        .split("](")
        .skip(1)
        .filter_map(|rest| rest.split_once(')').map(|(target, _)| target))
        .filter(|target| !target.contains("://") && !target.starts_with('#'))
        .map(|target| target.split_once('#').map_or(target, |(file, _)| file))
}

/// Returns the repository path that `target`, a link written in `node`,
/// points to.
fn resolve(node: &str, target: &str) -> String {
    let mut segments: Vec<&str> = node.split('/').collect();
    segments.pop();
    for segment in target.split('/') {
        match segment {
            ".." => {
                segments.pop();
            }
            "." | "" => {}
            _ => segments.push(segment),
        }
    }
    segments.join("/")
}
