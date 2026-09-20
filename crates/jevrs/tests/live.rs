//! Live API coverage and fixture recording for the native client.

use std::sync::{Arc, Mutex};

use http::{Request, Response, header::AUTHORIZATION};
use jevrs::{Client, Error, Model, ReqwestTransport, RetryPolicy, TokioSleep, Transport};

#[path = "../../../tests/fixtures/mod.rs"]
mod fixtures;

#[derive(Clone, Default)]
struct Capture {
    bodies: Arc<Mutex<Bodies>>,
}

#[derive(Default)]
struct Bodies {
    request: Vec<u8>,
    status: u16,
    response: Vec<u8>,
}

impl Capture {
    fn set_request(&self, body: &[u8]) {
        self.bodies.lock().unwrap().request = body.to_vec();
    }

    fn set_response(&self, status: u16, body: &[u8]) {
        let mut bodies = self.bodies.lock().unwrap();
        bodies.status = status;
        bodies.response = body.to_vec();
    }

    fn record_or_assert(&self, name: &str) {
        let bodies = self.bodies.lock().unwrap();
        fixtures::record_or_assert(name, &bodies.request, bodies.status, &bodies.response);
    }
}

#[derive(Clone)]
struct RecordingTransport {
    inner: ReqwestTransport,
    capture: Capture,
    strip_authorization: bool,
}

impl RecordingTransport {
    fn new(strip_authorization: bool) -> (Self, Capture) {
        let capture = Capture::default();
        (
            Self {
                inner: ReqwestTransport::default(),
                capture: capture.clone(),
                strip_authorization,
            },
            capture,
        )
    }
}

impl Transport for RecordingTransport {
    type Error = reqwest::Error;

    async fn send(&self, mut request: Request<Vec<u8>>) -> Result<Response<Vec<u8>>, Self::Error> {
        if self.strip_authorization {
            request.headers_mut().remove(AUTHORIZATION);
        }
        self.capture.set_request(request.body());
        let response = self.inner.send(request).await?;
        self.capture
            .set_response(response.status().as_u16(), response.body());
        Ok(response)
    }

    fn is_retryable(error: &Self::Error) -> bool {
        ReqwestTransport::is_retryable(error)
    }
}

fn builder(
    strip_authorization: bool,
) -> (
    jevrs::ClientBuilder<RecordingTransport, TokioSleep>,
    Capture,
) {
    let (transport, capture) = RecordingTransport::new(strip_authorization);
    let retry = RetryPolicy {
        max_retries: 0,
        ..RetryPolicy::default()
    };
    (
        Client::builder(transport).sleep(TokioSleep).retry(retry),
        capture,
    )
}

#[tokio::test]
#[ignore = "calls the live Jev API"]
async fn triage_happy_path() {
    let (questions, urgent, department, frustration) = fixtures::triage();
    let (builder, capture) = builder(false);
    let client = builder.from_env().unwrap().build().unwrap();

    let answers = client.evaluate(&fixtures::STATE, &questions).await.unwrap();

    assert!(answers.model().starts_with("jev-"));
    assert!((0.0..=1.0).contains(&answers[urgent].p.get()));
    let department = &answers[department];
    assert!(["billing", "technical", "sales"].contains(&department.pick.as_str()));
    assert_eq!(department.probs.len(), 3);
    assert!(
        department
            .probs
            .iter()
            .all(|(_, probability)| (0.0..=1.0).contains(&probability.get()))
    );
    assert!((0.0..=1.0).contains(&department.confidence.get()));
    let frustration = &answers[frustration];
    assert!((0.0..=2.0).contains(&frustration.expected()));
    assert_eq!(frustration.probs.len(), 3);
    assert_eq!(frustration.legend.len(), 3);
    assert!((0.0..=1.0).contains(&frustration.confidence.get()));
    assert!(answers.usage().input_tokens > 0);
    assert!(answers.usage().output_tokens > 0);
    capture.record_or_assert("triage");
}

#[tokio::test]
#[ignore = "calls the live Jev API"]
async fn models_include_latest() {
    let (builder, capture) = builder(false);
    let client = builder.from_env().unwrap().build().unwrap();

    let models = client.models().await.unwrap();

    assert!(models.iter().any(|model| model.name == "jev-latest"));
    capture.record_or_assert("models");
}

#[tokio::test]
#[ignore = "calls the live Jev API"]
async fn unknown_model_is_bad_request() {
    let (questions, _, _, _) = fixtures::triage();
    let (builder, capture) = builder(false);
    let client = builder.from_env().unwrap().build().unwrap();

    let error = client
        .evaluate_with(&Model::from("jev-0.0.1"), &fixtures::STATE, &questions)
        .await
        .unwrap_err();

    assert!(matches!(error, Error::BadRequest { status: 400, .. }));
    capture.record_or_assert("unknown_model");
}

#[tokio::test]
#[ignore = "calls the live Jev API"]
async fn bad_key_is_auth() {
    let (questions, _, _, _) = fixtures::triage();
    let (builder, capture) = builder(false);
    let client = builder.api_key("invalid-key").build().unwrap();

    let error = client
        .evaluate(&fixtures::STATE, &questions)
        .await
        .unwrap_err();

    assert!(matches!(error, Error::Auth { status: 401, .. }));
    capture.record_or_assert("bad_key");
}

#[tokio::test]
#[ignore = "calls the live Jev API"]
async fn missing_key_is_auth() {
    let (questions, _, _, _) = fixtures::triage();
    let (builder, capture) = builder(true);
    let client = builder.api_key("removed-before-send").build().unwrap();

    let error = client
        .evaluate(&fixtures::STATE, &questions)
        .await
        .unwrap_err();

    assert!(matches!(error, Error::Auth { status: 403, .. }));
    capture.record_or_assert("missing_key");
}
