use alloc::{collections::BTreeMap, string::String};

use serde::Deserialize;

use crate::{Confidence, Model, Probability, Usage};

#[allow(dead_code)]
#[derive(Deserialize)]
pub(crate) struct WireResponse {
    pub(crate) model: Model,
    pub(crate) answers: BTreeMap<String, WireAnswer>,
    pub(crate) usage: Usage,
}

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(tag = "type")]
pub(crate) enum WireAnswer {
    #[serde(rename = "noul")]
    Noul { noul: Probability },
    #[serde(rename = "choice")]
    Choice {
        choice: String,
        probabilities: BTreeMap<String, Probability>,
        confidence: Confidence,
    },
    #[serde(rename = "score")]
    Score {
        score: f64,
        legend: BTreeMap<String, String>,
        probabilities: BTreeMap<String, Probability>,
        confidence: Confidence,
    },
    #[serde(other)]
    Unknown,
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use super::{WireAnswer, WireResponse};

    const RECORDED_RESPONSE: &[u8] = br#"{
      "model": "jev-1.13.0",
      "answers": {
        "is_urgent": { "type": "noul", "noul": 0.95 },
        "department": { "type": "choice", "choice": "billing", "confidence": 0.79,
          "probabilities": { "billing": 0.86, "technical": 0.14, "sales": 0.0 } },
        "frustration": { "type": "score", "score": 1.05, "confidence": 0.93,
          "legend": { "0": "Calm", "1": "Frustrated", "2": "Very angry" },
          "probabilities": { "0": 0.0, "1": 0.95, "2": 0.05 } }
      },
      "usage": { "input_tokens": 414, "output_tokens": 73 }
    }"#;

    #[test]
    fn recorded_response_parses() {
        let response: WireResponse = serde_json::from_slice(RECORDED_RESPONSE).unwrap();
        assert_eq!(response.model.as_ref(), "jev-1.13.0");
        assert_eq!(response.answers.len(), 3);
        assert_eq!(response.usage.input_tokens, 414);
        assert!(matches!(
            response.answers["is_urgent"],
            WireAnswer::Noul { noul } if (noul.get() - 0.95).abs() < f64::EPSILON
        ));
    }

    #[test]
    fn unknown_answer_type_uses_the_catch_all() {
        let answer: WireAnswer =
            serde_json::from_str(r#"{"type":"ranking","ranking":["first"]}"#).unwrap();
        assert!(matches!(answer, WireAnswer::Unknown));
    }

    #[test]
    fn out_of_range_probability_is_rejected() {
        let error = serde_json::from_str::<WireAnswer>(r#"{"type":"noul","noul":1.2}"#)
            .err()
            .unwrap();
        assert!(error.to_string().contains("invalid Probability value 1.2"));
    }
}
