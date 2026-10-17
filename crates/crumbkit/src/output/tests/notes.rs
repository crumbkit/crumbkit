//! Tests for where `finish_to` prints the lines that describe a result: a
//! listing's footer, its empty note, the note of a listing, a batch or a
//! prose result, the `Next:` line, the cut line and the dry-run line all go
//! to the error writer, after the output writer is flushed (ADR-0050,
//! ADR-0077).

use std::cell::RefCell;
use std::fmt;
use std::io::{self, BufWriter, Write};
use std::process::ExitCode;
use std::rc::Rc;

use clap::CommandFactory as _;

use super::{
    PLANET_LISTING, PLANETS, Planet, Tool, built_with, finish_output, finish_with, mercury,
    parse_against, saturn,
};
use crate::answered::answered;
use crate::help::Guide;
use crate::{
    Batch, Call, Code, Document, Error, Failure, Listing, Prose, Recovery, Sink, Spelling,
    finish_to,
};

/// Returns a listing of Mercury and Saturn out of 8 planets, with a note and
/// a next step.
fn two_of_eight() -> Listing<Planet> {
    Listing::new(vec![mercury(), saturn()], 8)
        .described("planets")
        .note("Pluto is not a planet.")
        .next(Call::new(&["show"]).arg("names"), "shows planets by name.")
}

#[test]
fn a_listings_footer_note_and_next_line_print_on_standard_error_and_standard_output_holds_the_rows_alone()
 {
    let finished = finish_with(&[], PLANET_LISTING, two_of_eight());

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(finished.out, "NAME     MOONS\nMercury  0\nSaturn   274\n");
    assert_eq!(
        finished.err,
        "\
Showing 2 of 8 planets. --all shows every one.
Pluto is not a planet.
Next: tool show <NAME>... shows planets by name.
"
    );
}

#[test]
fn a_listing_with_no_rows_prints_its_header_on_standard_output_and_its_empty_note_on_standard_error()
 {
    let listing: Listing<Planet> = Listing::new(Vec::new(), 0).described("planets");
    let finished = finish_with(&[], PLANET_LISTING, listing);

    assert_eq!(finished.out, "NAME  MOONS\n");
    assert_eq!(finished.err, "note: no planets match\n");
}

#[test]
fn a_listings_json_prints_no_notes_on_either_writer() {
    let finished = finish_with(&["--json"], PLANET_LISTING, two_of_eight());

    assert_eq!(finished.out.lines().count(), 1);
    assert_eq!(finished.err, "");
}

/// One writer that both of a sink's writers append to, as a terminal or
/// `2>&1` merges standard output and standard error.
#[derive(Clone, Default)]
struct Merged(Rc<RefCell<Vec<u8>>>);

impl Write for Merged {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_reader_that_merges_the_two_streams_sees_the_notes_after_the_rows_because_standard_output_is_flushed_first()
 {
    let merged = Merged::default();
    // The output writer holds its bytes until it is flushed, as the
    // process's standard output does on a pipe.
    let mut out = BufWriter::new(merged.clone());
    let mut err = merged.clone();
    let mut output = parse_against(Tool::command(), &[]);
    let result: Result<_, Error> = Ok(two_of_eight());
    finish_to(
        &mut Sink::new(&mut out, &mut err),
        &mut output,
        Spelling::COLONS,
        PLANET_LISTING,
        result,
    );

    let text = String::from_utf8(merged.0.borrow().clone()).unwrap();
    assert!(
        text.starts_with("NAME     MOONS\nMercury  0\nSaturn   274\nShowing 2 of 8"),
        "{text}"
    );
}

/// A writer that counts its calls, so a test sees how many writes a note
/// takes.
#[derive(Default)]
struct Calls {
    writes: usize,
    bytes: Vec<u8>,
}

impl Write for Calls {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.writes += 1;
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn the_notes_reach_standard_error_in_one_write_so_no_note_is_split_between_other_lines() {
    let mut out = Vec::new();
    let mut err = Calls::default();
    let mut output = parse_against(Tool::command(), &[]);
    let result: Result<_, Error> = Ok(two_of_eight());
    finish_to(
        &mut Sink::new(&mut out, &mut err),
        &mut output,
        Spelling::COLONS,
        PLANET_LISTING,
        result,
    );

    assert_eq!(err.writes, 1);
    assert!(err.bytes.ends_with(b"by name.\n"));
}

const GONE: Code = Code::new("solar", "planets", "gone");

/// A planet that the batch could not find.
#[derive(Debug)]
struct Gone;

impl fmt::Display for Gone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Vulcan is gone")
    }
}

impl std::error::Error for Gone {}

impl Failure for Gone {
    fn code(&self) -> &'static Code {
        &GONE
    }

    fn recovery(&self) -> Recovery {
        Recovery::Retry
    }
}

#[test]
fn a_dry_runs_line_prints_on_standard_error_after_a_batchs_failures() {
    let mut output = parse_against(Tool::command(), &[]);
    output.dry_run = true;
    let finished = finish_output(&mut output, SHOWN, answered(vec![saturn()], vec![Gone]));

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(finished.out, "NAME    MOONS\nSaturn  274\n");
    assert!(
        finished.err.starts_with("error: Vulcan is gone\n"),
        "{}",
        finished.err
    );
    assert!(
        finished
            .err
            .ends_with("\nThis was a dry run, so nothing changed.\n"),
        "{}",
        finished.err
    );
}

#[test]
fn a_dry_runs_line_prints_on_standard_error_after_a_list() {
    let mut output = parse_against(Tool::command(), &[]);
    output.dry_run = true;
    let finished = finish_output(&mut output, PLANETS, vec![saturn()]);

    assert_eq!(finished.out, "NAME    MOONS\nSaturn  274\n");
    assert_eq!(finished.err, "This was a dry run, so nothing changed.\n");
}

/// The answer of a command that counts planets, which prints as one sentence.
#[derive(serde::Serialize, schemars::JsonSchema)]
struct Count {
    /// The number of planets.
    planets: u32,
}

impl fmt::Display for Count {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "The solar system has {} planets.", self.planets)
    }
}

const COUNT: Document<Prose<Count, Gone>> =
    Document::new("solar", "count", 1, "The number of planets.");

const SHOWN: Document<Batch<Planet, Gone>> =
    Document::new("solar", "planets-show", 1, "The test rows.");

#[test]
fn a_prose_results_note_prints_on_standard_error_after_its_text_is_flushed() {
    let prose = Prose::new(Count { planets: 8 }).note("Catalogue: solar.toml");
    let finished = finish_with(&[], COUNT, prose);

    assert_eq!(finished.code, ExitCode::SUCCESS);
    assert_eq!(finished.out, "The solar system has 8 planets.\n");
    assert_eq!(finished.err, "Catalogue: solar.toml\n");
}

#[test]
fn a_prose_results_json_leaves_its_note_out() {
    let prose = Prose::new(Count { planets: 8 }).note("Catalogue: solar.toml");
    let finished = finish_with(&["--json"], COUNT, prose);

    assert!(!finished.out.contains("solar.toml"), "{}", finished.out);
    assert_eq!(finished.err, "");
}

#[test]
fn a_prose_results_note_prints_before_its_failures() {
    let prose =
        Prose::with_failures(Count { planets: 8 }, vec![Gone]).note("Catalogue: solar.toml");
    let finished = finish_with(&[], COUNT, prose);

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert!(
        finished
            .err
            .starts_with("Catalogue: solar.toml\nerror: Vulcan is gone\n"),
        "{}",
        finished.err
    );
}

#[test]
fn a_batchs_note_prints_on_standard_error_before_its_failures_and_the_dry_runs_line() {
    let mut output = parse_against(Tool::command(), &[]);
    output.dry_run = true;
    let batch = answered(vec![saturn()], vec![Gone]).note("Pluto is not a planet.");
    let finished = finish_output(&mut output, SHOWN, batch);

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert_eq!(finished.out, "NAME    MOONS\nSaturn  274\n");
    assert!(
        finished
            .err
            .starts_with("Pluto is not a planet.\nerror: Vulcan is gone\n"),
        "{}",
        finished.err
    );
    assert!(
        finished
            .err
            .ends_with("\nThis was a dry run, so nothing changed.\n"),
        "{}",
        finished.err
    );
}

#[test]
fn a_batchs_json_leaves_its_note_out() {
    let batch = answered(vec![saturn()], vec![Gone]).note("Pluto is not a planet.");
    let finished = finish_with(&["--json"], SHOWN, batch);

    assert!(!finished.out.contains("Pluto"), "{}", finished.out);
    assert_eq!(finished.err, "");
}

#[test]
fn a_batch_that_the_output_budget_cuts_prints_its_note_before_the_failure_that_names_the_keys_left_out()
 {
    let guide = Guide::new().output_budget(1024, "the tests print little");
    let mut output = built_with(Tool::command(), &guide, &["tool", "list"]);
    let rows: Vec<Planet> = (0..100).map(|_| mercury()).collect();
    let batch = answered::<Planet, Gone>(rows, Vec::new()).note("Pluto is not a planet.");
    let finished = finish_output(&mut output, SHOWN, batch);

    assert_eq!(finished.code, ExitCode::FAILURE);
    assert!(
        finished.err.starts_with("Pluto is not a planet.\nerror: ")
            && finished
                .err
                .contains("more answers did not fit in the output budget"),
        "{}",
        finished.err
    );
}
