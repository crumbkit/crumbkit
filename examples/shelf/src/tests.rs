//! Tests for shelf as a whole.

use clap::CommandFactory as _;

use crate::SPELLING;
use crate::books::BooksError;
use crate::cli::{Cli, GUIDE};

#[test]
fn every_failure_code_spells_uniquely_as_printable_ascii() {
    crumbkit_test::check_codes(SPELLING, &[BooksError::CODES]);
}

#[test]
fn every_command_and_argument_has_one_name() {
    crumbkit_test::check_names(&Cli::command(), &[]);
}

#[test]
fn every_command_is_a_resource_followed_by_list_or_show() {
    crumbkit_test::check_grammar(&Cli::command(), &[], &[]);
}

#[test]
fn every_list_command_pages_and_every_show_command_takes_up_to_100_ids() {
    crumbkit_test::check_listings(&Cli::command(), &[]);
}

#[test]
fn every_option_with_many_values_accepts_repeats_and_commas() {
    crumbkit_test::check_filters(&Cli::command(), &[]);
}

#[test]
fn every_visible_command_is_under_a_goal_heading_and_every_topic_is_well_formed() {
    crumbkit_test::check_guide(&Cli::command(), &GUIDE);
}
