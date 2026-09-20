use alloc::{borrow::Cow, string::String};
use core::fmt;

use serde::{Deserialize, Deserializer, Serialize};

/// A Jev model name used for requests and reported by responses.
///
/// Use a named constant to follow a release channel, or construct a model from
/// a version string when a request must target a specific release.
///
/// ```
/// use jevrs_core::Model;
///
/// assert_eq!(Model::default(), Model::LATEST);
/// assert_eq!(Model::from("jev-1.13.0").as_ref(), "jev-1.13.0");
/// ```
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[repr(transparent)]
pub struct Model(Cow<'static, str>);

impl Model {
    /// The stable model release channel.
    pub const LATEST: Self = Self(Cow::Borrowed("jev-latest"));

    /// The preview model release channel.
    pub const PREVIEW: Self = Self(Cow::Borrowed("jev-preview"));
}

impl From<&'static str> for Model {
    fn from(value: &'static str) -> Self {
        Self(Cow::Borrowed(value))
    }
}

impl From<String> for Model {
    fn from(value: String) -> Self {
        Self(Cow::Owned(value))
    }
}

impl AsRef<str> for Model {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Model {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl<'de> Deserialize<'de> for Model {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Self::from)
    }
}

impl Default for Model {
    fn default() -> Self {
        Self::LATEST
    }
}

/// Natural-language guidance supplied with a Jev question.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[repr(transparent)]
pub struct Instructions(String);

impl From<&str> for Instructions {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl From<String> for Instructions {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl AsRef<str> for Instructions {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Token counts consumed while producing a Jev response.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Usage {
    /// Tokens consumed by the request input.
    pub input_tokens: u64,
    /// Tokens generated in the response output.
    pub output_tokens: u64,
}

#[cfg(test)]
mod tests {
    use super::{Instructions, Model, Usage};

    #[test]
    fn model_channels_use_wire_names() {
        assert_eq!(Model::LATEST.as_ref(), "jev-latest");
        assert_eq!(Model::PREVIEW.as_ref(), "jev-preview");
    }

    #[test]
    fn concrete_model_round_trips() {
        let model: Model = serde_json::from_str(r#""jev-1.13.0""#).unwrap();
        assert_eq!(model.as_ref(), "jev-1.13.0");
        assert_eq!(serde_json::to_string(&model).unwrap(), r#""jev-1.13.0""#);
    }

    #[test]
    fn recorded_usage_round_trips() {
        let json = r#"{"input_tokens":414,"output_tokens":73}"#;
        let usage: Usage = serde_json::from_str(json).unwrap();
        assert_eq!(usage.input_tokens, 414);
        assert_eq!(usage.output_tokens, 73);
        assert_eq!(serde_json::to_string(&usage).unwrap(), json);
    }

    #[test]
    fn instructions_serialize_as_a_string() {
        let instructions = Instructions::from("Assess urgency");
        assert_eq!(
            serde_json::to_string(&instructions).unwrap(),
            r#""Assess urgency""#
        );
    }
}
