//! Codec coverage for every body recorded by the live client tests.

use jevrs_core::{Error, Model, classify, decode, encode};

#[path = "../../../tests/fixtures/mod.rs"]
mod fixtures;

#[test]
fn every_recorded_fixture_exercises_the_codec() {
    let names = fixtures::names();
    assert!(!names.is_empty());

    for name in names {
        let request = fixtures::load(&name, "request");
        let response = fixtures::load(&name, "response");
        let (questions, urgent, department, frustration) = fixtures::triage();

        if name == "models" {
            assert!(request.is_null());
            continue;
        }
        let model = if name == "unknown_model" {
            Model::from("jev-0.0.1")
        } else {
            Model::LATEST
        };
        let encoded = encode(&model, &fixtures::STATE, &questions).unwrap();
        let encoded: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(encoded, request, "request fixture {name}");

        let status = u16::try_from(response["status"].as_u64().unwrap()).unwrap();
        let body = serde_json::to_vec(&response["body"]).unwrap();
        if name == "triage" {
            let answers = decode(&questions, &body).unwrap();
            assert!((0.0..=1.0).contains(&answers[urgent].p.get()));
            assert_eq!(answers[department].probs.len(), 3);
            assert_eq!(answers[frustration].probs.len(), 3);
            continue;
        }

        let (expected_status, expected_auth) = match name.as_str() {
            "bad_key" => (401, true),
            "missing_key" => (403, true),
            "unknown_model" => (400, false),
            _ => panic!("unexpected fixture: {name}"),
        };
        assert_eq!(status, expected_status);
        let detail = match (expected_auth, classify(status, &body, None, None)) {
            (true, Error::Auth { status, detail }) if status == expected_status => detail,
            (false, Error::BadRequest { status, detail }) if status == expected_status => detail,
            (_, error) => panic!("unexpected classification for {name}: {error:?}"),
        };
        assert!(detail.message.is_some_and(|message| !message.is_empty()));
    }
}
