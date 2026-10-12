//! The help pages that the help checks read: the `-h` or `--help` page of
//! each visible command, as crumbkit prints it to a pipe.

use std::process::ExitCode;

use clap::Command;
use crumbkit::help::Guide;
use crumbkit::{Sink, Spelling};

use crate::tree::{self, Visit};

#[cfg(test)]
mod tests;

/// Returns the help page that `flag`, `-h` or `--help`, prints for `command`
/// and for each visible command below it, in depth-first order, with the
/// command it belongs to. It leaves out each hidden command and everything
/// below it, as help does.
///
/// Each page is what `crumbkit::parse_to` prints for the command's path and
/// `flag`, such as `shelf books list -h`, with `guide`: the plain text that
/// a caller reads on a pipe, with the guide's goal headings, the leaf's
/// examples and columns, and crumbkit's lines. `parse_to` builds the tree
/// that it parses, so each page parses a clone of `command`.
///
/// A command line that prints no help page, such as `-h` on a command that
/// turned its help flag off, adds a problem to `problems`, and the command
/// has no page.
pub(crate) fn pages<'a>(
    command: &'a Command,
    guide: &Guide,
    flag: &str,
    problems: &mut Vec<String>,
) -> Vec<(Visit<'a>, String)> {
    let mut pages = Vec::new();
    for visit in tree::walk_visible(command) {
        let line = format!("{} {flag}", visit.path());
        let mut out = Vec::new();
        // `()` takes no values from the matches, and help never reaches it.
        // The spelling only spells the code of a refusal, which the problem
        // below leaves out.
        let result = crumbkit::parse_to::<(), _, _, _, _>(
            &mut Sink::new(&mut out, Vec::new()),
            command.clone(),
            guide,
            Spelling::COLONS,
            line.split(' '),
        );
        if result.err().map(ExitCode::from) == Some(ExitCode::SUCCESS) {
            pages.push((visit, String::from_utf8_lossy(&out).into_owned()));
        } else {
            problems.push(format!("{line}: the command line prints no help page"));
        }
    }
    pages
}
