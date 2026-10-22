//! Tests for a leaf whose command prints no result on standard output, such
//! as a server that runs until it is interrupted: `parse` refuses the view
//! flags, accepts `--json`, and its help prints an `Output:` line.

use std::fmt;
use std::process::ExitCode;

use clap::{Args as _, Command};

use super::parse_to;
use crate::help::{Guide, Leaf};
use crate::twins::assert_one_word_apart;
use crate::{Code, Failure, Output, Recovery, Sink};

const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["serve"]).prints_no_result()]);

/// The `Output:` line of a leaf that prints no result.
const OUTPUT_LINE: &str =
    "Output: no result on standard output, and --json prints a failure as JSON.";

/// A tool with crumbkit's output flags on its root and one leaf, `serve`.
fn tool() -> Command {
    crate::ViewFlags::augment_args(Command::new("tool"))
        .subcommand_required(true)
        .subcommand(Command::new("serve").about("Serve until interrupted"))
}

/// What `parse_to` printed, its exit code when it did not return a parsed
/// type, and the output when it did.
struct Parsed {
    code: Option<ExitCode>,
    output: Option<Output>,
    out: String,
    err: String,
}

/// Runs `parse_to` over `tool()` with the guide's leaf and `args`.
fn parse(args: &[&str]) -> Parsed {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result =
        parse_to::<(), _, _, _, _>(&mut Sink::new(&mut out, &mut err), tool(), &GUIDE, args);
    let (code, output) = match result {
        Ok(((), output)) => (None, Some(output)),
        Err(stop) => (Some(ExitCode::from(stop)), None),
    };
    Parsed {
        code,
        output,
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

#[test]
fn fields_long_and_full_on_a_leaf_that_prints_no_result_are_refused_with_a_hint_to_drop_the_flag() {
    let admitted = ["tool", "serve"];
    assert_eq!(parse(&admitted).code, None, "{}", parse(&admitted).err);
    for (word, flag, does) in [
        (
            "--fields=id",
            "--fields",
            "--fields selects the columns of a table",
        ),
        (
            "--long",
            "--long",
            "--long adds the long columns of a table",
        ),
        ("--full", "--full", "--full prints every field of every row"),
    ] {
        let refused = ["tool", "serve", word];
        assert_one_word_apart(&refused, &admitted);

        let parsed = parse(&refused);

        assert_eq!(parsed.code, Some(ExitCode::from(2)), "{flag}");
        assert_eq!(
            parsed.err,
            format!(
                "error: {does}, and this command prints no result\n  help:  drop {flag}\n  code:  tool::cli::usage\n"
            )
        );
    }
}

#[test]
fn the_refusal_of_a_view_flag_on_a_leaf_that_prints_no_result_never_mentions_reading_fields() {
    for flag in ["--fields=id", "--long", "--full"] {
        let parsed = parse(&["tool", "serve", flag, "--json"]);

        assert_eq!(parsed.code, Some(ExitCode::from(2)), "{flag}");
        assert!(
            parsed.out.contains("this command prints no result"),
            "{}",
            parsed.out
        );
        assert!(!parsed.out.contains("read every field"), "{}", parsed.out);
    }
}

#[test]
fn json_on_a_leaf_that_prints_no_result_parses() {
    let parsed = parse(&["tool", "serve", "--json"]);

    assert_eq!(parsed.code, None, "{}", parsed.err);
    assert!(parsed.output.unwrap().is_json());
}

#[test]
fn the_help_of_a_leaf_that_prints_no_result_prints_the_output_line_and_no_view_flag() {
    let short = parse(&["tool", "serve", "-h"]);
    let long = parse(&["tool", "serve", "--help"]);

    let line = format!("\n{OUTPUT_LINE}\n");
    assert!(short.out.contains(&line), "{}", short.out);
    assert!(long.out.contains(&line), "{}", long.out);
    assert!(
        short.out.contains("Global flags: --json. "),
        "{}",
        short.out
    );
    for flag in ["--fields", "--long", "--full"] {
        assert!(!long.out.contains(&format!("  {flag}")), "{}", long.out);
    }
}

/// The failure that ends a server's run when the caller interrupts it.
#[derive(Debug)]
struct Interrupted;

/// The code of [`Interrupted`].
const INTERRUPTED: Code = Code::new("tool", "server", "interrupted");

impl fmt::Display for Interrupted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the server stopped because it was interrupted")
    }
}

impl std::error::Error for Interrupted {}

impl Failure for Interrupted {
    fn code(&self) -> &'static Code {
        &INTERRUPTED
    }

    fn recovery(&self) -> Recovery {
        Recovery::Retry
    }
}

#[test]
fn a_failure_of_a_leaf_that_prints_no_result_prints_a_failure_document_with_json() {
    let mut output = parse(&["tool", "serve", "--json"]).output.unwrap();
    let mut out = Vec::new();
    let mut err = Vec::new();

    output.print_failure(&mut Sink::new(&mut out, &mut err), &Interrupted);

    let out = String::from_utf8(out).unwrap();
    assert!(out.starts_with(r#"{"schema":"tool/failure/v1""#), "{out}");
    assert!(out.contains("tool::server::interrupted"), "{out}");
    assert_eq!(String::from_utf8(err).unwrap(), "");
    assert_eq!(output.failure_exit(&INTERRUPTED), 1);
}
