use alloc::{boxed::Box, string::String, string::ToString, vec::Vec};
use core::{fmt, time::Duration};

use serde::Deserialize;

/// An error produced while constructing, sending, or decoding a Jev request.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A request was encoded without any questions.
    #[error("at least one question is required")]
    NoQuestions,
    /// A question identifier was inserted more than once.
    #[error("duplicate question id: {0}")]
    DuplicateId(String),
    /// A question's option or level set is invalid.
    #[error("invalid criteria for {id:?}: {reason}")]
    InvalidCriteria {
        /// The affected question identifier, when one is available.
        id: Option<String>,
        /// The violated criteria rule.
        reason: &'static str,
    },
    /// The client configuration cannot produce a valid request.
    #[error("the client configuration is invalid: {reason}")]
    Config {
        /// The invalid setting and why it cannot be used.
        reason: String,
    },
    /// Authentication failed with HTTP status 401 or 403.
    #[error("authentication failed (HTTP {status}): {detail}")]
    Auth {
        /// The HTTP response status.
        status: u16,
        /// Parsed API error information.
        detail: ErrorDetail,
    },
    /// The API rejected the request with HTTP status 400 or 422.
    #[error("bad request (HTTP {status}): {detail}")]
    BadRequest {
        /// The HTTP response status.
        status: u16,
        /// Parsed API error information.
        detail: ErrorDetail,
    },
    /// The API rate limit was reached.
    #[error("rate limited (HTTP 429): {detail}")]
    RateLimited {
        /// Server-requested delay before another attempt, when supplied.
        retry_after: Option<Duration>,
        /// Parsed API error information.
        detail: ErrorDetail,
    },
    /// The API was overloaded and returned HTTP status 529.
    #[error("service overloaded (HTTP 529): {detail}")]
    Overloaded {
        /// Parsed API error information.
        detail: ErrorDetail,
    },
    /// The API returned another unsuccessful HTTP status.
    #[error("HTTP {status}: {detail}")]
    Http {
        /// The HTTP response status.
        status: u16,
        /// Parsed API error information.
        detail: ErrorDetail,
    },
    /// A response violated the Jev wire protocol.
    #[error("protocol error for {id:?}: {reason}")]
    Protocol {
        /// The affected question identifier, when one is available.
        id: Option<String>,
        /// The protocol violation.
        reason: String,
    },
    /// JSON encoding or decoding failed.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// The underlying transport failed.
    #[error("transport error: {0}")]
    Transport(#[source] Box<dyn core::error::Error + Send + Sync>),
}

/// Information recovered from an unsuccessful API response body.
#[derive(Debug, Eq, PartialEq)]
pub struct ErrorDetail {
    /// The API's machine-readable error category, when present.
    pub error_type: Option<String>,
    /// The API's human-readable explanation, when present.
    pub message: Option<String>,
    /// The `x-typesafe-request-id` response header, when present.
    pub request_id: Option<String>,
    /// The response body decoded lossily as UTF-8.
    pub raw: String,
}

impl ErrorDetail {
    /// Parses any known Jev error body without discarding malformed content.
    ///
    /// ```
    /// let body = br#"{"detail":{"error_type":"api_usage_error","message":"Invalid request."}}"#;
    /// let detail = jevrs_core::ErrorDetail::parse(body, Some("req_123"));
    ///
    /// assert_eq!(detail.error_type.as_deref(), Some("api_usage_error"));
    /// assert_eq!(detail.request_id.as_deref(), Some("req_123"));
    /// ```
    #[must_use]
    pub fn parse(body: &[u8], request_id: Option<&str>) -> Self {
        let raw = String::from_utf8_lossy(body).into_owned();

        if let Ok(body) = serde_json::from_slice::<ObjectBody>(body) {
            return Self::new(raw, request_id, body.detail.error_type, body.detail.message);
        }
        if let Ok(body) = serde_json::from_slice::<StringBody>(body) {
            return Self::new(raw, request_id, None, Some(body.detail));
        }
        if let Ok(body) = serde_json::from_slice::<PydanticBody>(body) {
            return Self::new(raw, request_id, None, Some(body.message()));
        }

        Self::new(raw, request_id, None, None)
    }

    fn new(
        raw: String,
        request_id: Option<&str>,
        error_type: Option<String>,
        message: Option<String>,
    ) -> Self {
        Self {
            error_type,
            message,
            request_id: request_id.map(String::from),
            raw,
        }
    }
}

impl fmt::Display for ErrorDetail {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.error_type, &self.message) {
            (Some(error_type), Some(message)) => write!(formatter, "{error_type}: {message}")?,
            (Some(error_type), None) => error_type.fmt(formatter)?,
            (None, Some(message)) => message.fmt(formatter)?,
            (None, None) => formatter.write_str("unknown API error")?,
        }
        if let Some(request_id) = &self.request_id {
            write!(formatter, " (request_id: {request_id})")?;
        }
        Ok(())
    }
}

/// Classifies an unsuccessful HTTP response and preserves its API details.
///
/// Transports use this after extracting `Retry-After` and
/// `x-typesafe-request-id` headers.
///
/// ```
/// let error = jevrs_core::classify(
///     401,
///     br#"{"detail":{"error_type":"authentication_error","message":"Invalid key"}}"#,
///     None,
///     Some("req_123"),
/// );
/// assert!(matches!(error, jevrs_core::Error::Auth { status: 401, .. }));
/// ```
#[must_use]
pub fn classify(
    status: u16,
    body: &[u8],
    retry_after: Option<Duration>,
    request_id: Option<&str>,
) -> Error {
    let detail = ErrorDetail::parse(body, request_id);
    match status {
        401 | 403 => Error::Auth { status, detail },
        400 | 422 => Error::BadRequest { status, detail },
        429 => Error::RateLimited {
            retry_after,
            detail,
        },
        529 => Error::Overloaded { detail },
        _ => Error::Http { status, detail },
    }
}

#[derive(Deserialize)]
struct ObjectBody {
    detail: ObjectDetail,
}

#[derive(Deserialize)]
struct ObjectDetail {
    error_type: Option<String>,
    message: Option<String>,
}

#[derive(Deserialize)]
struct StringBody {
    detail: String,
}

#[derive(Deserialize)]
struct PydanticBody {
    detail: Vec<PydanticEntry>,
}

impl PydanticBody {
    fn message(self) -> String {
        let mut message = String::new();
        for (entry_index, entry) in self.detail.into_iter().enumerate() {
            if entry_index != 0 {
                message.push_str("; ");
            }
            for (location_index, location) in entry.loc.into_iter().enumerate() {
                if location_index != 0 {
                    message.push('.');
                }
                match location {
                    Location::Text(value) => message.push_str(&value),
                    Location::Index(value) => message.push_str(&value.to_string()),
                }
            }
            message.push_str(": ");
            message.push_str(&entry.msg);
        }
        message
    }
}

#[derive(Deserialize)]
struct PydanticEntry {
    loc: Vec<Location>,
    msg: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Location {
    Text(String),
    Index(u64),
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;
    use core::time::Duration;

    use super::{Error, ErrorDetail, classify};

    mod fixtures {
        pub const BAD_REQUEST_OBJECT: &str =
            r#"{"detail":{"error_type":"api_usage_error","message":"Invalid request."}}"#;
        pub const CHOICE_EMPTY: &str =
            r#"{"detail":"Choice question must have at least one choice: q"}"#;
        pub const CHOICE_TOO_LARGE: &str =
            r#"{"detail":"Too many choices. Must have at most 255 choices."}"#;
        pub const SCORE_TOO_LARGE: &str =
            r#"{"detail":"Too many score levels. Must have at most 10 levels."}"#;
        pub const NOUL_EMPTY: &str =
            r#"{"detail":"Noul question must have criteria or instructions: q"}"#;
        pub const INVALID_KEY: &str = r#"{"detail":{"error_type":"authentication_error","message":"Cannot authenticate with the server. Please check your API key and try again."}}"#;
        pub const MISSING_KEY: &str = r#"{"detail":{"error_type":"authentication_error","message":"Must supply an API key! Check your request and try again."}}"#;
        pub const EMPTY_QUESTIONS: &str = r#"{"detail":[{"type":"too_short","loc":["body","questions"],"msg":"Dictionary should have at least 1 item after validation, not 0","input":{},"ctx":{"field_type":"Dictionary","min_length":1,"actual_length":0}}]}"#;
        pub const EMPTY_SCORE: &str = r#"{"detail":[{"type":"too_short","loc":["body","questions","q","score","criteria"],"msg":"List should have at least 1 item after validation, not 0","input":[],"ctx":{"field_type":"List","min_length":1,"actual_length":0}}]}"#;
        pub const MISSING_MODEL: &str = r#"{"detail":[{"type":"missing","loc":["body","model"],"msg":"Field required","input":{"state":"x","questions":{"q":{"type":"noul","instructions":"is it?"}}}}]}"#;
        pub const NOT_FOUND: &str = r#"{"detail":"Not Found"}"#;
    }

    fn assert_detail(
        detail: &ErrorDetail,
        body: &str,
        error_type: Option<&str>,
        message: Option<&str>,
    ) {
        assert_eq!(detail.error_type.as_deref(), error_type);
        assert_eq!(detail.message.as_deref(), message);
        assert_eq!(detail.request_id.as_deref(), Some("req_deadbeef"));
        assert_eq!(detail.raw, body);
    }

    fn assert_http_parts(
        error: &Error,
        status: u16,
        detail: &ErrorDetail,
        body: &str,
        error_type: Option<&str>,
        message: Option<&str>,
    ) {
        assert_detail(detail, body, error_type, message);
        let displayed = error.to_string();
        assert!(displayed.contains(&status.to_string()));
        if let Some(error_type) = error_type {
            assert!(displayed.contains(error_type));
        }
        if let Some(message) = message {
            assert!(displayed.contains(message));
        }
        assert!(displayed.contains("req_deadbeef"));
    }

    fn assert_bad_request(status: u16, body: &str, error_type: Option<&str>, message: &str) {
        let error = classify(status, body.as_bytes(), None, Some("req_deadbeef"));
        let Error::BadRequest { status, detail } = &error else {
            panic!("expected BadRequest, got {error:?}");
        };
        assert_http_parts(&error, *status, detail, body, error_type, Some(message));
    }

    fn assert_auth(status: u16, body: &str, message: &str) {
        let error = classify(status, body.as_bytes(), None, Some("req_deadbeef"));
        let Error::Auth { status, detail } = &error else {
            panic!("expected Auth, got {error:?}");
        };
        assert_http_parts(
            &error,
            *status,
            detail,
            body,
            Some("authentication_error"),
            Some(message),
        );
    }

    fn assert_http(status: u16, body: &str, error_type: Option<&str>, message: Option<&str>) {
        let error = classify(status, body.as_bytes(), None, Some("req_deadbeef"));
        let Error::Http { status, detail } = &error else {
            panic!("expected Http, got {error:?}");
        };
        assert_http_parts(&error, *status, detail, body, error_type, message);
    }

    #[test]
    fn config_error_identifies_the_invalid_setting() {
        let error = Error::Config {
            reason: "base URL is malformed".into(),
        };

        assert_eq!(
            error.to_string(),
            "the client configuration is invalid: base URL is malformed"
        );
    }

    #[test]
    fn classifies_400_object() {
        assert_bad_request(
            400,
            fixtures::BAD_REQUEST_OBJECT,
            Some("api_usage_error"),
            "Invalid request.",
        );
    }

    #[test]
    fn classifies_400_empty_choice() {
        assert_bad_request(
            400,
            fixtures::CHOICE_EMPTY,
            None,
            "Choice question must have at least one choice: q",
        );
    }

    #[test]
    fn classifies_400_too_many_choices() {
        assert_bad_request(
            400,
            fixtures::CHOICE_TOO_LARGE,
            None,
            "Too many choices. Must have at most 255 choices.",
        );
    }

    #[test]
    fn classifies_400_too_many_score_levels() {
        assert_bad_request(
            400,
            fixtures::SCORE_TOO_LARGE,
            None,
            "Too many score levels. Must have at most 10 levels.",
        );
    }

    #[test]
    fn classifies_400_empty_noul() {
        assert_bad_request(
            400,
            fixtures::NOUL_EMPTY,
            None,
            "Noul question must have criteria or instructions: q",
        );
    }

    #[test]
    fn classifies_401_invalid_key() {
        assert_auth(
            401,
            fixtures::INVALID_KEY,
            "Cannot authenticate with the server. Please check your API key and try again.",
        );
    }

    #[test]
    fn classifies_403_missing_key() {
        assert_auth(
            403,
            fixtures::MISSING_KEY,
            "Must supply an API key! Check your request and try again.",
        );
    }

    #[test]
    fn classifies_422_empty_questions() {
        assert_bad_request(
            422,
            fixtures::EMPTY_QUESTIONS,
            None,
            "body.questions: Dictionary should have at least 1 item after validation, not 0",
        );
    }

    #[test]
    fn classifies_422_empty_score_criteria() {
        assert_bad_request(
            422,
            fixtures::EMPTY_SCORE,
            None,
            "body.questions.q.score.criteria: List should have at least 1 item after validation, not 0",
        );
    }

    #[test]
    fn classifies_422_missing_model() {
        assert_bad_request(
            422,
            fixtures::MISSING_MODEL,
            None,
            "body.model: Field required",
        );
    }

    #[test]
    fn classifies_404_string() {
        assert_http(404, fixtures::NOT_FOUND, None, Some("Not Found"));
    }

    #[test]
    fn classifies_529_with_an_empty_body() {
        let error = classify(529, b"", None, Some("req_deadbeef"));
        let Error::Overloaded { detail } = &error else {
            panic!("expected Overloaded, got {error:?}");
        };
        assert_http_parts(&error, 529, detail, "", None, None);
    }

    #[test]
    fn classifies_429_with_retry_after() {
        let retry_after = Duration::from_secs(2);
        let error = classify(
            429,
            fixtures::CHOICE_TOO_LARGE.as_bytes(),
            Some(retry_after),
            Some("req_deadbeef"),
        );
        let Error::RateLimited {
            retry_after: actual_retry_after,
            detail,
        } = &error
        else {
            panic!("expected RateLimited, got {error:?}");
        };
        assert_eq!(*actual_retry_after, Some(retry_after));
        assert_http_parts(
            &error,
            429,
            detail,
            fixtures::CHOICE_TOO_LARGE,
            None,
            Some("Too many choices. Must have at most 255 choices."),
        );
    }

    #[test]
    fn classifies_500_with_a_non_json_body() {
        assert_http(500, "upstream exploded", None, None);
    }

    #[test]
    fn preserves_valid_json_with_an_unknown_shape() {
        let body = r#"{"detail":42}"#;
        assert_http(500, body, None, None);
    }

    #[test]
    fn joins_multiple_pydantic_entries() {
        let body = br#"{"detail":[{"loc":["body","questions",0],"msg":"first"},{"loc":["body","model"],"msg":"second"}]}"#;
        let detail = ErrorDetail::parse(body, None);
        assert_eq!(
            detail.message.as_deref(),
            Some("body.questions.0: first; body.model: second")
        );
    }

    #[test]
    fn non_utf8_body_is_preserved_lossily() {
        let detail = ErrorDetail::parse(&[0xff], None);
        assert_eq!(detail.raw, "�");
        assert_eq!(detail.error_type, None);
        assert_eq!(detail.message, None);
    }
}
