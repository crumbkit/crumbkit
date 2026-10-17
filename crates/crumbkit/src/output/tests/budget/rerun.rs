//! Tests for the recovery of a batch over the output budget, which reruns
//! the caller's own words with the keys left out in place of the keys the
//! caller typed.

use clap::{CommandFactory as _, FromArgMatches as _, Parser, Subcommand};
use serde_json::{Value, json};

use super::{BUDGET, STAR_BATCH, Star, parsed, star, stars};
use crate::help::Guide;
use crate::output::tests::{Finished, finish_output};
use crate::{Batch, Document, Error, Output, ViewFlags};

/// A tool with a global option before its subcommands, one subcommand that
/// takes its keys as a positional argument and one that takes them after a
/// flag.
#[derive(Parser)]
#[command(name = "tool")]
struct Sky {
    #[command(subcommand)]
    command: Look,
    #[command(flatten)]
    view: ViewFlags,
    /// Run as if the tool started in this directory.
    #[arg(short = 'C', global = true, value_name = "DIR")]
    dir: Option<String>,
}

#[derive(Subcommand)]
enum Look {
    /// Show the stars with these names.
    Show {
        #[arg(required = true, value_name = "NAME")]
        names: Vec<String>,
        /// Show only the bright stars.
        #[arg(long)]
        bright: bool,
    },
    /// Find the stars with these names.
    Find {
        #[arg(long = "name", required = true, num_args = 1.., value_name = "NAME")]
        names: Vec<String>,
    },
}

/// Returns the names of the first 20 stars.
fn keys() -> Vec<String> {
    (1..=20).map(|number| star(number).name).collect()
}

/// Returns a batch of the command at `path` with the first 20 stars as its
/// keys and its rows.
fn batch(path: &'static [&'static str]) -> Batch<Star, Error> {
    let mut batch = Batch::new(path, "names", &keys());
    batch.extend(stars(20).into_iter().map(Ok));
    batch
}

/// Returns `before`, the first 20 star names, then `after`, as the words
/// of a command line from the program name on.
fn line(before: &[&str], after: &[&str]) -> Vec<String> {
    let keys = keys();
    std::iter::once("tool")
        .chain(before.iter().copied())
        .chain(keys.iter().map(String::as_str))
        .chain(after.iter().copied())
        .map(str::to_owned)
        .collect()
}

/// Parses `args` against [`Sky`] with clap alone and returns the output that
/// a tool's `main` builds from them with a budget of [`BUDGET`].
fn output(args: &[String]) -> Output {
    let mut command = Sky::command();
    let matches = command.try_get_matches_from_mut(args).unwrap();
    let view = ViewFlags::from_arg_matches(&matches).unwrap();
    Output::new(view, command, &guide(), args)
}

/// Returns a guide with the budget of [`BUDGET`].
fn guide() -> Guide {
    Guide::new().output_budget(BUDGET, "the tests print little")
}

/// Finishes the batch of the command at `path` with the command line
/// `args`.
fn finish(
    args: &[String],
    document: Document<Batch<Star, Error>>,
    path: &'static [&'static str],
) -> Finished {
    finish_output(&mut output(args), document, batch(path))
}

/// Returns the recovery of the last failure of a batch's JSON document.
fn last_recovery(finished: &Finished) -> Value {
    let document = parsed(&finished.out);
    let failures = document["failures"].as_array().unwrap();
    failures.last().unwrap()["recovery"].clone()
}

#[test]
fn a_batch_s_recovery_keeps_json_whether_the_caller_typed_it_before_or_after_the_keys() {
    let before = finish(&line(&["show", "--json"], &[]), STAR_BATCH, &["show"]);
    let after = finish(&line(&["show"], &["--json"]), STAR_BATCH, &["show"]);

    assert_eq!(
        last_recovery(&before),
        json!({"kind": "run", "call": "tool show --json <NAME>...", "all": "left_out"})
    );
    assert_eq!(
        last_recovery(&after),
        json!({"kind": "run", "call": "tool show <NAME>... --json", "all": "left_out"})
    );
}

#[test]
fn a_batch_s_recovery_keeps_a_global_option_that_the_caller_typed_before_the_subcommand() {
    let args = line(&["-C", "/srv/sky", "show"], &["--json"]);

    let finished = finish(&args, STAR_BATCH, &["show"]);

    assert_eq!(
        last_recovery(&finished),
        json!({"kind": "run", "call": "tool -C /srv/sky show <NAME>... --json", "all": "left_out"})
    );
}

#[test]
fn a_batch_s_recovery_keeps_the_fields_that_the_caller_selected() {
    let args = line(&["show"], &["--json", "--fields", "story"]);

    let finished = finish(&args, STAR_BATCH, &["show"]);

    assert_eq!(
        last_recovery(&finished),
        json!({"kind": "run", "call": "tool show <NAME>... --json --fields story", "all": "left_out"})
    );
}

#[test]
fn a_batch_s_recovery_in_text_runs_the_caller_s_words_with_the_keys_left_out_in_place() {
    let args = line(&["-C", "/srv/my sky", "show", "--bright"], &[]);

    let finished = finish(&args, STAR_BATCH, &["show"]);

    let shown = finished.out.lines().count() - 1;
    let left_out: Vec<String> = keys().split_off(shown);
    assert!(
        finished.err.contains(&format!(
            "help:  run `tool -C '/srv/my sky' show --bright {}`\n",
            left_out.join(" ")
        )),
        "{}",
        finished.err
    );
}

#[test]
fn a_batch_whose_keys_follow_a_flag_keeps_the_flag_in_front_of_the_keys_left_out() {
    let args = line(&["find", "--name"], &["--json"]);

    let finished = finish(&args, STAR_BATCH, &["find"]);

    assert_eq!(
        last_recovery(&finished),
        json!({"kind": "run", "call": "tool find --name <NAME>... --json", "all": "left_out"})
    );
}

#[test]
fn a_batch_s_recovery_without_the_caller_s_words_runs_the_batch_s_command_alone() {
    let args = line(&["-C", "/srv/sky", "show"], &["--json"]);
    let mut command = Sky::command();
    let matches = command.try_get_matches_from_mut(&args).unwrap();
    let view = ViewFlags::from_arg_matches(&matches).unwrap();
    let mut output = Output::new(view, command, &guide(), ["tool"]);

    let finished = finish_output(&mut output, STAR_BATCH, batch(&["show"]));

    assert_eq!(
        last_recovery(&finished),
        json!({"kind": "run", "call": "tool show <NAME>...", "all": "left_out"})
    );
}

#[test]
fn a_batch_s_recovery_runs_the_batch_s_command_alone_when_the_caller_s_words_name_another_command()
{
    let args = line(&["find", "--name"], &["--json"]);

    let finished = finish(&args, STAR_BATCH, &["show"]);

    assert_eq!(
        last_recovery(&finished),
        json!({"kind": "run", "call": "tool show <NAME>...", "all": "left_out"})
    );
}
