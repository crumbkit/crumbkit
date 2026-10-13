//! Tests that crumbkit's argument types add no description to the command
//! that flattens them or takes them as its arguments.

use clap::{Command, CommandFactory as _, Parser, Subcommand};

use crate::commands::CommandsList;
use crate::schemas::{SchemasList, SchemasShow};
use crate::{DryRun, Expect, NoInput, Output, Page, Yes};

// A root without a doc comment, which flattens every flag type of crumbkit.
#[derive(Parser)]
#[command(name = "tool")]
#[expect(
    dead_code,
    reason = "the test reads the command, never the parsed values"
)]
struct Bare {
    #[command(flatten)]
    output: Output,
    #[command(flatten)]
    page: Page,
    #[command(flatten)]
    no_input: NoInput,
    #[command(flatten)]
    dry_run: DryRun,
    #[command(flatten)]
    yes: Yes,
    #[command(flatten)]
    expect: Expect,
    #[command(subcommand)]
    command: Option<Leaves>,
}

// Subcommands without doc comments, which take crumbkit's argument types.
#[derive(Subcommand)]
#[expect(
    dead_code,
    reason = "the test reads the command, never the parsed values"
)]
enum Leaves {
    Commands(CommandsList),
    SchemasList(SchemasList),
    SchemasShow(SchemasShow),
}

/// The root that `Bare` describes with a doc comment.
#[derive(Parser)]
#[command(name = "tool")]
#[expect(
    dead_code,
    reason = "the test reads the command, never the parsed values"
)]
struct Described {
    #[command(flatten)]
    output: Output,
    #[command(flatten)]
    page: Page,
}

/// Returns the about and the long about of `command`, as text.
fn abouts(command: &Command) -> (Option<String>, Option<String>) {
    (
        command.get_about().map(ToString::to_string),
        command.get_long_about().map(ToString::to_string),
    )
}

#[test]
fn a_root_without_a_doc_comment_takes_no_description_from_the_flags_it_flattens() {
    assert_eq!(abouts(&Bare::command()), (None, None));
}

#[test]
fn a_subcommand_without_a_doc_comment_takes_no_description_from_crumbkits_arguments() {
    let command = Bare::command();

    for leaf in command.get_subcommands() {
        assert_eq!(abouts(leaf), (None, None), "{}", leaf.get_name());
    }
}

#[test]
fn a_root_with_a_doc_comment_keeps_it_beside_the_flags_it_flattens() {
    assert_eq!(
        abouts(&Described::command()),
        (
            Some("The root that `Bare` describes with a doc comment".to_owned()),
            None
        )
    );
}

#[cfg(feature = "usage-log")]
#[test]
fn a_subcommand_without_a_doc_comment_takes_no_description_from_the_usage_reports() {
    // The report commands, without doc comments.
    #[derive(Subcommand)]
    #[expect(
        dead_code,
        reason = "the test reads the command, never the parsed values"
    )]
    enum Reports {
        Usage(crate::usage::SelfUsage),
        Figures(crate::usage::SelfFigures),
    }
    let command = Reports::augment_subcommands(Command::new("self"));

    for leaf in command.get_subcommands() {
        assert_eq!(abouts(leaf), (None, None), "{}", leaf.get_name());
    }
}
