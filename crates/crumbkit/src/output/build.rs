//! The builder of `Output`, for a tool that defines the view flags in its
//! own parser in place of flattening `Output`.

use super::{JsonStyle, Output};
use crate::help::rename::Renames;

impl Output {
    /// Returns the output flags with every flag off and no command, the
    /// same value as `Output::default()`.
    ///
    /// A tool that defines `--json`, `--fields`, `--long` or `--full` in its
    /// own clap parser cannot also flatten `Output`, because clap refuses
    /// two arguments with one name. Such a tool builds `Output` from the
    /// values it parsed and then calls [`Output::set_command`]:
    ///
    /// ```
    /// use crumbkit::Output;
    ///
    /// let output = Output::new()
    ///     .json(true)
    ///     .fields(vec!["id".to_owned(), "title".to_owned()]);
    /// assert!(output.is_json());
    /// ```
    ///
    /// `finish` and `stream` check a built `Output` as they check a parsed
    /// one: they refuse `--full` without `--json`, `--fields` with `--long`
    /// or `--full`, a view flag that the result cannot use and an unknown
    /// `--fields` name, after the handler has run. `parse` refuses these
    /// before the handler runs only for arguments whose ids are `json`,
    /// `fields`, `long` and `full` and that are global, and it hides an
    /// unoffered view flag from a leaf's help by the same ids. A tool that
    /// owns the flags therefore gives its arguments those ids and marks them
    /// global.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            json: false,
            fields: Vec::new(),
            long: false,
            full: false,
            command: None,
            words: None,
            dry_run: false,
            no_input: false,
            failure_form: None,
            no_terminal: false,
            json_style: JsonStyle {
                layout: None,
                keys: None,
            },
            budget: None,
            spent: 0,
            exits: &[],
            renames: Renames::NONE,
            field_names: &[],
        }
    }

    /// Returns the flags with `--json` set to `on`.
    #[must_use]
    pub const fn json(mut self, on: bool) -> Self {
        self.json = on;
        self
    }

    /// Returns the flags with `--long` set to `on`.
    #[must_use]
    pub const fn long(mut self, on: bool) -> Self {
        self.long = on;
        self
    }

    /// Returns the flags with `--full` set to `on`.
    #[must_use]
    pub const fn full(mut self, on: bool) -> Self {
        self.full = on;
        self
    }

    /// Returns the flags with `--fields` naming `names`, in that order. An
    /// empty list means that the caller passed no `--fields`. It is not a
    /// `const fn`, because a `const fn` cannot drop the list it replaces.
    #[must_use]
    pub fn fields(mut self, names: Vec<String>) -> Self {
        self.fields = names;
        self
    }
}
