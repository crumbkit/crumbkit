//! The codes check, which a tool's test runs over every failure code the tool
//! can print.

use crumbkit::{Code, Spelling};

#[cfg(test)]
mod tests;

/// Checks a tool's failure codes as `spelling` prints them, and panics, as a
/// failed test assertion does, naming every problem it finds.
///
/// `codes` holds the `CODES` list of each of the tool's error types. A code
/// that two lists share counts once. The check finds:
///
/// - two different codes that spell the same text, so a caller could not
///   tell them apart;
/// - a code that spells as an empty string;
/// - a code whose text holds whitespace, so it would not read as one word;
/// - a code whose text holds characters outside ASCII.
///
/// The failure document's JSON Schema accepts exactly the codes that pass:
/// non-empty ASCII text without whitespace.
///
/// ```
/// use crumbkit::{Code, Spelling};
///
/// const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");
/// const TIMED_OUT: Code = Code::new("shelf", "network", "timed_out");
///
/// crumbkit_test::check_codes(Spelling::COLONS, &[&[&NOT_FOUND], &[&TIMED_OUT]]);
/// ```
///
/// # Panics
///
/// Panics when any code breaks a rule above. The message lists each problem
/// on its own line and names each code as the `Code::new` call that makes it.
pub fn check_codes(spelling: Spelling, codes: &[&[&Code]]) {
    let mut problems = Vec::new();
    let mut spelled: Vec<(String, &Code)> = Vec::new();
    for &code in codes.iter().flat_map(|list| list.iter()) {
        if spelled.iter().any(|&(_, seen)| seen == code) {
            continue;
        }
        let text = spelling.spell(code);
        if let Some((_, other)) = spelled.iter().find(|(seen, _)| *seen == text) {
            problems.push(format!(
                "{} and {} both spell {text:?}",
                source(other),
                source(code)
            ));
        }
        if text.is_empty() {
            problems.push(format!("{} spells as an empty string", source(code)));
        }
        if text.contains(char::is_whitespace) {
            problems.push(format!(
                "{} spells as {text:?}, which holds whitespace",
                source(code)
            ));
        }
        if !text.is_ascii() {
            problems.push(format!(
                "{} spells as {text:?}, which holds characters outside ASCII",
                source(code)
            ));
        }
        spelled.push((text, code));
    }
    assert!(
        problems.is_empty(),
        "the failure codes break crumbkit's rules:\n{}",
        problems.join("\n")
    );
}

/// Returns the `Code::new` call that makes `code`, such as
/// `Code::new("shelf", "books", "not_found")`, so a reader can search for it.
fn source(code: &Code) -> String {
    format!(
        "Code::new({:?}, {:?}, {:?})",
        code.component(),
        code.area(),
        code.reason()
    )
}
