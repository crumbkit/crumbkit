//! The steps of a guide topic and the text that `help` prints for them.

use std::fmt::Write as _;

use crate::help::quote;

/// One step of a [guide](super::Topic::guide): a one-line explanation and
/// the commands that carry it out, each as the words after the tool's name,
/// or a pointer to a topic of another tool that carries it out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    /// The one-line explanation that help prints after the step's number.
    about: &'static str,
    /// The commands of the step, each one argument for each word.
    commands: &'static [&'static [&'static str]],
    /// The tool and topic of a step that another tool carries out, or `None`.
    pointer: Option<(&'static str, &'static str)>,
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
        Self {
            about,
            commands,
            pointer: None,
        }
    }

    /// Returns the step that `about` explains in one line and that the
    /// topic `topic` of the tool `tool` carries out. `<tool> help <guide>`
    /// prints the line `$ <tool> help <topic>` under the explanation, so a
    /// reader opens the other tool's guide from its installed binary. The
    /// step has no commands, and the limit of [`Step::MAX_COMMANDS`] does
    /// not apply to it.
    ///
    /// `check_help` checks the form of both names and that `tool` is not
    /// the tool's own name. It does not run or parse the other tool's
    /// commands, because it holds no other tool's command tree, so the other
    /// tool's own `check_help` keeps that topic's commands true.
    #[must_use]
    pub const fn see(about: &'static str, tool: &'static str, topic: &'static str) -> Self {
        Self {
            about,
            commands: &[],
            pointer: Some((tool, topic)),
        }
    }

    /// Returns the one-line explanation of the step.
    #[must_use]
    pub const fn about(&self) -> &'static str {
        self.about
    }

    /// Returns the tool and the topic that a [`Step::see`] step points to, or
    /// `None` for a step of commands.
    #[must_use]
    pub const fn pointer(&self) -> Option<(&'static str, &'static str)> {
        self.pointer
    }

    /// Returns the commands of the step, each the words after the tool's
    /// name. A step made by [`Step::see`] has none.
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
        if let Some((tool, topic)) = step.pointer {
            let _ = writeln!(
                text,
                "{:indent$}$ {tool} help {topic}",
                "",
                indent = width + 2
            );
        }
    }
    text
}
