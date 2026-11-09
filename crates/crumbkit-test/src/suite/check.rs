//! The checks of the convention suite and the macro that expands to one test
//! for each of them.

/// Declares [`Check`] and `conventions!` from one table, so a check added to
/// the table reaches both, and every tool that uses the macro runs it with no
/// edit. `$d` carries a literal `$` into the macro that this one writes,
/// because a `macro_rules` body cannot spell one on stable Rust.
macro_rules! checks {
    ($d:tt $($(#[$doc:meta])* $check:ident => $test:ident,)*) => {
        /// One check of the convention suite, which
        /// [`Suite::skip`](crate::Suite::skip) names to leave it out.
        ///
        /// Each variant runs the `check_*` function that it names.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[non_exhaustive]
        pub enum Check {
            $($(#[$doc])* $check,)*
        }

        impl Check {
            /// Every check, in the order that the enum declares them.
            pub(crate) const ALL: &'static [Self] = &[$(Self::$check,)*];
        }

        /// Expands a [`Suite`](crate::Suite) into one `#[test]` for each
        /// [`Check`], named as a sentence, so each check keeps its name in
        /// test output and in `cargo test` filters.
        ///
        /// A tool calls it once, in a file of its own in an integration test
        /// of its binary's package. The tests land in the calling module, so
        /// a tool's own test in that module could take the name of a check
        /// that crumbkit-test adds later.
        ///
        /// When crumbkit-test adds a check, the macro runs it with no edit to
        /// the tool. A check that needs a new input fails and names that
        /// input until the tool sets it or skips the check.
        ///
        /// ```no_run
        /// use clap::Command;
        /// use crumbkit::Tool;
        /// use crumbkit_test::Suite;
        ///
        /// const SUITE: Suite<'_> = Suite::new(|| (Command::new("shelf"), Tool::new()));
        ///
        /// crumbkit_test::conventions!(SUITE);
        /// # fn main() {}
        /// ```
        #[macro_export]
        macro_rules! conventions {
            ($d suite:expr $d(,)?) => {
                $(
                    #[::core::prelude::v1::test]
                    fn $test() {
                        $crate::Suite::run(&$d suite, $crate::Check::$check);
                    }
                )*
            };
        }
    };
}

checks! { $
    /// [`check_names`](crate::check_names).
    Names => every_command_and_argument_has_one_name,
    /// [`check_grammar`](crate::check_grammar).
    Grammar => every_command_is_a_resource_followed_by_a_verb,
    /// [`check_effects`](crate::check_effects).
    Effects => every_command_declares_its_effects,
    /// [`check_listings`](crate::check_listings).
    Listings => every_list_command_pages_and_every_show_command_takes_up_to_100_ids,
    /// [`check_filters`](crate::check_filters).
    Filters => every_option_with_many_values_accepts_repeats_and_commas,
    /// [`check_help`](crate::check_help).
    Help => every_command_leaf_and_topic_meets_the_help_rules,
    /// [`check_exit_codes`](crate::check_exit_codes).
    ExitCodes => every_exit_code_meets_the_exit_code_rules,
    /// [`check_codes`](crate::check_codes).
    Codes => every_failure_code_spells_uniquely_as_printable_ascii,
    /// [`check_help_sizes`](crate::check_help_sizes).
    HelpSizes => every_help_page_fits_its_size_budget,
    /// [`check_path_weights`](crate::check_path_weights).
    PathWeights => every_path_through_the_help_fits_its_weight_budget,
    /// [`check_help_goldens`](crate::check_help_goldens).
    HelpGoldens => every_help_page_matches_its_golden_file,
    /// [`check_schema_goldens`](crate::check_schema_goldens).
    SchemaGoldens => every_document_schema_matches_its_golden_file,
    /// [`check_document_ids`](crate::check_document_ids).
    DocumentIds => every_document_is_named_after_the_path_of_the_command_that_prints_it,
    /// [`check_mounts`](crate::check_mounts).
    Mounts => every_crumbkit_command_sits_at_its_fixed_path,
    /// [`check_examples`](crate::check_examples).
    Examples => every_example_in_the_tool_runs_and_prints_its_golden_output,
    /// [`check_output_budget`](crate::check_output_budget).
    OutputBudget => every_json_example_cuts_or_refuses_a_result_past_its_output_budget,
    /// [`check_command_reference`](crate::check_command_reference).
    CommandReference => every_command_matches_the_checked_in_command_reference,
    /// [`check_opt_outs`](crate::check_opt_outs).
    OptOuts => every_opt_out_matches_the_opt_out_report,
    /// [`check_recoveries`](crate::check_recoveries).
    Recoveries => every_recovery_leads_to_a_command_that_runs,
    /// [`check_twins`](crate::check_twins).
    Twins => every_declared_code_has_a_twin_that_the_tool_accepts,
}
