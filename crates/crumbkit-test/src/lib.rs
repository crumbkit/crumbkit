// The crate docs are the README, so its example runs as a doctest. The README
// links to files in the repository, which work on GitHub. The definitions below
// come first, so rustdoc uses them for the same labels (ADR-0013).
//! [`check_codes`]: crate::check_codes
//! [`Allow`]: crate::Allow
//! [`check_opt_outs`]: crate::check_opt_outs
//! [`check_names`]: crate::check_names
//! [`check_grammar`]: crate::check_grammar
//! [`check_effects`]: crate::check_effects
//! [`check_listings`]: crate::check_listings
//! [`check_filters`]: crate::check_filters
//! [`check_guide`]: crate::check_guide
//! [`check_exit_codes`]: crate::check_exit_codes
//! [`check_help_sizes`]: crate::check_help_sizes
//! [`Budgets`]: crate::Budgets
//! [`Guide`]: crate::Guide
//! [`Pages::clap`]: crate::Pages::clap
//! [`Pages::rendered`]: crate::Pages::rendered
//! [`Pages::binary`]: crate::Pages::binary
//! [`PathBudgets::help_only`]: crate::PathBudgets::help_only
//! [`Budget`]: crate::Budget
//! [`check_path_weights`]: crate::check_path_weights
//! [`PathBudgets`]: crate::PathBudgets
//! [`path_totals`]: crate::path_totals
//! [`output_weight`]: crate::output_weight
//! [`check_help_goldens`]: crate::check_help_goldens
//! [`check_schema_goldens`]: crate::check_schema_goldens
//! [`check_document_ids`]: crate::check_document_ids
//! [`check_mounts`]: crate::check_mounts
//! [`check_examples`]: crate::check_examples
//! [`check_output_budget`]: crate::check_output_budget
//! [`Examples`]: crate::Examples
//! [`FailureReader`]: crate::FailureReader
//! [`example_golden_name`]: crate::example_golden_name
//! [`check_command_reference`]: crate::check_command_reference
//! [`check_recoveries`]: crate::check_recoveries
//! [`Recoveries`]: crate::Recoveries
//! [`check_twin`]: crate::check_twin
//! [ADR-0025]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/0025-help-checks.md
//! [output budget docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/output/budget/README.md
//! [command reference docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/help/reference.md
//! [checks docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/checks/README.md
//! [opt-outs docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/adoption/opt-outs.md
//! [schema commands docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/schemas/commands.md
//! [golden files docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/checks/golden-files.md
#![doc = include_str!("../README.md")]
// docs.rs builds the docs on nightly with `--cfg docsrs`, and rustdoc's
// `doc_cfg` feature labels each item behind a cargo feature with its name.
#![cfg_attr(docsrs, feature(doc_cfg))]

mod allow;
mod budget;
mod codes;
mod documents;
mod effects;
mod examples;
mod exits;
mod failures;
mod filters;
mod goldens;
mod grammar;
mod guide;
mod listings;
mod names;
mod opt_outs;
mod output_budget;
mod pages;
mod recoveries;
mod reference;
mod runs;
mod schemas;
mod sizes;
mod temp;
#[cfg(test)]
mod testing;
mod tree;
mod twins;
mod weights;

pub use crate::allow::{Allow, Rule};
pub use crate::budget::Budget;
pub use crate::codes::check_codes;
pub use crate::documents::{check_document_ids, check_mounts};
pub use crate::effects::check_effects;
pub use crate::examples::{Examples, check_examples, example_golden_name};
pub use crate::exits::check_exit_codes;
pub use crate::failures::FailureReader;
pub use crate::filters::check_filters;
pub use crate::goldens::check_help_goldens;
pub use crate::grammar::check_grammar;
pub use crate::guide::check_guide;
pub use crate::listings::check_listings;
pub use crate::names::check_names;
pub use crate::opt_outs::check_opt_outs;
pub use crate::output_budget::check_output_budget;
pub use crate::pages::Pages;
pub use crate::recoveries::{
    Failing, Recoveries, RuntimeFailing, RuntimePlace, Unprovoked, check_recoveries,
};
pub use crate::reference::check_command_reference;
pub use crate::schemas::check_schema_goldens;
pub use crate::sizes::{Budgets, check_help_sizes};
pub use crate::twins::{Twin, Twins, check_twin, check_twins};
pub use crate::weights::{
    PathBudgets, UnreadableGolden, check_path_weights, output_weight, path_totals,
};
/// crumbkit's guide, which several checks and `Pages::rendered` and
/// `Pages::binary` take, so a test can name it with crumbkit-test alone.
pub use crumbkit::help::Guide;
