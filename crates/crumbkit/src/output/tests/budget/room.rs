//! Tests for `Output::room`, which counts the bytes that `finish_to` prints
//! for a batch's rows as the handler pushes them (ADR-0068).

use schemars::JsonSchema;
use serde::Serialize;

use crate::help::Guide;
use crate::{Batch, Document, Error, JsonLayout, Row};

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
        .json_layout(JsonLayout::Pretty, "the readers expect it");
    let mut output = guided(&guide, &["--json", "--fields", "story"]);
    let mut room = output.room(STAR_BATCH);
    let mut batch = answered(Vec::new());
    assert!(room.has_room(&batch));
    let empty = room.printed();
    batch = answered((1..=3).map(star).collect());
    assert!(room.has_room(&batch));

    let finished = finish_output(&mut output, STAR_BATCH, batch);

    assert_eq!(room.printed(), finished.out.len(), "{}", finished.out);
    let flags = ["--json", "--fields", "story"];
    let nothing = finish_output(
        &mut guided(&guide, &flags),
        STAR_BATCH,
        answered(Vec::new()),
    );
    assert_eq!(empty, nothing.out.len());
}

#[test]
fn room_counts_a_text_table_with_its_widest_cells() {
    let mut output = budgeted(1 << 20, &[]);
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

    let finished = finish_output(&mut output, STAR_BATCH, batch);

    assert_eq!(room.printed(), finished.out.len(), "{}", finished.out);
    let mut small = budgeted(1024, &[]).room(STAR_BATCH);
    assert!(!small.has_room(&answered((1..=12).map(star).collect())));
}

/// A query answer, which serializes as an array of its values and so has no
/// fields to name as columns.
#[derive(Serialize, JsonSchema)]
struct Answer(String, u32);

impl Row for Answer {
    const COLUMNS: &'static [&'static str] = &[];
    const WHOLE: bool = true;
}

#[test]
fn room_counts_each_row_of_a_whole_row_type_without_columns_as_one_line_of_json() {
    const ANSWERS: Document<Batch<Answer, Error>> =
        Document::new("tool", "answers", 1, "The test answers.");
    let keys = ["io", "titan"];
    let mut batch = Batch::new(&["show"], "names", &keys);
    batch.push(Ok(Answer("Io".to_owned(), 1610)));
    batch.push(Ok(Answer("Titan".to_owned(), 1655)));
    let mut output = budgeted(1 << 20, &[]);
    let mut room = output.room(ANSWERS);
    assert!(room.has_room(&batch));

    let finished = finish_output(&mut output, ANSWERS, batch);

    assert_eq!(finished.out, "[\"Io\",1610]\n[\"Titan\",1655]\n");
    assert_eq!(room.printed(), finished.out.len());
}
