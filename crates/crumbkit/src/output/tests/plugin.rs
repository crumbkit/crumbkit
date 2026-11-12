//! Tests for a failure whose code the tool learns only at run time, such as
//! the code that a plugin answers with. The tool reads the plugin's code
//! back into its segments with crumbkit's spelling, makes the code with
//! `Code::runtime`, and `finish` prints it as it prints a declared code.

use std::fmt;

use clap::{Parser, Subcommand};

use super::{PLANETS, Planet};
use crate::{
    Code, CodeBuf, Failure, Recovery, Sink, Spelling, Tool, ViewFlags, finish_to, parse_to,
};

/// The tool's own code for a plugin answer whose code the tool cannot read.
const PLUGIN_FAILED: Code<'static> = Code::new("tool", "plugin", "failed");

/// The tool record.
const TOOL: Tool = Tool::new();

/// The failure that a plugin answered with.
#[derive(Debug)]
struct PluginFailure {
    /// The plugin's code, or the tool's own code when the plugin's code
    /// cannot be read, which the failure owns.
    code: CodeBuf,
    /// The plugin's message.
    message: String,
}

impl PluginFailure {
    /// Returns the failure of a plugin's answer, whose code is the text
    /// `code` in crumbkit's spelling and whose message is `message`. The
    /// failure has the plugin's code when the spelling splits the text into
    /// three segments that [`Code::runtime`] accepts, and [`PLUGIN_FAILED`]
    /// otherwise.
    fn from_answer(code: &str, message: &str) -> Self {
        let code = Spelling::COLONS
            .split(code)
            .and_then(|(component, area, reason)| Code::runtime(component, area, reason).ok())
            .unwrap_or(PLUGIN_FAILED)
            .to_buf();
        Self {
            code,
            message: message.to_owned(),
        }
    }
}

impl fmt::Display for PluginFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for PluginFailure {}

impl Failure for PluginFailure {
    fn code(&self) -> Code<'_> {
        self.code.as_code()
    }

    fn recovery(&self) -> Recovery {
        Recovery::None {
            reason: "the plugin gives no next step".to_owned(),
        }
    }
}

/// A tool with one `list` command and the view flags.
#[derive(Parser)]
#[command(name = "tool")]
struct Cli {
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    view: ViewFlags,
}

#[derive(Subcommand)]
enum Command {
    List,
}

/// Parses `args` with [`TOOL`] into memory, finishes `failure` with the
/// output, and returns the exit code and the standard output.
fn finish(args: &[&str], failure: PluginFailure) -> (u8, String) {
    let mut sink = Sink::new(Vec::new(), Vec::new());
    let Ok((_, output)) = parse_to::<Cli>(&mut sink, &TOOL, args) else {
        panic!("parse refused {args:?}");
    };
    let mut out = Vec::new();
    let result: Result<Vec<Planet>, PluginFailure> = Err(failure);
    let code = finish_to(
        &mut Sink::new(&mut out, Vec::new()),
        output,
        PLANETS,
        result,
    );
    (code, String::from_utf8(out).unwrap())
}

#[test]
fn a_code_that_a_plugin_answers_with_prints_as_the_failure_s_code_and_exits_with_1() {
    let failure = PluginFailure::from_answer("lending::loans::overdue", "the loan is late");

    let (code, out) = finish(&["tool", "list", "--json"], failure);

    assert_eq!(code, 1);
    assert!(out.contains(r#""code":"lending::loans::overdue""#), "{out}");
}

#[test]
fn a_plugin_code_that_the_spelling_cannot_split_or_that_breaks_the_segment_rule_gives_the_tool_s_own_code()
 {
    for answer in ["lending::overdue", "lending::Loans::overdue"] {
        let failure = PluginFailure::from_answer(answer, "the loan is late");

        assert_eq!(failure.code(), PLUGIN_FAILED, "{answer}");
    }
}
