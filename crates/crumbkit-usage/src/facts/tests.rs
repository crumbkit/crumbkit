use std::time::Duration;

use serde_json::json;

use super::{Extension, Extensions, Facts, Key, Label, is_key, is_label};

const REQUESTS: Key = Key::new("requests");
const RETRIES: Key = Key::new("retries");
const ROUTE: Key = Key::new("route");
const SHOW: Label = Label::new("/books/{id}");
const LIST: Label = Label::new("/books");

/// A tool's facts about the requests that a run sent to a catalogue
/// server.
#[derive(Default)]
struct Requests {
    sent: u64,
    routes: Vec<Label>,
    waited: Duration,
}

impl Extension for Requests {
    fn write(&self, facts: &mut Facts) {
        facts.count(REQUESTS, self.sent).waited(self.waited);
        for route in &self.routes {
            facts.label(ROUTE, *route);
        }
    }
}

/// A second extension of the same run, such as one that a library crate
/// keeps beside the tool's own.
#[derive(Default)]
struct Retries(u64);

impl Extension for Retries {
    fn write(&self, facts: &mut Facts) {
        facts.count(RETRIES, self.0).count(REQUESTS, self.0);
    }
}

#[test]
fn a_run_without_an_extension_writes_no_facts() {
    assert!(Extensions::default().written().is_none());
}

#[test]
fn an_extension_changed_twice_keeps_one_value_and_writes_its_facts() {
    let mut extensions = Extensions::default();

    extensions.change(|requests: &mut Requests| {
        requests.sent += 1;
        requests.routes.push(SHOW);
        requests.waited += Duration::from_millis(120);
    });
    extensions.change(|requests: &mut Requests| {
        requests.sent += 1;
        requests.routes.push(SHOW);
        requests.routes.push(LIST);
        requests.waited += Duration::from_millis(80);
    });

    assert_eq!(
        serde_json::to_value(extensions.written().unwrap()).unwrap(),
        json!({
            "counts": {"requests": 2},
            "labels": {"route": ["/books/{id}", "/books"]},
            "waited_ms": 200,
        })
    );
}

#[test]
fn two_extensions_add_their_counts_under_a_shared_name() {
    let mut extensions = Extensions::default();

    extensions.change(|requests: &mut Requests| requests.sent = 3);
    extensions.change(|retries: &mut Retries| retries.0 = 1);

    assert_eq!(
        serde_json::to_value(extensions.written().unwrap()).unwrap(),
        json!({"counts": {"requests": 4, "retries": 1}})
    );
}

#[test]
fn facts_that_hold_nothing_but_zeros_write_no_section() {
    let mut extensions = Extensions::default();

    extensions.change(|_: &mut Requests| {});

    assert!(extensions.written().is_none());
}

#[test]
fn a_count_that_would_overflow_stays_at_the_largest_number() {
    let mut facts = Facts::default();

    facts.count(REQUESTS, u64::MAX).count(REQUESTS, 1);

    assert_eq!(facts.0.counts.get("requests"), Some(&u64::MAX));
}

#[test]
fn a_key_is_a_lowercase_name_of_at_most_32_bytes() {
    for good in ["requests", "status_5xx", "a", &"a".repeat(32)] {
        assert!(is_key(good), "{good}");
    }
    for bad in [
        "",
        "Requests",
        "5xx",
        "_requests",
        "wait-ms",
        "a b",
        &"a".repeat(33),
    ] {
        assert!(!is_key(bad), "{bad}");
    }
}

#[test]
fn a_label_is_at_most_64_bytes_of_a_route_templates_characters() {
    for good in [
        "2xx",
        "/books/{id}",
        "GET",
        "staging",
        "v1.2:read",
        &"a".repeat(64),
    ] {
        assert!(is_label(good), "{good}");
    }
    for bad in [
        "",
        "/books/42?key=s3cr3t",
        "a b",
        "naïve",
        "x@y",
        &"a".repeat(65),
    ] {
        assert!(!is_label(bad), "{bad}");
    }
}

#[test]
#[should_panic(expected = "a usage key")]
fn a_key_built_at_run_time_from_a_bad_name_panics() {
    let name: &'static str = Box::leak("Bad Name".to_owned().into_boxed_str());
    let _ = Key::new(name);
}

#[test]
fn extend_does_not_run_its_change_when_no_log_was_started() {
    // No unit test starts the process's log, so its recorder stays idle.
    let mut ran = false;

    super::extend(|_: &mut Requests| ran = true);

    assert!(!ran);
}

#[test]
fn an_extension_that_panics_while_it_writes_drops_every_fact_of_the_run() {
    #[derive(Default)]
    struct Broken;
    impl Extension for Broken {
        fn write(&self, _facts: &mut Facts) {
            panic!("the extension's write failed");
        }
    }
    let mut extensions = Extensions::default();

    extensions.change(|requests: &mut Requests| requests.sent = 3);
    extensions.change(|_: &mut Broken| {});

    assert!(extensions.written().is_none());
}
