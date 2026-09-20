use std::{env, fmt, time::Duration};

use http::{
    HeaderName, HeaderValue, Method, Request, Response, StatusCode,
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, USER_AGENT},
};
use jevrs_core::{
    Answered, Answers, Error, ErrorDetail, Model, QuestionSet, Questions, classify, decode, encode,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{NoSleep, RetryPolicy, Sleep, Transport};
#[cfg(any(feature = "reqwest", feature = "native-tls"))]
use crate::{ReqwestTransport, TokioSleep};

const DEFAULT_BASE_URL: &str = "https://api.typesafe.ai";
const EVALUATE_PATH: &str = "/v1/systemone";
const MODELS_PATH: &str = "/v1/models";

/// A Jev model advertised by the models endpoint.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ModelInfo {
    /// Model name accepted in an evaluation request.
    pub name: String,
    /// Human-readable model summary, when supplied.
    pub description: Option<String>,
    /// API-provided release timestamp, preserved as text.
    pub release_date: Option<String>,
    /// Additional fields returned by newer API versions.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// An async Jev API client backed by caller-selected I/O implementations.
pub struct Client<T: Transport, S: Sleep = NoSleep> {
    base_url: String,
    api_key: String,
    model: Model,
    retry: RetryPolicy,
    transport: T,
    sleep: S,
    extra_headers: Vec<(HeaderName, HeaderValue)>,
}

impl<T: Transport> Client<T, NoSleep> {
    /// Starts configuring a client with `transport` and no retries.
    pub fn builder(transport: T) -> ClientBuilder<T, NoSleep> {
        ClientBuilder::new(transport)
    }
}

#[cfg(any(feature = "reqwest", feature = "native-tls"))]
impl Client<ReqwestTransport, TokioSleep> {
    /// Starts configuring a native client using reqwest and Tokio.
    ///
    /// Use this when an application runs on Tokio and does not need to
    /// customize reqwest's client configuration.
    ///
    /// ```no_run
    /// use jevrs::{Client, Error};
    ///
    /// # fn configured() -> Result<(), Error> {
    /// let client = Client::reqwest().from_env()?.build()?;
    /// # let _ = client;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg_attr(docsrs, doc(cfg(feature = "reqwest")))]
    #[must_use]
    pub fn reqwest() -> ClientBuilder<ReqwestTransport, TokioSleep> {
        Client::<ReqwestTransport>::builder(ReqwestTransport::default()).sleep(TokioSleep)
    }
}

impl<T: Transport, S: Sleep> Client<T, S> {
    /// Asks a statically declared question set using the configured model.
    ///
    /// The returned [`Answered`] value exposes generated fields directly while
    /// retaining model and token-usage metadata.
    ///
    /// ```no_run
    /// use jevrs::{Client, Noul, Questions};
    ///
    /// #[derive(Questions)]
    /// struct Triage {
    ///     #[jev(noul = "Does this convey urgency?")]
    ///     is_urgent: Noul,
    /// }
    ///
    /// # async fn run() -> Result<(), jevrs::Error> {
    /// let client = Client::reqwest().from_env()?.build()?;
    /// let triage = client
    ///     .ask::<Triage>(&"Help! My payouts have been failing for 3 days.")
    ///     .await?;
    /// println!("urgent probability: {:.2}", triage.is_urgent.p.get());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns a question-building, encoding, transport, HTTP, or decoding
    /// [`Error`].
    pub async fn ask<Q: QuestionSet>(&self, state: &impl Serialize) -> Result<Answered<Q>, Error> {
        self.ask_with::<Q>(&self.model, state).await
    }

    /// Asks a statically declared question set with a model override.
    ///
    /// # Errors
    ///
    /// Returns a question-building, encoding, transport, HTTP, or decoding
    /// [`Error`].
    pub async fn ask_with<Q: QuestionSet>(
        &self,
        model: &Model,
        state: &impl Serialize,
    ) -> Result<Answered<Q>, Error> {
        let (questions, handles) = Q::questions()?;
        let answers = self.evaluate_with(model, state, &questions).await?;
        Ok(Answered::from_answers(&handles, &answers))
    }

    /// Evaluates `questions` using the client's configured model.
    ///
    /// # Errors
    ///
    /// Returns an encoding, transport, HTTP, or response-decoding [`Error`].
    pub async fn evaluate(
        &self,
        state: &impl Serialize,
        questions: &Questions,
    ) -> Result<Answers, Error> {
        self.evaluate_with(&self.model, state, questions).await
    }

    /// Evaluates `questions` with a model that overrides the client default.
    ///
    /// # Errors
    ///
    /// Returns an encoding, transport, HTTP, or response-decoding [`Error`].
    pub async fn evaluate_with(
        &self,
        model: &Model,
        state: &impl Serialize,
        questions: &Questions,
    ) -> Result<Answers, Error> {
        let request_body = encode(model, state, questions)?;
        let response_body = self.send(Method::POST, EVALUATE_PATH, request_body).await?;
        decode(questions, &response_body)
    }

    /// Lists models currently advertised by the API.
    ///
    /// # Errors
    ///
    /// Returns a transport, HTTP, or JSON-decoding [`Error`].
    pub async fn models(&self) -> Result<Vec<ModelInfo>, Error> {
        let body = self.send(Method::GET, MODELS_PATH, Vec::new()).await?;
        let response: ModelsResponse = serde_json::from_slice(&body)?;
        Ok(response.models)
    }

    async fn send(&self, method: Method, path: &str, body: Vec<u8>) -> Result<Vec<u8>, Error> {
        let mut attempt = 0;
        loop {
            let request = self.request(method.clone(), path, body.clone())?;
            match self.transport.send(request).await {
                Ok(response) if response.status().is_success() => return Ok(response.into_body()),
                Ok(response) => {
                    let error = classify_response(&response);
                    let retry_after = match &error {
                        Error::RateLimited { retry_after, .. } => *retry_after,
                        Error::Overloaded { .. } => None,
                        _ => return Err(error),
                    };
                    if attempt >= self.retry.max_retries {
                        return Err(error);
                    }
                    self.sleep
                        .sleep(self.retry.delay_for(attempt, retry_after))
                        .await;
                }
                Err(error) => {
                    if attempt >= self.retry.max_retries || !T::is_retryable(&error) {
                        return Err(Error::Transport(Box::new(error)));
                    }
                    self.sleep.sleep(self.retry.delay_for(attempt, None)).await;
                }
            }
            attempt += 1;
        }
    }

    fn request(
        &self,
        method: Method,
        path: &str,
        body: Vec<u8>,
    ) -> Result<Request<Vec<u8>>, Error> {
        let authorization = HeaderValue::from_str(&format!("Bearer {}", self.api_key))
            .map_err(|error| config(error.to_string()))?;
        let mut builder = Request::builder()
            .method(method)
            .uri(format!("{}{path}", self.base_url))
            .header(AUTHORIZATION, authorization)
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json")
            .header(USER_AGENT, concat!("jevrs/", env!("CARGO_PKG_VERSION")));
        for (name, value) in &self.extra_headers {
            builder = builder.header(name, value);
        }
        builder
            .body(body)
            .map_err(|error| config(error.to_string()))
    }
}

impl<T: Transport, S: Sleep> fmt::Debug for Client<T, S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Client")
            .field("base_url", &self.base_url)
            .field("api_key", &"***")
            .field("model", &self.model)
            .field("retry", &self.retry)
            .field("extra_header_count", &self.extra_headers.len())
            .finish_non_exhaustive()
    }
}

/// Configures a [`Client`] without coupling it to a particular runtime.
///
/// Supplying a [`Sleep`] implementation enables the configured retry policy;
/// the default [`NoSleep`] forces retries off.
///
/// ```no_run
/// use jevrs::{Client, ClientBuilder, Error, Model, Transport};
///
/// fn configured<T: Transport>(transport: T) -> Result<Client<T>, Error> {
///     let builder: ClientBuilder<T> = Client::builder(transport);
///     builder.from_env()?.model(Model::PREVIEW).build()
/// }
/// ```
pub struct ClientBuilder<T: Transport, S: Sleep = NoSleep> {
    base_url: String,
    api_key: Option<String>,
    model: Model,
    retry: RetryPolicy,
    transport: T,
    sleep: S,
    extra_headers: Vec<(HeaderName, HeaderValue)>,
}

impl<T: Transport> ClientBuilder<T, NoSleep> {
    fn new(transport: T) -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.into(),
            api_key: None,
            model: Model::default(),
            retry: RetryPolicy::default(),
            transport,
            sleep: NoSleep,
            extra_headers: Vec::new(),
        }
    }
}

impl<T: Transport, S: Sleep> ClientBuilder<T, S> {
    /// Sets the bearer token used for every request.
    #[must_use]
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Loads the API key and optional base URL overrides from the environment.
    ///
    /// `TYPESAFE_BASE_URL` takes precedence over `TYPESAFE_API_BASE`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Auth`] when `TYPESAFE_API_KEY` is unavailable.
    pub fn from_env(mut self) -> Result<Self, Error> {
        let api_key = env::var("TYPESAFE_API_KEY")
            .map_err(|_| missing_api_key("TYPESAFE_API_KEY is not set"))?;
        self.api_key = Some(api_key);
        if let Ok(base_url) = env::var("TYPESAFE_BASE_URL") {
            self.base_url = base_url;
        } else if let Ok(base_url) = env::var("TYPESAFE_API_BASE") {
            self.base_url = base_url;
        }
        Ok(self)
    }

    /// Overrides the API origin. One trailing slash is tolerated.
    #[must_use]
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Sets the model used by [`Client::evaluate`].
    #[must_use]
    pub fn model(mut self, model: Model) -> Self {
        self.model = model;
        self
    }

    /// Replaces the client's retry policy.
    #[must_use]
    pub fn retry(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    /// Supplies the timer used between retry attempts.
    ///
    /// A client built with [`NoSleep`] always has zero retries.
    #[must_use]
    pub fn sleep<S2: Sleep>(self, sleep: S2) -> ClientBuilder<T, S2> {
        ClientBuilder {
            base_url: self.base_url,
            api_key: self.api_key,
            model: self.model,
            retry: self.retry,
            transport: self.transport,
            sleep,
            extra_headers: self.extra_headers,
        }
    }

    /// Appends an HTTP header after the client's standard headers.
    #[must_use]
    pub fn header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.extra_headers.push((name, value));
        self
    }

    /// Validates the configuration and creates a client.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Auth`] when no API key was configured, or
    /// [`Error::Config`] when the base URL cannot form a valid endpoint URI.
    pub fn build(mut self) -> Result<Client<T, S>, Error> {
        let api_key = self
            .api_key
            .take()
            .ok_or_else(|| missing_api_key("TYPESAFE_API_KEY was not configured"))?;
        if let Some(stripped) = self.base_url.strip_suffix('/') {
            self.base_url = stripped.into();
        }
        format!("{}{}", self.base_url, EVALUATE_PATH)
            .parse::<http::Uri>()
            .map_err(|error| config(error.to_string()))?;
        if !S::RETRIES_ENABLED {
            self.retry.max_retries = 0;
        }
        Ok(Client {
            base_url: self.base_url,
            api_key,
            model: self.model,
            retry: self.retry,
            transport: self.transport,
            sleep: self.sleep,
            extra_headers: self.extra_headers,
        })
    }
}

impl<T: Transport, S: Sleep> fmt::Debug for ClientBuilder<T, S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ClientBuilder")
            .field("base_url", &self.base_url)
            .field("api_key", &"***")
            .field("model", &self.model)
            .field("retry", &self.retry)
            .field("extra_header_count", &self.extra_headers.len())
            .finish_non_exhaustive()
    }
}

#[derive(Deserialize)]
struct ModelsResponse {
    models: Vec<ModelInfo>,
}

fn classify_response(response: &Response<Vec<u8>>) -> Error {
    let status = response.status().as_u16();
    let retry_after = response
        .headers()
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs);
    let request_id = response
        .headers()
        .get("x-typesafe-request-id")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    classify(status, response.body(), retry_after, request_id.as_deref())
}

fn missing_api_key(message: &str) -> Error {
    Error::Auth {
        status: StatusCode::UNAUTHORIZED.as_u16(),
        detail: ErrorDetail {
            error_type: Some("configuration_error".into()),
            message: Some(message.into()),
            request_id: None,
            raw: String::new(),
        },
    }
}

fn config(reason: String) -> Error {
    Error::Config { reason }
}

#[cfg(all(test, feature = "test-util"))]
mod tests {
    use std::time::Duration;

    use http::{HeaderName, HeaderValue, Method, Response, header};
    use jevrs_core::{
        Answers, DynChoiceAnswer, DynChoiceQ, DynScoreAnswer, DynScoreQ, Handle, Model, NoulAnswer,
        NoulQ, QuestionSet, Questions,
    };
    use serde_json::{Value, json};

    use super::Client;
    use crate::{
        MockError, MockSleep, MockTransport, RetryPolicy,
        test_support::{block_on, fixtures},
    };

    const KEY: &str = "super-secret-key";

    fn fixture_body(name: &str) -> Vec<u8> {
        serde_json::to_vec(&fixtures::load(name, "response")["body"]).unwrap()
    }

    fn response(status: u16, body: &[u8]) -> Response<Vec<u8>> {
        Response::builder()
            .status(status)
            .header("x-typesafe-request-id", "req_deadbeef")
            .body(body.to_vec())
            .unwrap()
    }

    fn fixed_retry() -> RetryPolicy {
        RetryPolicy {
            jitter: false,
            ..RetryPolicy::default()
        }
    }

    #[test]
    fn evaluate_frames_method_url_headers_and_body() {
        let transport = MockTransport::new([Ok(response(200, &fixture_body("triage")))]);
        let recorder = transport.clone();
        let client = Client::builder(transport)
            .api_key(KEY)
            .base_url("https://example.test")
            .header(
                HeaderName::from_static("x-extra"),
                HeaderValue::from_static("last"),
            )
            .build()
            .unwrap();
        let (questions, _, _, _) = fixtures::triage();

        block_on(client.evaluate(&fixtures::STATE, &questions)).unwrap();

        let mut requests = recorder.take_requests();
        let request = requests.pop().unwrap();
        assert_eq!(request.method(), Method::POST);
        assert_eq!(request.uri(), "https://example.test/v1/systemone");
        assert_eq!(request.headers().len(), 5);
        assert_eq!(
            request.headers()[header::AUTHORIZATION],
            "Bearer super-secret-key"
        );
        assert_eq!(request.headers()[header::CONTENT_TYPE], "application/json");
        assert_eq!(request.headers()[header::ACCEPT], "application/json");
        assert_eq!(request.headers()[header::USER_AGENT], "jevrs/0.0.0");
        assert_eq!(request.headers()["x-extra"], "last");
        let body: Value = serde_json::from_slice(request.body()).unwrap();
        assert_eq!(body["state"], fixtures::STATE);
        assert_eq!(body["model"], "jev-latest");
        assert_eq!(body["questions"].as_object().unwrap().len(), 3);
    }

    struct Triage;

    struct TriageHandles {
        is_urgent: Handle<NoulQ>,
        department: Handle<DynChoiceQ>,
        frustration: Handle<DynScoreQ>,
    }

    struct TriageAnswers {
        is_urgent: NoulAnswer,
        department: DynChoiceAnswer,
        frustration: DynScoreAnswer,
    }

    impl QuestionSet for Triage {
        type Handles = TriageHandles;
        type Answers = TriageAnswers;

        fn questions() -> Result<(Questions, Self::Handles), jevrs_core::Error> {
            let (questions, is_urgent, department, frustration) = fixtures::triage();
            Ok((
                questions,
                TriageHandles {
                    is_urgent,
                    department,
                    frustration,
                },
            ))
        }

        fn answers(handles: &Self::Handles, answers: &Answers) -> Self::Answers {
            TriageAnswers {
                is_urgent: Clone::clone(answers.get(handles.is_urgent)),
                department: Clone::clone(answers.get(handles.department)),
                frustration: Clone::clone(answers.get(handles.frustration)),
            }
        }
    }

    #[test]
    fn ask_sends_derived_shape_and_returns_typed_result() {
        let transport = MockTransport::new([Ok(response(200, &fixture_body("triage")))]);
        let recorder = transport.clone();
        let client = Client::builder(transport).api_key(KEY).build().unwrap();

        let triage = block_on(client.ask::<Triage>(&fixtures::STATE)).unwrap();

        let request = recorder.take_requests().pop().unwrap();
        let body: Value = serde_json::from_slice(request.body()).unwrap();
        assert_eq!(body, fixtures::load("triage", "request"));
        assert!((triage.is_urgent.p.get() - 0.95).abs() < f64::EPSILON);
        assert_eq!(triage.department.pick, "billing");
        assert_eq!(triage.frustration.nearest(), 1);
        assert_eq!(triage.model(), "jev-1.13.0");
        assert_eq!(triage.usage().input_tokens, 414);
    }

    #[test]
    fn models_frames_method_url_and_full_headers() {
        let transport = MockTransport::new([Ok(response(200, &fixture_body("models")))]);
        let recorder = transport.clone();
        let client = Client::builder(transport)
            .api_key(KEY)
            .base_url("https://example.test")
            .build()
            .unwrap();

        block_on(client.models()).unwrap();

        let request = recorder.take_requests().pop().unwrap();
        assert_eq!(request.method(), Method::GET);
        assert_eq!(request.uri(), "https://example.test/v1/models");
        assert!(request.body().is_empty());
        assert_eq!(request.headers().len(), 4);
        assert_eq!(
            request.headers()[header::AUTHORIZATION],
            "Bearer super-secret-key"
        );
        assert_eq!(request.headers()[header::CONTENT_TYPE], "application/json");
        assert_eq!(request.headers()[header::ACCEPT], "application/json");
        assert_eq!(request.headers()[header::USER_AGENT], "jevrs/0.0.0");
    }

    #[test]
    fn base_url_accepts_one_trailing_slash() {
        for base_url in ["https://example.test", "https://example.test/"] {
            let transport = MockTransport::new([Ok(response(200, &fixture_body("models")))]);
            let recorder = transport.clone();
            let client = Client::builder(transport)
                .api_key(KEY)
                .base_url(base_url)
                .build()
                .unwrap();

            block_on(client.models()).unwrap();

            assert_eq!(
                recorder.take_requests().pop().unwrap().uri(),
                "https://example.test/v1/models"
            );
        }
    }

    #[test]
    fn malformed_base_url_is_a_configuration_error() {
        let error = Client::builder(MockTransport::default())
            .api_key(KEY)
            .base_url("not a url")
            .build()
            .unwrap_err();

        assert!(matches!(error, jevrs_core::Error::Config { .. }));
    }

    #[test]
    fn invalid_authorization_value_is_a_configuration_error() {
        let client = Client::builder(MockTransport::default())
            .api_key("bad\nkey")
            .build()
            .unwrap();
        let (questions, _, _, _) = fixtures::triage();

        let error = block_on(client.evaluate(&fixtures::STATE, &questions)).unwrap_err();

        assert!(matches!(error, jevrs_core::Error::Config { .. }));
    }

    #[test]
    fn evaluate_with_overrides_the_body_model() {
        let transport = MockTransport::new([Ok(response(200, &fixture_body("triage")))]);
        let recorder = transport.clone();
        let client = Client::builder(transport).api_key(KEY).build().unwrap();
        let (questions, _, _, _) = fixtures::triage();

        block_on(client.evaluate_with(&Model::PREVIEW, &fixtures::STATE, &questions)).unwrap();

        let request = recorder.take_requests().pop().unwrap();
        let body: Value = serde_json::from_slice(request.body()).unwrap();
        assert_eq!(body["model"], "jev-preview");
    }

    #[test]
    fn recorded_triage_pair_round_trips() {
        let transport = MockTransport::new([Ok(response(200, &fixture_body("triage")))]);
        let client = Client::builder(transport).api_key(KEY).build().unwrap();
        let (questions, urgent, department, frustration) = fixtures::triage();

        let answers = block_on(client.evaluate(&fixtures::STATE, &questions)).unwrap();

        assert_eq!(answers.model(), "jev-1.13.0");
        assert!(answers[urgent].is_yes(0.9));
        assert_eq!(answers[department].pick, "billing");
        assert!((0.0..=2.0).contains(&answers[frustration].expected()));
        assert_eq!(answers.usage().input_tokens, 414);
    }

    #[test]
    fn rate_limit_retries_once_with_policy_delay() {
        let transport = MockTransport::new([
            Ok(response(429, br#"{"detail":"slow down"}"#)),
            Ok(response(200, &fixture_body("triage"))),
        ]);
        let recorder = transport.clone();
        let sleep = MockSleep::default();
        let sleep_recorder = sleep.clone();
        let policy = fixed_retry();
        let client = Client::builder(transport)
            .api_key(KEY)
            .retry(policy)
            .sleep(sleep)
            .build()
            .unwrap();
        let (questions, _, _, _) = fixtures::triage();

        block_on(client.evaluate(&fixtures::STATE, &questions)).unwrap();

        assert_eq!(recorder.take_requests().len(), 2);
        assert_eq!(sleep_recorder.take_durations(), [policy.delay_for(0, None)]);
    }

    #[test]
    fn retry_after_integer_seconds_takes_precedence() {
        let limited = Response::builder()
            .status(429)
            .header("retry-after", "2")
            .body(br#"{"detail":"slow down"}"#.to_vec())
            .unwrap();
        let transport =
            MockTransport::new([Ok(limited), Ok(response(200, &fixture_body("triage")))]);
        let sleep = MockSleep::default();
        let sleep_recorder = sleep.clone();
        let client = Client::builder(transport)
            .api_key(KEY)
            .sleep(sleep)
            .build()
            .unwrap();
        let (questions, _, _, _) = fixtures::triage();

        block_on(client.evaluate(&fixtures::STATE, &questions)).unwrap();

        assert_eq!(sleep_recorder.take_durations(), [Duration::from_secs(2)]);
    }

    #[test]
    fn overload_exhausts_retries() {
        let transport = MockTransport::new([
            Ok(response(529, br#"{"detail":"busy"}"#)),
            Ok(response(529, br#"{"detail":"busy"}"#)),
            Ok(response(529, br#"{"detail":"busy"}"#)),
        ]);
        let recorder = transport.clone();
        let sleep = MockSleep::default();
        let sleep_recorder = sleep.clone();
        let client = Client::builder(transport)
            .api_key(KEY)
            .retry(fixed_retry())
            .sleep(sleep)
            .build()
            .unwrap();
        let (questions, _, _, _) = fixtures::triage();

        let error = block_on(client.evaluate(&fixtures::STATE, &questions)).unwrap_err();

        assert!(matches!(error, jevrs_core::Error::Overloaded { .. }));
        assert_eq!(recorder.take_requests().len(), 3);
        assert_eq!(
            sleep_recorder.take_durations(),
            [Duration::from_millis(500), Duration::from_secs(1)]
        );
    }

    #[test]
    fn authentication_failure_does_not_retry() {
        let transport = MockTransport::new([
            Ok(response(401, br#"{"detail":"bad key"}"#)),
            Ok(response(200, &fixture_body("triage"))),
        ]);
        let recorder = transport.clone();
        let sleep = MockSleep::default();
        let sleep_recorder = sleep.clone();
        let client = Client::builder(transport)
            .api_key(KEY)
            .sleep(sleep)
            .build()
            .unwrap();
        let (questions, _, _, _) = fixtures::triage();

        let error = block_on(client.evaluate(&fixtures::STATE, &questions)).unwrap_err();

        let jevrs_core::Error::Auth { detail, .. } = error else {
            panic!("expected authentication error");
        };
        assert_eq!(detail.request_id.as_deref(), Some("req_deadbeef"));
        assert_eq!(recorder.take_requests().len(), 1);
        assert!(sleep_recorder.take_durations().is_empty());
    }

    #[test]
    fn transport_retryability_controls_retries() {
        let retrying = MockTransport::new([
            Err(MockError { retryable: true }),
            Ok(response(200, &fixture_body("triage"))),
        ]);
        let retrying_recorder = retrying.clone();
        let client = Client::builder(retrying)
            .api_key(KEY)
            .sleep(MockSleep::default())
            .build()
            .unwrap();
        let (questions, _, _, _) = fixtures::triage();
        block_on(client.evaluate(&fixtures::STATE, &questions)).unwrap();
        assert_eq!(retrying_recorder.take_requests().len(), 2);

        let failing = MockTransport::new([
            Err(MockError { retryable: false }),
            Ok(response(200, &fixture_body("triage"))),
        ]);
        let failing_recorder = failing.clone();
        let client = Client::builder(failing)
            .api_key(KEY)
            .sleep(MockSleep::default())
            .build()
            .unwrap();
        let (questions, _, _, _) = fixtures::triage();
        let error = block_on(client.evaluate(&fixtures::STATE, &questions)).unwrap_err();
        assert!(matches!(error, jevrs_core::Error::Transport(_)));
        assert_eq!(failing_recorder.take_requests().len(), 1);
    }

    #[test]
    fn no_sleep_never_retries() {
        let transport = MockTransport::new([
            Ok(response(429, br#"{"detail":"slow down"}"#)),
            Ok(response(200, &fixture_body("triage"))),
        ]);
        let recorder = transport.clone();
        let client = Client::builder(transport)
            .api_key(KEY)
            .retry(fixed_retry())
            .build()
            .unwrap();
        let (questions, _, _, _) = fixtures::triage();

        let error = block_on(client.evaluate(&fixtures::STATE, &questions)).unwrap_err();

        assert!(matches!(error, jevrs_core::Error::RateLimited { .. }));
        assert_eq!(recorder.take_requests().len(), 1);
    }

    #[test]
    fn debug_redacts_api_key() {
        let builder = Client::builder(MockTransport::default()).api_key(KEY);
        let builder_debug = format!("{builder:?}");
        assert!(builder_debug.contains("***"));
        assert!(!builder_debug.contains(KEY));

        let client = builder.build().unwrap();
        let client_debug = format!("{client:?}");
        assert!(client_debug.contains("***"));
        assert!(!client_debug.contains(KEY));
    }

    #[test]
    fn models_parse_release_date_and_unknown_fields() {
        let mut body = fixtures::load("models", "response")["body"].clone();
        body["models"][0]["future_field"] = json!(true);
        let transport =
            MockTransport::new([Ok(response(200, &serde_json::to_vec(&body).unwrap()))]);
        let client = Client::builder(transport).api_key(KEY).build().unwrap();

        let models = block_on(client.models()).unwrap();

        assert_eq!(models[0].name, "jev-latest");
        assert_eq!(
            models[0].release_date.as_deref(),
            Some("2026-09-10T18:38:01.391457+00:00")
        );
        assert_eq!(models[0].extra["future_field"], json!(true));
    }
}
