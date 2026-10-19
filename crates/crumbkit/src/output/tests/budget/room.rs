//! Tests for `Output::room`, which counts the bytes that `finish_to` prints
//! for a batch's rows as the handler pushes them (ADR-0068).

use schemars::JsonSchema;
use serde::Serialize;

use crate::help::Guide;
use crate::{Batch, Document, Error, JsonLayout, Row, Rows as _};

use super::{STAR_BATCH, Star, budgeted, finish_output, guided, star};

/// Returns a batch of `tool show` that holds `rows`, one for each key.
fn answered(rows: Vec<Star>) -> Batch<Star, Error> {
    let keys: Vec<String> = rows.iter().map(|row| row.name.clone()).collect();
    let mut batch = Batch::new(&["show"], "names", &keys);
    batch.extend(rows.into_iter().map(Ok));
    batch
}

#[test]
fn room_counts_a_pretty_document_s_rows_as_the_document_prints_them() {
    let guide = Guide::new()
        .output_budget(1 << 20, "the tests print little")
        .json_layout(JsonLayout::Pretty {
            reason: "the readers expect it",
        });
    let output = guided(&guide, &["--json", "--fields", "story"]);
    let mut room = output.room(STAR_BATCH);
    let mut batch = answered(Vec::new());
    assert!(room.has_room(&batch));
    let empty = room.printed();
    batch = answered((1..=3).map(star).collect());
    assert!(room.has_room(&batch));

    let finished = finish_output(output, STAR_BATCH, batch);

    assert_eq!(room.printed(), finished.out.len(), "{}", finished.out);
    let flags = ["--json", "--fields", "story"];
    let nothing = finish_output(guided(&guide, &flags), STAR_BATCH, answered(Vec::new()));
    assert_eq!(empty, nothing.out.len());
}

#[test]
fn room_has_room_for_a_batch_without_rows_at_a_budget_of_0() {
    for flags in [&[][..], &["--json"]] {
        let mut room = budgeted(0, flags).room(STAR_BATCH);

        assert!(room.has_room(&answered(Vec::new())), "{flags:?}");
    }
}

#[test]
fn a_table_batch_fetched_through_room_prints_its_rows_that_fit_and_names_the_rest_in_text_and_json()
{
    let keys: Vec<String> = (1..=20).map(|number| star(number).name).collect();
    for flags in [&[][..], &["--json"]] {
        let output = budgeted(1024, flags);
        let mut room = output.room(STAR_BATCH);
        let mut batch = Batch::new(&["show"], "names", &keys);
        for number in 1..=20 {
            if !room.has_room(&batch) {
                break;
            }
            batch.push(Ok(star(number)));
        }
        let fetched = batch.rows().len();

        let finished = finish_output(output, STAR_BATCH, batch);

        assert!(fetched > 1 && fetched < 20, "{flags:?}: {fetched}");
        assert!(finished.out.len() <= 1024, "{flags:?}: {}", finished.out);
        let shown = if flags.is_empty() {
            finished.out.lines().count() - 1
        } else {
            let document: serde_json::Value = serde_json::from_str(&finished.out).unwrap();
            document["data"].as_array().unwrap().len()
        };
        let named = keys[shown..].join(" ");
        assert!(shown > 0, "{flags:?}: {}", finished.out);
        if flags.is_empty() {
            assert!(
                finished
                    .err
                    .contains(&format!("help:  run `tool show {named}`")),
                "{}",
                finished.err
            );
        } else {
            let document: serde_json::Value = serde_json::from_str(&finished.out).unwrap();
            assert_eq!(
                document["failures"][0]["fields"]["left_out"],
                serde_json::json!(keys[shown..])
            );
        }
    }
}

#[test]
fn room_counts_a_text_table_with_its_widest_cells() {
    let output = budgeted(1 << 20, &[]);
    let mut room = output.room(STAR_BATCH);
    let rows = |count: usize| -> Vec<Star> {
        (1..=count)
            .map(|number| Star {
                name: "n".repeat(number * 3),
                story: if number == 2 {
                    String::new()
                } else {
                    "é".repeat(number)
                },
            })
            .collect()
    };
    assert!(room.has_room(&answered(rows(2))));
    let batch = answered(rows(4));
    assert!(room.has_room(&batch));

    let finished = finish_output(output, STAR_BATCH, batch);

    assert_eq!(room.printed(), finished.out.len(), "{}", finished.out);
    let mut small = budgeted(1024, &[]).room(STAR_BATCH);
    assert!(!small.has_room(&answered((1..=12).map(star).collect())));
}

/// A query answer, which serializes as an array of its values and so has no
/// fields to name as columns. An answer with a year prints its own line, and
/// one without prints as one line of JSON.
#[derive(Serialize, JsonSchema)]
struct Answer(String, Option<u32>);

impl Row for Answer {
    const COLUMNS: &'static [&'static str] = &[];
    const LINE: Option<&'static str> = Some("<NAME> <YEAR>");
    const WHOLE: bool = true;

    fn text_line(&self) -> Option<String> {
        self.1.map(|year| format!("{} {year}", self.0))
    }
}

#[test]
fn room_counts_each_row_of_a_row_type_with_its_own_line_as_the_line_it_prints() {
    const ANSWERS: Document<Batch<Answer, Error>> =
        Document::new("tool", "answers", 1, "The test answers.");
    let keys = ["io", "titan"];
    let mut batch = Batch::new(&["show"], "names", &keys);
    batch.push(Ok(Answer("Io".to_owned(), Some(1610))));
    batch.push(Ok(Answer("Titan".to_owned(), None)));
    let output = budgeted(1 << 20, &[]);
    let mut room = output.room(ANSWERS);
    assert!(room.has_room(&batch));

    let finished = finish_output(output, ANSWERS, batch);

    assert_eq!(finished.out, "Io 1610\n[\"Titan\",null]\n");
    assert_eq!(room.printed(), finished.out.len());
}
