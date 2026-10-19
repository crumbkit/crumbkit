//! `Menu`, a picker that shows a numbered list and reads numbers or text.

use std::io::{self, BufRead, Write};

use super::{Answer, Choice, Picker, Question};

#[cfg(test)]
mod tests;

/// The number of choices that [`Menu::new`] shows at once.
const ROWS: usize = 20;

/// A picker that shows the choices as a numbered list and reads one line at
/// a time.
///
/// The person types a number to pick the choice shown with it, or several
/// numbers separated by spaces when the argument takes several values. Any
/// other text narrows the list to the choices whose label or value contains
/// it, ignoring case, and the numbers then count within the narrowed list.
/// An empty line, or the end of input, cancels, and `crumbkit::parse` then
/// refuses the command line as it would without a picker. A number that the
/// list does not show, or a count of numbers that the argument does not
/// take, prints why and asks again.
///
/// The menu shows at most [`Menu::rows`] choices, 20 unless the tool sets
/// another number, and then a line that says how many more match. Each
/// control character in a label shows as its escape, such as `\n`, so every
/// choice stays on one line and a label cannot move the cursor.
///
/// A tool keeps one menu in a `static` and sets it on its guide:
///
/// ```
/// use crumbkit::help::Guide;
/// use crumbkit::pick::Menu;
///
/// static MENU: Menu = Menu::new().rows(10);
/// const GUIDE: Guide = Guide::new().picker(&MENU);
/// assert!(GUIDE.get_picker().is_some());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Menu {
    /// The most choices that the menu shows at once.
    rows: usize,
}

impl Menu {
    /// Returns a menu that shows at most 20 choices at once.
    #[must_use]
    pub const fn new() -> Self {
        Self { rows: ROWS }
    }

    /// Sets the most choices that the menu shows at once. A menu set to 0
    /// shows 1.
    #[must_use]
    pub const fn rows(mut self, rows: usize) -> Self {
        self.rows = if rows == 0 { 1 } else { rows };
        self
    }
}

/// Returns [`Menu::new`].
impl Default for Menu {
    fn default() -> Self {
        Self::new()
    }
}

impl Picker for Menu {
    fn pick(
        &self,
        question: &Question<'_>,
        input: &mut dyn BufRead,
        output: &mut dyn Write,
    ) -> io::Result<Answer> {
        let choices = question.choices();
        let mut filter = String::new();
        loop {
            let mut matches = matching(choices, &filter);
            if matches.is_empty() {
                writeln!(output, "No choice matches `{}`.", escape(&filter))?;
                filter.clear();
                matches = matching(choices, &filter);
            }
            let shown = matches.len().min(self.rows);
            show(output, question, &matches[..shown], matches.len() - shown)?;
            let mut line = String::new();
            if input.read_line(&mut line)? == 0 {
                return Ok(Answer::Cancelled);
            }
            let line = line.trim();
            if line.is_empty() {
                return Ok(Answer::Cancelled);
            }
            let numbers: Option<Vec<usize>> = line
                .split_whitespace()
                .map(|word| word.parse().ok())
                .collect();
            let Some(numbers) = numbers else {
                line.clone_into(&mut filter);
                continue;
            };
            if numbers.len() < question.least() || numbers.len() > question.most() {
                writeln!(output, "{}", count(question.least(), question.most()))?;
                continue;
            }
            let picked: Option<Vec<usize>> = numbers
                .iter()
                .map(|&number| {
                    let index = number.checked_sub(1)?;
                    matches[..shown].get(index).copied()
                })
                .collect();
            match picked {
                Some(picked) => return Ok(Answer::Picked(picked)),
                None => writeln!(output, "Pick numbers from 1 to {shown}.")?,
            }
        }
    }
}

/// Returns the positions of the choices whose label or value contains
/// `filter`, ignoring case. An empty filter matches every choice.
fn matching(choices: &[Choice], filter: &str) -> Vec<usize> {
    let filter = filter.to_lowercase();
    choices
        .iter()
        .enumerate()
        .filter(|(_, choice)| {
            choice.label().to_lowercase().contains(&filter)
                || choice.value().to_lowercase().contains(&filter)
        })
        .map(|(position, _)| position)
        .collect()
}

/// Writes the question, the numbered `shown` choices, a line about the
/// `hidden` matches when there are any, and the prompt.
fn show(
    output: &mut dyn Write,
    question: &Question<'_>,
    shown: &[usize],
    hidden: usize,
) -> io::Result<()> {
    write!(output, "{} needs {}", question.command(), question.name())?;
    match question.get_help() {
        Some(help) => writeln!(output, ": {}", escape(help))?,
        None => writeln!(output)?,
    }
    let width = shown.len().to_string().len().max(3);
    for (number, &position) in (1..).zip(shown) {
        let label = question.choices().get(position).map_or("", Choice::label);
        writeln!(output, "{number:>width$}. {}", escape(label))?;
    }
    if hidden > 0 {
        writeln!(output, "{hidden} more match. Type text to narrow the list.")?;
    }
    let pick = if question.most() == 1 {
        "Type a number to pick"
    } else {
        "Type numbers separated by spaces to pick"
    };
    write!(
        output,
        "{pick}, text to narrow the list, or nothing to cancel: "
    )?;
    output.flush()
}

/// Returns the line that says how many numbers to type.
fn count(least: usize, most: usize) -> String {
    if least == most {
        format!("Pick exactly {least}.")
    } else if most == usize::MAX {
        format!("Pick at least {least}.")
    } else {
        format!("Pick from {least} to {most}.")
    }
}

/// Returns `text` with each control character replaced by its
/// `char::escape_default` form, such as `\n` or `\u{1b}`.
fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        if character.is_control() {
            escaped.extend(character.escape_default());
        } else {
            escaped.push(character);
        }
    }
    escaped
}
