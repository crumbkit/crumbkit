//! The steps of a guide topic and the text that `help` prints for them.

use std::fmt::Write as _;

use crate::help::quote;

/// One step of a [guide](super::Topic::guide): a one-line explanation and
/// the commands that carry it out, each as the words after the tool's name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    /// The one-line explanation that help prints after the step's number.
    about: &'static str,
    /// The commands of the step, each one argument for each word.
    commands: &'static [&'static [&'static str]],
}

impl Step {
    /// The most commands that a step has. `check_help` fails a step with
    /// more, and the tool splits it into two steps.
    pub const MAX_COMMANDS: usize = 3;

    /// Returns the step that `about` explains in one line and that
    /// `commands` carry out, each command the words after the tool's name,
    /// such as `&["create", "--title", "x"]`, as [`Example::new`] holds the
    /// words after a leaf's path. A step has between one and
    /// [`Step::MAX_COMMANDS`] commands.
    ///
    /// [`Example::new`]: crate::help::Example::new
    #[must_use]
    pub const fn new(about: &'static str, commands: &'static [&'static [&'static str]]) -> Self {
        Self { about, commands }
    }

    /// Returns the one-line explanation of the step.
    #[must_use]
    pub const fn about(&self) -> &'static str {
        self.about
    }

    /// Returns the commands of the step, each the words after the tool's
    /// name.
    #[must_use]
    pub const fn commands(&self) -> &'static [&'static [&'static str]] {
        self.commands
    }
}

/// Returns the guide of `steps` as help prints it: for each step, its number
/// and explanation on one line and each command on its own line below,
/// indented under the explanation and led by `$`. The numbers are as wide as
/// the last one, so the commands line up.
pub(super) fn render(tool_name: &str, steps: &[Step]) -> String {
    let width = steps.len().to_string().len();
    let mut text = String::new();
    for (index, step) in steps.iter().enumerate() {
        let _ = writeln!(text, "{:>width$}. {}", index + 1, step.about);
        for command in step.commands {
            let _ = write!(text, "{:indent$}$ {tool_name}", "", indent = width + 2);
            for word in *command {
                text.push(' ');
                text.push_str(&quote(word));
            }
            text.push('\n');
        }
    }
    text
}
