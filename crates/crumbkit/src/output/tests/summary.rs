//! Tests for how `finish_to` prints a listing's summary: a line above the
//! table in text, and an object under `summary` in JSON.

use std::fmt;

use serde::Serialize;

use super::{Planet, finish_with, mercury, saturn};
use crate::{Document, Listing};

/// The summary of a listing of planets.
#[derive(Serialize)]
struct Census {
    planets: u32,
    star: &'static str,
}

impl fmt::Display for Census {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} planets around the {}.", self.planets, self.star)
    }
}

const CENSUS: Document<Listing<Planet, Census>> = Document::new("solar", "census", 1);

/// Returns a listing of Mercury and Saturn out of 8 planets, with a summary
/// and a note.
fn census() -> Listing<Planet, Census> {
    Listing::new(vec![mercury(), saturn()], 8)
        .summary(Census {
            planets: 8,
            star: "Sun",
        })
        .note("Planets from the 2006 definition.")
}

#[test]
fn a_summary_prints_above_the_table_in_text() {
    let finished = finish_with(&[], CENSUS, census());

    let expected = "\
8 planets around the Sun.
NAME     MOONS
Mercury  0
Saturn   274
Showing 2 of 8. --all shows every one.
Planets from the 2006 definition.
";
    assert_eq!(finished.out, expected);
}

#[test]
fn json_holds_the_summary_as_an_object_after_the_total() {
    let compact = finish_with(&["--json"], CENSUS, census());
    let full = finish_with(&["--json", "--full"], CENSUS, census());

    assert_eq!(
        compact.out,
        concat!(
            r#"{"schema":"solar/census/v1","data":[{"name":"Mercury","moons":0},{"name":"Saturn","moons":274}],"#,
            r#""total":8,"summary":{"planets":8,"star":"Sun"}}"#,
            "\n"
        )
    );
    assert!(
        full.out
            .ends_with("\"total\":8,\"summary\":{\"planets\":8,\"star\":\"Sun\"}}\n"),
        "{}",
        full.out
    );
}
