# Overview

Jev evaluates several typed questions against the same JSON state in one request. `jevrs` adds compile-time question sets, a runtime builder, retry control, and portable transports around that API. `typesafe-rs` is the other Rust client. It requires reqwest and Tokio and builds questions from strings. `jevrs` has a sans-I/O core, typed questions and answers through derives, and WASI transports.

# Quick start

Declare criteria once, derive a [`QuestionSet`], and call [`Client::ask`].
[`Answered<T>`](Answered) exposes the generated fields directly and retains
the model version and [`Usage`]. [`ClientBuilder::from_env`] reads
`TYPESAFE_API_KEY` and optional `TYPESAFE_BASE_URL` or `TYPESAFE_API_BASE`.

```no_run
use jevrs::{Choice, Client, Levels, Noul, Options, Questions, Score};

#[derive(Clone, Copy, Debug, Eq, Options, PartialEq)]
enum Department {
    /// Payments, invoicing, refunds
    Billing,
    /// Bugs, outages, integrations
    Technical,
    Sales,
}

#[derive(Clone, Copy, Debug, Eq, Levels, Ord, PartialEq, PartialOrd)]
enum Frustration {
    /// Calm
    Calm,
    /// Frustrated
    Frustrated,
    /// Very angry
    VeryAngry,
}

#[derive(Questions)]
struct Triage {
    #[jev(
        noul = "Does this convey urgency?",
        yes = "Explicitly time-sensitive",
        no = "No urgency expressed"
    )]
    is_urgent: Noul,
    #[jev(choice = "Which team should handle this?")]
    department: Choice<Department>,
    #[jev(score = "How frustrated is the customer?")]
    frustration: Score<Frustration>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::reqwest().from_env()?.build()?;
    let triage = client
        .ask::<Triage>(&"Help! My payouts have been failing for 3 days.")
        .await?;

    println!("urgent: {:.0}%", triage.is_urgent.p.get() * 100.0);
    println!("department: {:?}", triage.department.pick);
    println!("frustration: {:.2}", triage.frustration.expected());
    Ok(())
}
```

Run the complete native program in the
[`native-derive` example](https://github.com/luizribeiro/jevrs/tree/main/examples/native-derive).

# The typed path

Use the typed path when question IDs and criteria are part of your program.

```
use jevrs::{Indexed, Levels, Options};

#[derive(Clone, Copy, Debug, Eq, Options, PartialEq)]
enum Route {
    /// Account and payment questions
    Billing,
    #[jev(key = "tech", desc = "Bugs and outages")]
    Technical,
}

#[derive(Clone, Copy, Debug, Eq, Levels, Ord, PartialEq, PartialOrd)]
enum Priority {
    /// Normal queue
    Normal,
    /// Immediate attention
    Urgent,
}

assert_eq!(Route::Technical.key(), "tech");
assert_eq!(Priority::all(), &[Priority::Normal, Priority::Urgent]);
```

[`derive@Options`] gives a choice enum stable keys and optional descriptions.
[`derive@Levels`] turns an ordered enum into described score levels.
[`derive@Questions`] generates a [`QuestionSet`], handles, and answer fields.
[`Client::ask`] returns [`Answered<T>`](Answered), so `department.pick` stays an enum.

# The dynamic path

Use [`struct@Questions`] when criteria come from configuration or a database.
The builder validates the criteria and names the question in
[`Error::InvalidCriteria`]. Use [`DynOptions`] and [`DynLevels`] to validate
once up front, such as at startup from configuration, and reuse the criteria
across batches. Keep each typed [`Handle`], then read the matching answer with
[`Answers::get`]. Dynamic choices produce [`DynChoiceAnswer`]; dynamic scores
produce [`DynScoreAnswer`].

```no_run
use jevrs::{Client, Questions};

# async fn run() -> Result<(), jevrs::Error> {
let mut questions = Questions::new();
let urgent = questions.noul_with(
    "is_urgent",
    "Does this convey urgency?",
    "Explicitly time-sensitive",
    "No urgency expressed",
)?;
let department = questions.choice_dyn(
    "department",
    "Which team should handle this?",
    [
        ("billing", Some("Payments, invoicing, refunds")),
        ("technical", Some("Bugs, outages, integrations")),
        ("sales", None),
    ],
)?;
let frustration = questions.score_dyn(
    "frustration",
    "How frustrated is the customer?",
    ["Calm", "Frustrated", "Very angry"],
)?;

let client = Client::reqwest().from_env()?.build()?;
let answers = client
    .evaluate(&"Help! My payouts have been failing for 3 days.", &questions)
    .await?;
println!("urgent: {:.0}%", answers.get(urgent).p.get() * 100.0);
println!("department: {}", answers.get(department).pick);
println!("frustration: {:.2}", answers.get(frustration).expected());
# Ok(())
# }
```

See the
[`native-dynamic` example](https://github.com/luizribeiro/jevrs/tree/main/examples/native-dynamic)
for the complete program.

# Mixing both

A [`struct@Questions`] builder can use a static [`Options`] enum or [`Levels`] enum
beside runtime-defined criteria. Prefer this when only part of a request is
configured at runtime.

```
use jevrs::{Model, Options, Questions, encode};

#[derive(Clone, Copy, Debug, Eq, Options, PartialEq)]
enum Department {
    /// Payments, invoicing, refunds
    Billing,
    /// Bugs, outages, integrations
    Technical,
}

let mut questions = Questions::new();
questions.choice::<Department>(
    "department",
    "Which team should handle this?",
)?;
let request = encode(&Model::LATEST, &"A customer's card was declined.", &questions)?;
let request: serde_json::Value = serde_json::from_slice(&request)?;
assert_eq!(
    request["questions"]["department"]["criteria"]["billing"],
    "Payments, invoicing, refunds",
);
# Ok::<(), jevrs::Error>(())
```

# Transports

The default `reqwest` feature provides [`Client::reqwest`],
[`ReqwestTransport`], and [`TokioSleep`] for native Tokio applications. The
`wasip2` feature provides `Client::wasip2` and a synchronous WASI HTTP 0.2
transport on `wasm32-wasip2`; see the
[`wasip2` example](https://github.com/luizribeiro/jevrs/tree/main/examples/wasip2).
The shipped `wasip3` feature provides `Client::wasip3` and an asynchronous WASI
HTTP 0.3 transport on `wasm32-wasip3`; see the
[`wasip3` example](https://github.com/luizribeiro/jevrs/tree/main/examples/wasip3).

Implement [`Transport`] when you already have an HTTP stack. The client gives
it a complete `http::Request<Vec<u8>>`, including authorization. Return the
complete response and mark only safe transport failures as retryable: a
failure before any bytes reached the server, never one after a partial send.

```
use std::{convert::Infallible, future::Future};
use http::{Request, Response};
use jevrs::{MaybeSend, Transport};

struct MyTransport;

impl Transport for MyTransport {
    type Error = Infallible;

    fn send(
        &self,
        _request: Request<Vec<u8>>,
    ) -> impl Future<Output = Result<Response<Vec<u8>>, Self::Error>> + MaybeSend {
        async { Ok(Response::new(Vec::new())) }
    }
}
```

# Retries and errors

[`RetryPolicy`] retries HTTP 429 and 529 responses plus failures that
[`Transport::is_retryable`] accepts. A server `Retry-After` value wins over
exponential backoff. A custom transport needs a [`Sleep`] implementation to
wait between attempts; [`NoSleep`] disables retries. Native and WASI client
constructors install their matching sleeper.

Match the non-exhaustive [`Error`] enum by category. API errors retain parsed
details, the raw body, and [`ErrorDetail::request_id`] for support requests.

```no_run
use jevrs::{Client, Error, RetryPolicy};

fn report(error: &Error) {
    match error {
        Error::Auth { detail, .. }
        | Error::BadRequest { detail, .. }
        | Error::RateLimited { detail, .. }
        | Error::Overloaded { detail }
        | Error::Http { detail, .. } => {
            eprintln!("API error; request id: {:?}", detail.request_id);
        }
        Error::Transport(source) => eprintln!("transport error: {source}"),
        other => eprintln!("request failed: {other}"),
    }
}

# fn configured() -> Result<(), Error> {
let client = Client::reqwest()
    .retry(RetryPolicy { max_retries: 4, ..RetryPolicy::default() })
    .from_env()?
    .build()?;
# let _ = (client, report);
# Ok(())
# }
```

# Feature flags

| Feature | Default | Use it for |
| --- | --- | --- |
| `derive` | yes | The `Options`, `Levels`, and `Questions` derives. |
| `reqwest` | yes | Native HTTP with reqwest's rustls backend and Tokio sleep. |
| `native-tls` | no | Native HTTP with the platform TLS backend. |
| `wasip2` | no | WASI HTTP 0.2 components on `wasm32-wasip2`. |
| `wasip3` | no | Draft WASI HTTP 0.3 components on `wasm32-wasip3`. |
| `test-util` | no | [`MockTransport`] and [`MockSleep`] in downstream tests. |
| `web` | no | Reserved; it does not provide a browser transport yet. |

# Limits

Jev 1.13 accepts at most 64,000 tokens total. The state plus the longest
question may use at most 32,000 tokens. `jevrs` leaves token counting to the
server. A request needs at least one question. [`Options`] and [`DynOptions`]
accept 1 to 255 choices; [`Levels`] and [`DynLevels`] accept 2 to 10 score
levels.

```
use jevrs::{DynLevels, Error};

let levels = (0..11).map(|index| format!("Level {index}"));
assert!(matches!(
    DynLevels::new(levels),
    Err(Error::InvalidCriteria { .. }),
));
```
