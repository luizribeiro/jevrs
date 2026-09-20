# `jevrs-core`

Use this crate when your runtime or HTTP stack is not supported by `jevrs`.
It contains the request builder, typed answer validation, and JSON codecs, but
performs no I/O and works without `std`.

Build a [`Questions`] batch and keep its typed [`Handle`] values. Call
[`encode`] to create the `POST /v1/systemone` body. Send those bytes with your
HTTP implementation, then call [`decode`] for a successful response or
[`classify`] for an unsuccessful status. You only need to implement the HTTP
exchange and any retry policy.

```
use jevrs_core::{Model, Questions, decode, encode};

let mut questions = Questions::new();
let urgent = questions.noul_with(
    "is_urgent",
    "Does this convey urgency?",
    "Explicitly time-sensitive",
    "No urgency expressed",
)?;
let request = encode(
    &Model::LATEST,
    &"Help! My payouts have been failing for 3 days.",
    &questions,
)?;
assert!(!request.is_empty());

let response = br#"{
  "model":"jev-1.13.0",
  "answers":{"is_urgent":{"type":"noul","noul":0.95}},
  "usage":{"input_tokens":414,"output_tokens":73}
}"#;
let answers = decode(&questions, response)?;
assert!(answers.get(urgent).is_yes(0.9));
# Ok::<(), jevrs_core::Error>(())
```

Use [`Options`] and [`Levels`] implementations for compile-time criteria. Pass
runtime criteria straight to [`Questions::choice_dyn`] and
[`Questions::score_dyn`]; they validate the items and name the question in
[`Error::InvalidCriteria`]. Use [`DynOptions`] and [`DynLevels`] to validate
once up front and reuse the criteria across batches. Implement [`QuestionSet`]
when you want a reusable typed group; `jevrs` also provides derives for these
traits.

Transport adapters should preserve error details for callers:

```
use core::time::Duration;
use jevrs_core::{Error, classify};

let error = classify(
    429,
    br#"{"detail":"Try again later"}"#,
    Some(Duration::from_secs(2)),
    Some("req_123"),
);
assert!(matches!(error, Error::RateLimited { .. }));
```
