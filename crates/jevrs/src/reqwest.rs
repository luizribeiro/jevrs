use core::time::Duration;

use http::{Request, Response};

use crate::{Sleep, Transport};

/// Sends Jev requests with reqwest's native HTTP client.
///
/// Use this transport to share a configured [`reqwest::Client`] with `jevrs`.
/// Pair it with [`TokioSleep`] when retries should wait between attempts.
/// [`ReqwestTransport::new`] selects native TLS when the `native-tls` feature
/// is enabled and rustls otherwise. Native TLS wins when both features are on.
///
/// ```no_run
/// use jevrs::{Client, Error, ReqwestTransport, TokioSleep};
///
/// # fn configured() -> Result<(), Error> {
/// let transport = ReqwestTransport::from(reqwest::Client::new());
/// let client = Client::builder(transport)
///     .sleep(TokioSleep)
///     .from_env()?
///     .build()?;
/// # let _ = client;
/// # Ok(())
/// # }
/// ```
#[cfg_attr(docsrs, doc(cfg(any(feature = "reqwest", feature = "native-tls"))))]
#[derive(Clone, Debug)]
pub struct ReqwestTransport(reqwest::Client);

impl ReqwestTransport {
    /// Creates a transport when TLS initialization failure must be handled.
    ///
    /// Native TLS wins when both `reqwest` and `native-tls` are enabled.
    ///
    /// # Errors
    ///
    /// Returns an error when reqwest cannot initialize the selected backend.
    pub fn new() -> Result<Self, reqwest::Error> {
        let builder = reqwest::Client::builder();
        #[cfg(feature = "native-tls")]
        let builder = builder.use_native_tls();
        #[cfg(all(not(feature = "native-tls"), feature = "reqwest"))]
        let builder = builder.use_rustls_tls();
        builder.build().map(Self)
    }
}

impl Default for ReqwestTransport {
    /// Creates a transport when infallible startup is acceptable.
    ///
    /// # Panics
    ///
    /// Panics when reqwest cannot initialize the selected backend. This mirrors
    /// [`reqwest::Client::new`]'s own contract; use [`ReqwestTransport::new`]
    /// when initialization failure must be handled.
    fn default() -> Self {
        Self::new().expect("reqwest client initialization failed")
    }
}

impl From<reqwest::Client> for ReqwestTransport {
    fn from(client: reqwest::Client) -> Self {
        Self(client)
    }
}

impl Transport for ReqwestTransport {
    type Error = reqwest::Error;

    async fn send(&self, request: Request<Vec<u8>>) -> Result<Response<Vec<u8>>, Self::Error> {
        let request = reqwest::Request::try_from(request)?;
        let response = self.0.execute(request).await?;
        into_http_response(response).await
    }

    fn is_retryable(error: &Self::Error) -> bool {
        error.is_connect() || error.is_timeout()
    }
}

/// Waits between retry attempts with Tokio's timer.
///
/// Use this sleeper with [`ReqwestTransport`] in applications running on a
/// Tokio runtime.
///
/// ```no_run
/// use jevrs::{Client, ReqwestTransport, TokioSleep};
///
/// let builder = Client::builder(ReqwestTransport::default())
///     .sleep(TokioSleep);
/// # let _ = builder;
/// ```
#[cfg_attr(docsrs, doc(cfg(any(feature = "reqwest", feature = "native-tls"))))]
#[derive(Clone, Copy, Debug, Default)]
pub struct TokioSleep;

impl Sleep for TokioSleep {
    async fn sleep(&self, duration: Duration) {
        tokio::time::sleep(duration).await;
    }
}

async fn into_http_response(
    response: reqwest::Response,
) -> Result<Response<Vec<u8>>, reqwest::Error> {
    let status = response.status();
    let version = response.version();
    let headers = response.headers().clone();
    let body = response.bytes().await?.to_vec();
    let mut response = Response::new(body);
    *response.status_mut() = status;
    *response.version_mut() = version;
    *response.headers_mut() = headers;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use http::{Method, Request, Response, Version, header};

    use super::{ReqwestTransport, into_http_response};
    use crate::Transport;

    #[test]
    fn request_conversion_preserves_method_url_headers_and_body() {
        let request = Request::builder()
            .method(Method::POST)
            .uri("https://example.test/v1/systemone?mode=fast")
            .header(header::CONTENT_TYPE, "application/json")
            .body(br#"{"state":"triage"}"#.to_vec())
            .unwrap();

        let request = reqwest::Request::try_from(request).unwrap();

        assert_eq!(request.method(), Method::POST);
        assert_eq!(
            request.url().as_str(),
            "https://example.test/v1/systemone?mode=fast"
        );
        assert_eq!(request.headers()[header::CONTENT_TYPE], "application/json");
        assert_eq!(
            request.body().and_then(reqwest::Body::as_bytes),
            Some(br#"{"state":"triage"}"#.as_slice())
        );
    }

    #[tokio::test]
    async fn response_conversion_preserves_status_version_headers_and_body() {
        let response = Response::builder()
            .status(202)
            .version(Version::HTTP_2)
            .header(header::CONTENT_TYPE, "application/json")
            .body(reqwest::Body::from(br#"{"accepted":true}"#.as_slice()))
            .unwrap();

        let response = into_http_response(reqwest::Response::from(response))
            .await
            .unwrap();

        assert_eq!(response.status(), 202);
        assert_eq!(response.version(), Version::HTTP_2);
        assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
        assert_eq!(response.body(), br#"{"accepted":true}"#);
    }

    #[test]
    fn request_builder_errors_are_not_retryable() {
        let error = reqwest::Client::new().get("not a URL").build().unwrap_err();

        assert!(!ReqwestTransport::is_retryable(&error));
    }

    #[tokio::test]
    async fn connect_errors_are_retryable() {
        let request = Request::get("http://127.0.0.1:1/")
            .body(Vec::new())
            .unwrap();
        let error = ReqwestTransport::default().send(request).await.unwrap_err();

        assert!(ReqwestTransport::is_retryable(&error));
    }
}
