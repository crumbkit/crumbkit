//! Tests for finding a batch's keys among the caller's words.

use std::ffi::OsString;

use clap::{CommandFactory as _, Parser, Subcommand};
use schemars::JsonSchema;
use serde::Serialize;

use super::Rerun;
use crate::{Batch, Error, RenderCall as _, Row};

/// A row of the test batches, which hold none.
#[derive(Serialize, JsonSchema)]
struct Planet {
    name: String,
}

impl Row for Planet {
    const COLUMNS: &'static [&'static str] = &["name"];
}

/// A tool whose `show` takes its keys as a positional argument beside an
/// option, and whose `find` takes them after a flag.
#[derive(Parser)]
#[command(name = "tool")]
struct Tool {
    #[command(subcommand)]
    command: Look,
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Look {
    Show {
        #[arg(required = true, value_name = "NAME")]
        names: Vec<String>,
        #[arg(long)]
        tag: Option<String>,
    },
    Find {
        #[arg(long = "name", short = 'n', required = true, num_args = 1.., value_name = "NAME")]
        names: Vec<String>,
    },
}

/// Returns the split of `words` for a batch of the command at `path` whose
/// keys are `keys`, none of which it fetched.
fn find(path: &'static [&'static str], keys: &[&str], words: &[&str]) -> Option<Rerun> {
    let batch: Batch<Planet, Error> = Batch::new(path, "names", keys);
    let words: Vec<OsString> = words.iter().map(OsString::from).collect();
    Rerun::find(&Tool::command(), &words, &batch)
}

/// Returns `rerun`'s call of every key as text.
fn all(rerun: &Rerun) -> String {
    rerun.all().unwrap().render(&mut Tool::command()).unwrap()
}

#[test]
fn flags_joined_to_their_keys_by_an_equals_sign_rerun_as_one_flag_before_the_keys() {
    let rerun = find(
        &["find"],
        &["a", "b"],
        &["find", "--name=a", "--name=b", "--json"],
    )
    .unwrap();

    assert_eq!(all(&rerun), "tool find --name <NAME>... --json");
}

#[test]
fn short_flags_with_their_keys_attached_rerun_as_the_long_flag() {
    let rerun = find(&["find"], &["a", "b"], &["--json", "find", "-na", "-nb"]).unwrap();

    assert_eq!(all(&rerun), "tool --json find --name <NAME>...");
}

#[test]
fn a_key_that_starts_with_a_hyphen_after_a_flag_leaves_no_call_that_fetches_every_key() {
    let rerun = find(&["find"], &["a", "-b"], &["find", "--name", "a", "--json"]);

    assert!(rerun.is_none(), "{rerun:?}");
}

#[test]
fn keys_after_a_double_dash_rerun_after_it_so_a_key_with_a_hyphen_stays_a_value() {
    let rerun = find(
        &["show"],
        &["a", "-b"],
        &["show", "--json", "--", "a", "-b"],
    )
    .unwrap();

    assert_eq!(all(&rerun), "tool show --json -- <NAME>...");
}

#[test]
fn keys_typed_apart_rerun_together_at_the_place_of_the_first() {
    let rerun = find(&["show"], &["a", "b"], &["show", "a", "--tag", "x", "b"]).unwrap();

    assert_eq!(all(&rerun), "tool show <NAME>... --tag x");
}

#[test]
fn a_key_that_the_scan_takes_for_another_option_s_value_leaves_the_words_unsplit() {
    let rerun = find(&["show"], &["a", "b"], &["show", "--tag", "a", "a", "b"]);

    assert!(rerun.is_none(), "{rerun:?}");
}

#[test]
fn words_that_name_another_command_leave_the_words_unsplit() {
    let rerun = find(&["show"], &["a", "b"], &["find", "--name", "a", "b"]);

    assert!(rerun.is_none(), "{rerun:?}");
}

#[test]
fn no_words_leave_nothing_to_split() {
    assert!(find(&["show"], &["a"], &[]).is_none());
}
