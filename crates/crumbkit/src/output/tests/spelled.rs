//! Tests that a tool sets its spelling once, on its guide: `parse`,
//! `finish`, `stream` and `Output::new` print every code in the guide's
//! spelling, with the tool's name in crumbkit's own codes, and in the colon
//! spelling when the guide sets none.

use std::fmt;

use clap::{CommandFactory as _, Parser, Subcommand};

use super::{PLANETS, Planet};
use crate::help::Guide;
use crate::{Code, Document, Failure, Output, Recovery, Sink, Spelling, Stream, ViewFlags};
use crate::{finish_to, parse_to, stream_to};

/// A spelling that joins the segments of a code with `.`.
const DOTS: Spelling =
    Spelling::custom(|code, f| write!(f, "{}.{}.{}", code.component(), code.area(), code.reason()));

/// A guide that sets the dotted spelling and nothing else.
const DOTTED: Guide = Guide::new().spelling(DOTS);

/// A stream of planets, one line for each planet as it arrives.
const PLANETS_WATCH: Document<Stream<Planet>> =
    Document::new("solar", "planets-watch", 1, "Each planet as it arrives.");

const MISSING: Code = Code::new("tool", "planets", "missing");

/// A failure that the handler returns when a planet does not exist.
#[derive(Debug)]
struct Missing;

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("no such planet")
    }
}

impl std::error::Error for Missing {}

impl Failure for Missing {
    fn code(&self) -> &'static Code {
        &MISSING
    }

    fn recovery(&self) -> Recovery {
        Recovery::Hint("name a planet of the solar system".into())
    }
}

/// A tool with one `list` command and the view flags.
#[derive(Parser)]
#[command(name = "tool")]
struct Tool {
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    view: ViewFlags,
}

#[derive(Subcommand)]
enum Command {
    List,
}

/// Parses `args` with `guide` through `parse_to` into memory, and returns
/// the output of a run that parsed, or the error writer's text of one that
/// stopped.
fn parse(guide: &Guide, args: &[&str]) -> Result<Output, String> {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let parsed = parse_to::<Tool, _, _, _, _>(
        &mut Sink::new(&mut out, &mut err),
        Tool::command(),
        guide,
        args,
    );
    parsed
        .map(|(_, output)| output)
        .map_err(|_| String::from_utf8(err).unwrap())
}

/// Finishes the handler's `Missing` failure with `output` into memory, and
/// returns the error writer's text.
fn fail(output: &mut Output) -> String {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result: Result<Vec<Planet>, Missing> = Err(Missing);
    finish_to(&mut Sink::new(&mut out, &mut err), output, PLANETS, result);
    String::from_utf8(err).unwrap()
}

#[test]
fn a_guide_without_a_spelling_prints_colon_spelled_codes_with_the_tools_name() {
    let refused = parse(&Guide::new(), &["tool", "lisst"]).unwrap_err();
    let mut output = parse(&Guide::new(), &["tool", "list"]).unwrap();

    assert!(
        refused.ends_with("  code:  tool::cli::usage\n"),
        "{refused}"
    );
    let failed = fail(&mut output);
    assert!(
        failed.ends_with("  code:  tool::planets::missing\n"),
        "{failed}"
    );
}

#[test]
fn a_guide_with_a_custom_spelling_prints_it_when_parse_refuses_a_command_line() {
    let refused = parse(&DOTTED, &["tool", "lisst"]).unwrap_err();

    assert!(refused.ends_with("  code:  tool.cli.usage\n"), "{refused}");
}

#[test]
fn a_guide_with_a_custom_spelling_prints_it_through_finish_without_a_further_argument() {
    let mut output = parse(&DOTTED, &["tool", "list"]).unwrap();

    let failed = fail(&mut output);

    assert!(
        failed.ends_with("  code:  tool.planets.missing\n"),
        "{failed}"
    );
}

#[test]
fn a_guide_with_a_custom_spelling_prints_it_through_stream_without_a_further_argument() {
    let mut output = parse(&DOTTED, &["tool", "list"]).unwrap();
    let mut out = Vec::new();
    let mut err = Vec::new();

    let mut sink = Sink::new(&mut out, &mut err);
    let streaming = stream_to(&mut sink, &mut output, PLANETS_WATCH).unwrap();
    let _ = streaming.end(Err::<(), Missing>(Missing));

    let err = String::from_utf8(err).unwrap();
    assert!(err.ends_with("  code:  tool.planets.missing\n"), "{err}");
}

#[test]
fn a_clap_only_tools_output_new_carries_the_guides_spelling() {
    let args = ["tool", "list", "--full"];
    let view = Tool::try_parse_from(args).unwrap().view;
    let mut out = Vec::new();
    let mut err = Vec::new();

    // `--full` without `--json` is refused with crumbkit's usage code, which
    // prints with the tool's name.
    let stop = Output::new_to(
        &mut Sink::new(&mut out, &mut err),
        view,
        Tool::command(),
        &DOTTED,
        args,
    )
    .unwrap_err();

    assert_eq!(stop.number(), 2);
    let refused = String::from_utf8(err).unwrap();
    assert!(refused.ends_with("  code:  tool.cli.usage\n"), "{refused}");
}

#[test]
fn a_clap_only_tools_output_new_prints_the_handlers_code_in_the_guides_spelling() {
    let args = ["tool", "list"];
    let view = Tool::try_parse_from(args).unwrap().view;
    let mut output = Output::new(view, Tool::command(), &DOTTED, args).unwrap();

    let failed = fail(&mut output);

    assert!(
        failed.ends_with("  code:  tool.planets.missing\n"),
        "{failed}"
    );
}
