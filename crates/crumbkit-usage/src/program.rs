//! The name of the program that started a run, which the program gives in
//! the tool's `<TOOL>_PROGRAM` variable, such as `SHELF_PROGRAM=flightdeck`
//! (ADR-0195).

use schemars::JsonSchema;
use serde::Serialize;

#[cfg(test)]
mod tests;

/// The longest program name, in bytes, which are ASCII characters.
const LONGEST: usize = 32;

/// The rule of a program name, which the parser of `--exclude-program` and
/// the warning about a refused `<TOOL>_PROGRAM` print.
pub(super) const RULE: &str = "a program name is 1 to 32 lowercase letters, digits, `-` and `_`, starting with a letter, such as flightdeck";

/// The pattern that a program name matches, which the usage record's schema
/// states.
pub(super) const PATTERN: &str = "^[a-z][a-z0-9_-]{0,31}$";

/// The name of a program that marked a run, such as `flightdeck`: 1 to 32
/// lowercase ASCII letters, digits, `-` and `_`, starting with a letter.
///
/// The program chooses the name for itself, so the name never holds a value
/// that a caller typed as an argument, and the pattern keeps any other text
/// out of the usage log and the reports (ADR-0029).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(transparent)]
#[schemars(extend("pattern" = PATTERN))]
pub(super) struct Program(String);

impl Program {
    /// Returns the program that `text` names, or `None` when `text` is not
    /// a program name.
    pub(super) fn new(text: &str) -> Option<Self> {
        is_program(text).then(|| Self(text.to_owned()))
    }

    /// Returns the name as text.
    pub(super) fn as_str(&self) -> &str {
        &self.0
    }
}

/// Returns whether `text` is a program name: 1 to 32 lowercase ASCII
/// letters, digits, `-` and `_`, starting with a letter.
pub(super) fn is_program(text: &str) -> bool {
    text.len() <= LONGEST
        && text.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && text
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"-_".contains(&byte))
}

/// Returns the program that `text` names, for clap's parser of
/// `--exclude-program`.
///
/// # Errors
///
/// Returns why `text` is refused when it is not a program name.
pub(super) fn parse(text: &str) -> Result<Program, String> {
    Program::new(text).ok_or_else(|| RULE.to_owned())
}
