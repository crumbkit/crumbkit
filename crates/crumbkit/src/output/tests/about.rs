//! Tests that crumbkit's argument types add no description to the command
//! that flattens them or takes them as its arguments.
//!
//! `#[command(flatten)]` and a subcommand variant that holds a type both
//! call that type's `Args::augment_args` on the parent command, so the tests
//! call it on a command without a description.

use clap::{Args, Command, Parser};

use crate::commands::CommandsList;
use crate::schemas::{SchemasList, SchemasShow};
use crate::{DryRun, Expect, NoInput, Output, Page, Yes};

/// The root that `Described` describes with a doc comment.
#[derive(Parser)]
#[command(name = "tool")]
struct Described {
    #[command(flatten)]
    output: Output,
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
    // Each type sets the about last in its `augment_args`, so the root of
    // each type is separate, and a later type cannot clear an earlier one.
    let roots = [
        ("Output", Output::augment_args(Command::new("tool"))),
        ("Page", <Page>::augment_args(Command::new("tool"))),
        ("NoInput", NoInput::augment_args(Command::new("tool"))),
        ("DryRun", DryRun::augment_args(Command::new("tool"))),
        ("Yes", Yes::augment_args(Command::new("tool"))),
        ("Expect", Expect::augment_args(Command::new("tool"))),
    ];

    for (name, root) in &roots {
        assert_eq!(abouts(root), (None, None), "{name}");
    }
}

#[test]
fn a_subcommand_without_a_doc_comment_takes_no_description_from_crumbkits_arguments() {
    let leaves = [
        CommandsList::augment_args(Command::new("commands")),
        SchemasList::augment_args(Command::new("list")),
        SchemasShow::augment_args(Command::new("show")),
    ];

    for leaf in &leaves {
        assert_eq!(abouts(leaf), (None, None), "{}", leaf.get_name());
    }
}

#[test]
fn a_root_with_a_doc_comment_keeps_it_beside_the_flags_it_flattens() {
    let described = Described::try_parse_from(["tool", "--json"]);

    assert!(described.is_ok_and(|described| described.output.json));
    assert_eq!(
        abouts(&<Described as clap::CommandFactory>::command()),
        (
            Some("The root that `Described` describes with a doc comment".to_owned()),
            None
        )
    );
}

#[cfg(feature = "usage-log")]
#[test]
fn a_subcommand_without_a_doc_comment_takes_no_description_from_the_usage_reports() {
    let leaves = [
        crate::usage::SelfUsage::augment_args(Command::new("usage")),
        crate::usage::SelfFigures::augment_args(Command::new("figures")),
    ];

    for leaf in &leaves {
        assert_eq!(abouts(leaf), (None, None), "{}", leaf.get_name());
    }
}
