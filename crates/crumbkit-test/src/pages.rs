//! The help pages that the help checks read: the `-h` or `--help` page of
//! each visible command, as clap or crumbkit prints it to a pipe or as the
//! tool's binary prints it.

use std::process::ExitCode;

use clap::Command;
use clap::error::ErrorKind;
use crumbkit::Sink;
use crumbkit::help::Guide;

use crate::examples::{Examples, setup_problem};
use crate::runs::{self, Place, Ran, TIMEOUT, WorkDir, Xdg};
use crate::tree::{self, Visit};

#[cfg(test)]
mod tests;

/// Where the help checks read a tool's help pages, and the guide that
/// they read with them.
///
/// [`Pages::clap`] renders each page in the test with clap alone, as a tool
/// that parses with clap's own `parse` prints it to a pipe, so the help
/// checks run on a tool that does not use crumbkit's runtime.
/// [`Pages::rendered`] renders each page in the test with
/// `crumbkit::parse_to`, as a tool built with crumbkit prints it to a pipe.
/// [`Pages::binary`] runs the tool's binary for each page and reads what it
/// prints on standard output, which a tool needs when it builds part of its
/// help at run time, such as from settings it reads at start. The guide
/// gives rendered pages their goal headings and examples, and
/// [`check_path_weights`](crate::check_path_weights) reads its examples
/// with rendered and binary pages.
///
/// ```
/// use crumbkit::help::Guide;
/// use crumbkit_test::{Examples, Pages};
///
/// const GUIDE: Guide = Guide::new();
/// // In an integration test, the binary is `env!("CARGO_BIN_EXE_shelf")`.
/// const EXAMPLES: Examples<'_> = Examples::new("/path/to/shelf", "tests/golden/examples");
///
/// let clap = Pages::clap();
/// let rendered = Pages::rendered(&GUIDE);
/// let binary = Pages::binary(&GUIDE, &EXAMPLES);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Pages<'a> {
    /// Where the pages come from.
    source: Source<'a>,
}

/// Where [`Pages`] reads each page.
#[derive(Clone, Copy, Debug)]
enum Source<'a> {
    /// clap renders each page in the test, without a guide.
    Clap,
    /// `crumbkit::parse_to` renders each page in the test with the guide.
    Rendered(&'a Guide),
    /// The binary of the examples prints each page, and the guide gives
    /// what a check reads besides the pages.
    Binary(&'a Guide, &'a Examples<'a>),
}

impl<'a> Pages<'a> {
    /// Returns the pages that clap renders in the test for each command's
    /// path and help flag, as a tool that parses with clap alone prints
    /// them to a pipe. The pages have none of crumbkit's lines and no
    /// guide, so [`check_path_weights`](crate::check_path_weights) counts
    /// no output weight with them.
    ///
    /// RUST-10: clap's `wrap_help` feature wraps a page at the width of the
    /// terminal that runs the test, unless the tool fixes the width with
    /// `Command::term_width`. clap has no public getter for that width, so
    /// these pages cannot fix it for the tool. A tool with `wrap_help` and
    /// no fixed width sets one or reads its pages with [`Pages::binary`].
    #[must_use]
    pub const fn clap() -> Self {
        Self {
            source: Source::Clap,
        }
    }

    /// Returns the pages that `crumbkit::parse_to` renders with `guide` in
    /// the test, without the line that names a person's aliases.
    #[must_use]
    pub const fn rendered(guide: &'a Guide) -> Self {
        Self {
            source: Source::Rendered(guide),
        }
    }

    /// Returns the pages that the binary of `examples` prints, with
    /// `guide` for what a check reads besides the pages.
    ///
    /// Each page is a run of the binary with the command's words and the
    /// help flag, such as `books list -h`, in the environment, working
    /// directory and XDG fixture of `examples`, with fresh XDG directories
    /// for each run, as [`check_examples`](crate::check_examples) runs an
    /// example. A run reads no standard input. A run that exits with any
    /// code but 0, does not start or does not end within 10 seconds prints
    /// no help page. A tool that keeps a person's aliases in its XDG config
    /// directory prints none of them, because each run's directories start
    /// empty or as copies of the fixture.
    #[must_use]
    pub const fn binary(guide: &'a Guide, examples: &'a Examples<'a>) -> Self {
        Self {
            source: Source::Binary(guide, examples),
        }
    }

    /// Returns the guide, or `None` for clap's pages.
    pub(crate) const fn guide(self) -> Option<&'a Guide> {
        match self.source {
            Source::Clap => None,
            Source::Rendered(guide) | Source::Binary(guide, _) => Some(guide),
        }
    }
}

/// Returns the help page that `flag`, `-h` or `--help`, prints for `command`
/// and for each visible command below it, in depth-first order, with the
/// command it belongs to. It leaves out each hidden command and everything
/// below it, as help does.
///
/// With [`Pages::clap`], each page is the help that clap's
/// `Command::try_get_matches_from` returns for the command's path and
/// `flag`, such as `shelf books list -h`, as plain text without styles,
/// which is what clap prints to a pipe. Each page parses a clone of
/// `command`. With [`Pages::binary`], each page is what the binary prints on standard
/// output for the command's words and `flag`. With [`Pages::rendered`],
/// each page is what `crumbkit::parse_to` prints for the command's path and
/// `flag`, such as `shelf books list -h`, with the guide: the plain text that
/// a caller reads on a pipe, with the guide's goal headings, the leaf's
/// examples and columns, and crumbkit's lines. `parse_to` builds the tree
/// that it parses, so each page parses a clone of `command`. The pages
/// leave out the line that names a person's aliases, because those names
/// differ between machines (ADR-0027).
///
/// A command line that prints no help page, such as `-h` on a command that
/// turned its help flag off, adds a problem to `problems`, and the command
/// has no page. Settings of `Examples` that leave the binary unable to run,
/// such as a working directory that does not exist, add one problem, and no
/// command has a page.
pub(crate) fn pages<'a>(
    command: &'a Command,
    pages: Pages<'_>,
    flag: &str,
    problems: &mut Vec<String>,
) -> Vec<(Visit<'a>, String)> {
    let guide = match pages.source {
        Source::Clap => return clap_pages(command, flag, problems),
        Source::Binary(_, examples) => return binary_pages(command, examples, flag, problems),
        Source::Rendered(guide) => guide.without_personal_aliases(),
    };
    let mut pages = Vec::new();
    for visit in tree::walk_visible(command) {
        let line = format!("{} {flag}", visit.path());
        let mut out = Vec::new();
        // `()` takes no values from the matches, and help never reaches it.
        let result = crumbkit::parse_to::<(), _, _, _, _>(
            &mut Sink::new(&mut out, Vec::new()),
            command.clone(),
            &guide,
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

/// Returns the pages that clap renders, as [`pages`] describes.
fn clap_pages<'a>(
    command: &'a Command,
    flag: &str,
    problems: &mut Vec<String>,
) -> Vec<(Visit<'a>, String)> {
    let mut pages = Vec::new();
    for visit in tree::walk_visible(command) {
        let line = format!("{} {flag}", visit.path());
        // clap answers a help flag with an error of the kind `DisplayHelp`
        // that holds the page, and `StyledStr`'s `Display` leaves out the
        // styles, as clap does when it prints to a pipe.
        match command.clone().try_get_matches_from(line.split(' ')) {
            Err(error) if error.kind() == ErrorKind::DisplayHelp => {
                pages.push((visit, error.render().to_string()));
            }
            _ => problems.push(format!("{line}: the command line prints no help page")),
        }
    }
    pages
}

/// Returns the pages that the binary of `examples` prints, as [`pages`]
/// describes.
fn binary_pages<'a>(
    command: &'a Command,
    examples: &Examples<'_>,
    flag: &str,
    problems: &mut Vec<String>,
) -> Vec<(Visit<'a>, String)> {
    if let Some(problem) = setup_problem(examples) {
        problems.push(problem);
        return Vec::new();
    }
    let mut pages = Vec::new();
    for visit in tree::walk_visible(command) {
        let line = format!("{} {flag}", visit.path());
        let below = visit.below();
        let args: Vec<&str> = below.split_whitespace().chain([flag]).collect();
        let output = Xdg::new("help", examples.xdg_fixture()).and_then(|xdg| {
            let work = WorkDir::new("help-dir", examples.working_dir())?;
            let place = Place {
                dir: work.path(),
                stdin: None,
                env: &[],
            };
            runs::run(
                examples.binary(),
                examples.variables(),
                &xdg,
                &args,
                place,
                TIMEOUT,
            )
        });
        match output {
            Ok(Ran::Exited(output)) if output.status.success() => {
                pages.push((visit, String::from_utf8_lossy(&output.stdout).into_owned()));
            }
            Ok(Ran::Exited(output)) => problems.push(format!(
                "{line}: the command line prints no help page, and the binary ends with {}",
                output.status
            )),
            Ok(Ran::Stopped) => problems.push(format!(
                "{line}: the command line prints no help page, because {}",
                runs::stopped(TIMEOUT)
            )),
            Err(problem) => problems.push(format!(
                "{line}: the command line prints no help page, because {problem}"
            )),
        }
    }
    pages
}
