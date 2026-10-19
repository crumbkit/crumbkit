//! A tool outside crumbkit that reads a refused command line before
//! anything prints, as a tool does that resolves its own aliases after a
//! refusal or prints its own failure document.

use std::ffi::OsString;

use clap::{CommandFactory as _, Parser, Subcommand};
use crumbkit::help::Guide;
use crumbkit::{Sink, Unparsed, ViewFlags};

/// A tool with a `list` command.
#[derive(Debug, Parser)]
#[command(name = "tool")]
struct Tool {
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    view: ViewFlags,
}

#[derive(Debug, Subcommand)]
enum Command {
    List,
}

/// What a run printed on standard output and standard error.
type Printed = (String, String);

/// Parses `args` with `try_parse_to`, then prints whatever it returned
/// unprinted, and returns the typed words, the culprit and what printed
/// before and after the print.
fn refuse(args: &[&str]) -> (Vec<OsString>, Option<String>, Printed, Printed) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let parsed = crumbkit::try_parse_to::<Tool, _, _, _, _>(
        &mut Sink::new(&mut out, &mut err),
        Tool::command(),
        &Guide::new(),
        args,
    );
    let before = (
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    );
    let (typed, culprit, unparsed) = match parsed {
        Ok(_) => return Default::default(),
        Err(Unparsed::Refused(refused)) => (
            refused.typed().to_vec(),
            refused.culprit().map(ToOwned::to_owned),
            Unparsed::Refused(refused),
        ),
        Err(unparsed) => (Vec::new(), None, unparsed),
    };
    let mut out = Vec::new();
    let mut err = Vec::new();
    let _ = unparsed.print_to(&mut Sink::new(&mut out, &mut err));
    let after = (
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    );
    (typed, culprit, before, after)
}

#[test]
fn a_tool_reads_the_typed_words_and_culprit_of_a_refusal_before_it_prints() {
    let (typed, culprit, before, after) = refuse(&["tool", "lst", "--json"]);

    assert_eq!(typed, ["lst", "--json"]);
    assert_eq!(culprit.as_deref(), Some("lst"));
    assert_eq!(before, Printed::default());
    assert!(
        after.0.contains("\"code\":\"tool::cli::usage\""),
        "{after:?}"
    );
}

#[test]
fn a_tool_reads_the_help_word_that_the_caller_typed() {
    let (typed, culprit, before, after) = refuse(&["tool", "help", "lst"]);

    assert_eq!(typed, ["help", "lst"]);
    assert_eq!(culprit.as_deref(), Some("lst"));
    assert_eq!(before, Printed::default());
    assert!(after.1.contains("code:  tool::cli::usage"), "{after:?}");
}
