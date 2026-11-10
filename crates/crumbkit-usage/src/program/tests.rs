use super::{Program, is_program, parse};

#[test]
fn a_name_of_lowercase_letters_digits_dashes_and_underscores_that_starts_with_a_letter_is_a_program()
 {
    for name in [
        "flightdeck",
        "f",
        "cargo-crumbkit",
        "my_tool2",
        &"a".repeat(32),
    ] {
        assert!(is_program(name), "{name}");
        assert_eq!(Program::new(name).unwrap().as_str(), name);
    }
}

#[test]
fn an_empty_long_upper_case_or_punctuated_name_or_one_that_starts_with_a_digit_is_not_a_program() {
    for name in [
        "",
        &"a".repeat(33),
        "Flightdeck",
        "2fa",
        "-flightdeck",
        "_flightdeck",
        "flight deck",
        "flight.deck",
        "flightdéck",
        "--password=hunter2",
    ] {
        assert!(!is_program(name), "{name}");
        assert_eq!(Program::new(name), None, "{name}");
    }
}

#[test]
fn the_parser_names_the_pattern_when_it_refuses_a_name() {
    assert_eq!(parse("flightdeck"), Ok(Program::new("flightdeck").unwrap()));
    assert!(
        parse("Flight Deck")
            .unwrap_err()
            .contains("1 to 32 lowercase letters")
    );
}
