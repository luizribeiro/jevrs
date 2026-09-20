//! Shared live API fixture support.
//!
//! Re-recording a fixture means deleting its request/response pair first;
//! [`write_once`] refuses to overwrite recorded API data.

#![allow(dead_code)]

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

use jevrs_core::{DynLevels, DynOptions, Handle, NoulQ, Questions};
use serde_json::Value;

pub(crate) const STATE: &str = "Help! My payouts have been failing for 3 days.";

pub(crate) type Triage = (
    Questions,
    Handle<NoulQ>,
    Handle<jevrs_core::DynChoiceQ>,
    Handle<jevrs_core::DynScoreQ>,
);

pub(crate) fn triage() -> Triage {
    let mut questions = Questions::new();
    let urgent = questions
        .noul_with(
            "is_urgent",
            "Does this convey urgency?",
            "Explicitly time-sensitive",
            "No urgency expressed",
        )
        .unwrap();
    let department = questions
        .choice_dyn(
            "department",
            "Which team should handle this?",
            DynOptions::new([
                ("billing", Some("Payments, invoicing, refunds")),
                ("technical", Some("Bugs, outages, integrations")),
                ("sales", None),
            ])
            .unwrap(),
        )
        .unwrap();
    let frustration = questions
        .score_dyn(
            "frustration",
            "How frustrated is the customer?",
            DynLevels::new(["Calm", "Frustrated", "Very angry"]).unwrap(),
        )
        .unwrap();
    (questions, urgent, department, frustration)
}

pub(crate) fn record_or_assert(name: &str, request: &[u8], status: u16, response: &[u8]) {
    let request = parse_body(request);
    let response = serde_json::json!({
        "status": status,
        "body": parse_body(response),
    });
    if std::env::var_os("JEVRS_RECORD").as_deref() == Some("1".as_ref()) {
        write_once(name, "request", &request);
        write_once(name, "response", &response);
    } else {
        assert_same_shape(&load(name, "request"), &request, "$request");
        assert_same_shape(&load(name, "response"), &response, "$response");
    }
}

pub(crate) fn load(name: &str, kind: &str) -> Value {
    serde_json::from_slice(&fs::read(path(name, kind)).unwrap()).unwrap()
}

pub(crate) fn names() -> Vec<String> {
    let mut names = fs::read_dir(directory())
        .unwrap()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry
                .file_name()
                .to_str()
                .and_then(|name| name.strip_suffix(".request.json"))
                .map(str::to_owned)
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn parse_body(body: &[u8]) -> Value {
    if body.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(body).unwrap()
    }
}

fn write_once(name: &str, kind: &str, value: &Value) {
    let path = path(name, kind);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap();
    writeln!(file, "{}", serde_json::to_string_pretty(value).unwrap()).unwrap();
}

fn assert_same_shape(expected: &Value, actual: &Value, path: &str) {
    match (expected, actual) {
        (Value::Null, Value::Null)
        | (Value::Bool(_), Value::Bool(_))
        | (Value::Number(_), Value::Number(_))
        | (Value::String(_), Value::String(_)) => {}
        (Value::Array(expected), Value::Array(actual)) => {
            if let Some(element) = expected.first() {
                for (index, actual) in actual.iter().enumerate() {
                    assert_same_shape(element, actual, &format!("{path}[{index}]"));
                }
            } else {
                assert!(actual.is_empty(), "shape mismatch at {path}");
            }
        }
        (Value::Object(expected), Value::Object(actual)) => {
            let expected_keys = expected.keys().collect::<Vec<_>>();
            let actual_keys = actual.keys().collect::<Vec<_>>();
            assert_eq!(expected_keys, actual_keys, "key mismatch at {path}");
            for (key, expected) in expected {
                assert_same_shape(expected, &actual[key], &format!("{path}.{key}"));
            }
        }
        _ => panic!("type mismatch at {path}"),
    }
}

fn path(name: &str, kind: &str) -> PathBuf {
    directory().join(format!("{name}.{kind}.json"))
}

fn directory() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .map(|ancestor| ancestor.join("tests/fixtures"))
        .find(|candidate| candidate.is_dir())
        .unwrap_or_else(|| {
            panic!(
                "could not locate tests/fixtures from {}",
                manifest.display()
            )
        })
}
