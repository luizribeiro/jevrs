use alloc::{borrow::Cow, string::String};
use core::fmt;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::Error;

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
    /// Use the stable release channel when you want automatic compatible upgrades.
    pub const LATEST: Self = Self(Cow::Borrowed("jev-latest"));

    /// Use the preview channel when evaluating changes before stable release.
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

/// Guidance supplied with a Jev question as a JSON value.
///
/// String instructions can be constructed with [`From`]. Use [`Self::json`]
/// to serialize structured application data, or [`Self::from`] when a
/// [`Value`] is already available.
///
/// ```
/// use jevrs_core::Instructions;
/// use serde_json::json;
///
/// let instructions = Instructions::json(&json!({
///     "task": "Assess urgency",
///     "signals": ["deadline", "outage"],
/// }))?;
/// assert!(instructions.as_value().is_object());
/// # Ok::<(), jevrs_core::Error>(())
/// ```
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[repr(transparent)]
pub struct Instructions(Value);

impl Instructions {
    /// Serializes structured instructions when a string cannot express the schema.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Json`] if `value` cannot be represented as JSON.
    pub fn json(value: &impl Serialize) -> Result<Self, Error> {
        Ok(Self(serde_json::to_value(value)?))
    }

    /// Borrows text when a caller only handles string instructions.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        self.0.as_str()
    }

    /// Borrows the JSON value when a caller supports every instruction shape.
    #[must_use]
    pub const fn as_value(&self) -> &Value {
        &self.0
    }
}

impl From<&str> for Instructions {
    fn from(value: &str) -> Self {
        Self(Value::String(value.into()))
    }
}

impl From<String> for Instructions {
    fn from(value: String) -> Self {
        Self(Value::String(value))
    }
}

impl From<Value> for Instructions {
    fn from(value: Value) -> Self {
        Self(value)
    }
}

/// Token counts consumed while producing a Jev response.
///
/// Use these values for observability and cost accounting. They are available
/// from both [`crate::Answers::usage`] and [`crate::Answered::usage`].
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Usage {
    /// Use this count to track state and question input consumption.
    pub input_tokens: u64,
    /// Use this count to track answer output consumption.
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
    fn string_instructions_encode_verbatim() {
        let instructions = Instructions::from("Assess urgency");
        assert_eq!(instructions.as_str(), Some("Assess urgency"));
        assert_eq!(
            serde_json::to_string(&instructions).unwrap(),
            r#""Assess urgency""#
        );
    }

    #[test]
    fn object_instructions_encode_verbatim() {
        let value = serde_json::json!({
            "task": "Assess urgency",
            "signals": ["deadline", "outage"]
        });
        let instructions = Instructions::json(&value).unwrap();

        assert_eq!(instructions.as_str(), None);
        assert_eq!(instructions.as_value(), &value);
        assert_eq!(serde_json::to_value(instructions).unwrap(), value);
    }

    #[test]
    fn array_instructions_encode_verbatim() {
        let value = serde_json::json!(["Assess urgency", {"weight": 2}]);
        let instructions = Instructions::from(value.clone());

        assert_eq!(instructions.as_value(), &value);
        assert_eq!(serde_json::to_value(instructions).unwrap(), value);
    }
}
