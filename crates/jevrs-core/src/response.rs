use alloc::{
    boxed::Box,
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::any::Any;

use serde::Deserialize;

use crate::{
    ChoiceAnswer, Confidence, DynChoiceAnswer, DynScoreAnswer, Error, Levels, Model, NoulAnswer,
    Options, Probability, ScoreAnswer, Usage, builder::Criteria,
};

pub(crate) type AnswerSlot = Box<dyn Any>;
pub(crate) type Decoder = fn(&str, Option<&Criteria>, WireAnswer) -> Result<AnswerSlot, Error>;

#[allow(dead_code)]
#[derive(Deserialize)]
pub(crate) struct WireResponse {
    pub(crate) model: Model,
    pub(crate) answers: BTreeMap<String, WireAnswer>,
    pub(crate) usage: Usage,
}

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

impl WireAnswer {
    pub(crate) const fn kind(&self) -> &'static str {
        match self {
            Self::Noul { .. } => "noul",
            Self::Choice { .. } => "choice",
            Self::Score { .. } => "score",
            Self::Unknown => "unknown",
        }
    }
}

fn protocol(id: &str, reason: String) -> Error {
    Error::Protocol {
        id: Some(id.into()),
        reason,
    }
}

fn wrong_type(id: &str, expected: &str, answer: &WireAnswer) -> Error {
    protocol(
        id,
        format!("expected {expected} answer, got {}", answer.kind()),
    )
}

fn choice_keys<'a>(id: &str, criteria: Option<&'a Criteria>) -> Result<Vec<&'a str>, Error> {
    match criteria {
        Some(Criteria::Map(criteria)) => {
            Ok(criteria.0.iter().map(|(key, _)| key.as_str()).collect())
        }
        _ => Err(protocol(id, "choice criteria are unavailable".into())),
    }
}

fn score_levels<'a>(id: &str, criteria: Option<&'a Criteria>) -> Result<&'a [String], Error> {
    match criteria {
        Some(Criteria::Levels(levels)) => Ok(levels),
        _ => Err(protocol(id, "score criteria are unavailable".into())),
    }
}

fn validate_key_set<T>(
    id: &str,
    field: &str,
    expected: &[&str],
    actual: &BTreeMap<String, T>,
) -> Result<(), Error> {
    for key in expected {
        if !actual.contains_key(*key) {
            return Err(protocol(id, format!("{field} missing key {key:?}")));
        }
    }
    for key in actual.keys() {
        if !expected.contains(&key.as_str()) {
            return Err(protocol(id, format!("{field} has unexpected key {key:?}")));
        }
    }
    Ok(())
}

fn ordered_probabilities(
    id: &str,
    expected: &[&str],
    probabilities: &BTreeMap<String, Probability>,
) -> Result<Vec<Probability>, Error> {
    let ordered = expected
        .iter()
        .map(|key| {
            probabilities
                .get(*key)
                .copied()
                .ok_or_else(|| protocol(id, format!("probabilities missing key {key:?}")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if probabilities.len() != expected.len()
        && let Some(key) = probabilities
            .keys()
            .find(|key| !expected.contains(&key.as_str()))
    {
        return Err(protocol(
            id,
            format!("probabilities has unexpected key {key:?}"),
        ));
    }
    Ok(ordered)
}

pub(crate) fn decode_noul(
    id: &str,
    _criteria: Option<&Criteria>,
    answer: WireAnswer,
) -> Result<AnswerSlot, Error> {
    match answer {
        WireAnswer::Noul { noul } => Ok(Box::new(NoulAnswer { p: noul })),
        other => Err(wrong_type(id, "noul", &other)),
    }
}

struct ChoiceParts<'a> {
    choice: String,
    keys: Vec<&'a str>,
    ordered: Vec<Probability>,
    confidence: Confidence,
}

fn choice_parts<'a>(
    id: &str,
    criteria: Option<&'a Criteria>,
    answer: WireAnswer,
) -> Result<ChoiceParts<'a>, Error> {
    let WireAnswer::Choice {
        choice,
        probabilities,
        confidence,
    } = answer
    else {
        return Err(wrong_type(id, "choice", &answer));
    };
    let keys = choice_keys(id, criteria)?;
    let ordered = ordered_probabilities(id, &keys, &probabilities)?;
    Ok(ChoiceParts {
        choice,
        keys,
        ordered,
        confidence,
    })
}

fn unknown_choice(id: &str, choice: &str) -> Error {
    protocol(
        id,
        format!("choice key {choice:?} is not in the option set"),
    )
}

pub(crate) fn decode_static_choice<O: Options>(
    id: &str,
    criteria: Option<&Criteria>,
    answer: WireAnswer,
) -> Result<AnswerSlot, Error> {
    let parts = choice_parts(id, criteria, answer)?;
    let Some(pick) = O::from_key(&parts.choice) else {
        return Err(unknown_choice(id, &parts.choice));
    };
    let probs = O::map_from_fn(|option| parts.ordered[option.index()]);
    Ok(Box::new(ChoiceAnswer::<O> {
        pick,
        probs,
        confidence: parts.confidence,
    }))
}

pub(crate) fn decode_dynamic_choice(
    id: &str,
    criteria: Option<&Criteria>,
    answer: WireAnswer,
) -> Result<AnswerSlot, Error> {
    let ChoiceParts {
        choice,
        keys,
        ordered,
        confidence,
    } = choice_parts(id, criteria, answer)?;
    if !keys.contains(&choice.as_str()) {
        return Err(unknown_choice(id, &choice));
    }
    let probs = keys.into_iter().map(String::from).zip(ordered).collect();
    Ok(Box::new(DynChoiceAnswer {
        pick: choice,
        probs,
        confidence,
    }))
}

struct ScoreParts {
    score: f64,
    legend: BTreeMap<String, String>,
    probabilities: BTreeMap<String, Probability>,
    confidence: Confidence,
}

fn score_parts(id: &str, answer: WireAnswer) -> Result<ScoreParts, Error> {
    let WireAnswer::Score {
        score,
        legend,
        probabilities,
        confidence,
    } = answer
    else {
        return Err(wrong_type(id, "score", &answer));
    };
    Ok(ScoreParts {
        score,
        legend,
        probabilities,
        confidence,
    })
}

#[allow(clippy::cast_precision_loss)]
fn decode_score_parts(
    id: &str,
    criteria: Option<&Criteria>,
    answer: WireAnswer,
) -> Result<(ScoreParts, Vec<String>, Vec<Probability>), Error> {
    let parts = score_parts(id, answer)?;
    let levels = score_levels(id, criteria)?;
    let keys = (0..levels.len())
        .map(|index| index.to_string())
        .collect::<Vec<_>>();
    let key_refs = keys.iter().map(String::as_str).collect::<Vec<_>>();
    let probabilities = ordered_probabilities(id, &key_refs, &parts.probabilities)?;
    validate_key_set(id, "legend", &key_refs, &parts.legend)?;
    let maximum = levels.len().saturating_sub(1) as f64;
    if !(0.0..=maximum).contains(&parts.score) {
        return Err(protocol(
            id,
            format!("score {} is outside 0..={maximum}", parts.score),
        ));
    }
    Ok((parts, keys, probabilities))
}

pub(crate) fn decode_static_score<L: Levels>(
    id: &str,
    criteria: Option<&Criteria>,
    answer: WireAnswer,
) -> Result<AnswerSlot, Error> {
    let (parts, keys, ordered) = decode_score_parts(id, criteria, answer)?;
    for (level, key) in L::all().iter().zip(&keys) {
        let Some(actual) = parts.legend.get(key) else {
            return Err(protocol(id, format!("legend missing key {key:?}")));
        };
        let expected = level.description();
        if actual != expected {
            return Err(protocol(
                id,
                format!("legend key {key:?} expected {expected:?}, got {actual:?}"),
            ));
        }
    }
    let probs = L::map_from_fn(|level| ordered[level.index()]);
    Ok(Box::new(ScoreAnswer::<L>::new(
        parts.score,
        probs,
        parts.confidence,
    )))
}

pub(crate) fn decode_dynamic_score(
    id: &str,
    criteria: Option<&Criteria>,
    answer: WireAnswer,
) -> Result<AnswerSlot, Error> {
    let (parts, keys, probabilities) = decode_score_parts(id, criteria, answer)?;
    let legend = keys
        .iter()
        .map(|key| {
            parts
                .legend
                .get(key)
                .cloned()
                .ok_or_else(|| protocol(id, format!("legend missing key {key:?}")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Box::new(DynScoreAnswer::new(
        parts.score,
        probabilities,
        legend,
        parts.confidence,
    )))
}

#[cfg(test)]
mod tests {
    use alloc::{
        format,
        string::{String, ToString},
    };

    use super::{WireAnswer, WireResponse};
    use crate::{
        ChoiceAnswer, DynChoiceAnswer, DynLevels, DynOptions, DynScoreAnswer, Error, NoulAnswer,
        Questions, ScoreAnswer,
        test_support::{Dept, Frustration},
    };

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

    fn static_choice_questions() -> Questions {
        let mut questions = Questions::new();
        questions.choice::<Dept>("department", "Choose").unwrap();
        questions
    }

    fn dynamic_choice_questions() -> Questions {
        let mut questions = Questions::new();
        questions
            .choice_dyn(
                "department",
                "Choose",
                DynOptions::new([
                    ("billing".into(), None),
                    ("technical".into(), None),
                    ("sales".into(), None),
                ])
                .unwrap(),
            )
            .unwrap();
        questions
    }

    fn static_score_questions() -> Questions {
        let mut questions = Questions::new();
        questions
            .score::<Frustration>("frustration", "Rate")
            .unwrap();
        questions
    }

    fn dynamic_score_questions() -> Questions {
        let mut questions = Questions::new();
        questions
            .score_dyn(
                "frustration",
                "Rate",
                DynLevels::new(["Calm".into(), "Frustrated".into(), "Very angry".into()]).unwrap(),
            )
            .unwrap();
        questions
    }

    fn protocol_reason(questions: &Questions, json: &str) -> String {
        let answer: WireAnswer = serde_json::from_str(json).unwrap();
        let error = questions.entries[0].decode(answer).err().unwrap();
        match error {
            Error::Protocol { id, reason } => {
                assert_eq!(id.as_deref(), Some(questions.entries[0].id.as_str()));
                reason
            }
            other => panic!("expected protocol error, got {other:?}"),
        }
    }

    fn assert_choice_reason(json: &str, expected: &str) {
        for questions in [static_choice_questions(), dynamic_choice_questions()] {
            assert_eq!(protocol_reason(&questions, json), expected);
        }
    }

    fn assert_score_reason(json: &str, expected: &str) {
        for questions in [static_score_questions(), dynamic_score_questions()] {
            assert_eq!(protocol_reason(&questions, json), expected);
        }
    }

    #[test]
    fn every_question_kind_decodes_its_happy_path() {
        let mut questions = Questions::new();
        questions.noul("noul", "Urgent?").unwrap();
        questions.choice::<Dept>("static_choice", "Choose").unwrap();
        questions
            .score::<Frustration>("static_score", "Rate")
            .unwrap();
        questions
            .choice_dyn(
                "dynamic_choice",
                "Choose",
                DynOptions::new([("first".into(), None), ("second".into(), None)]).unwrap(),
            )
            .unwrap();
        questions
            .score_dyn(
                "dynamic_score",
                "Rate",
                DynLevels::new(["Low".into(), "High".into()]).unwrap(),
            )
            .unwrap();

        let noul = questions.entries[0]
            .decode(serde_json::from_str(r#"{"type":"noul","noul":0.95}"#).unwrap())
            .unwrap()
            .downcast::<NoulAnswer>()
            .unwrap();
        assert!((noul.p.get() - 0.95).abs() < f64::EPSILON);

        let choice = questions.entries[1]
            .decode(serde_json::from_str(
                r#"{"type":"choice","choice":"billing","confidence":0.79,"probabilities":{"billing":0.86,"technical":0.14,"sales":0.0}}"#,
            ).unwrap())
            .unwrap()
            .downcast::<ChoiceAnswer<Dept>>()
            .unwrap();
        assert_eq!(choice.pick, Dept::Billing);
        assert!((choice.probs[Dept::Billing].get() - 0.86).abs() < f64::EPSILON);

        let score = questions.entries[2]
            .decode(serde_json::from_str(
                r#"{"type":"score","score":1.05,"confidence":0.93,"legend":{"0":"Calm","1":"Frustrated","2":"Very angry"},"probabilities":{"0":0.0,"1":0.95,"2":0.05}}"#,
            ).unwrap())
            .unwrap()
            .downcast::<ScoreAnswer<Frustration>>()
            .unwrap();
        assert_eq!(score.nearest(), Frustration::Frustrated);

        let choice = questions.entries[3]
            .decode(serde_json::from_str(
                r#"{"type":"choice","choice":"second","confidence":0.8,"probabilities":{"first":0.2,"second":0.8}}"#,
            ).unwrap())
            .unwrap()
            .downcast::<DynChoiceAnswer>()
            .unwrap();
        assert_eq!(choice.pick, "second");
        assert_eq!(choice.probs[0].0, "first");

        let score = questions.entries[4]
            .decode(serde_json::from_str(
                r#"{"type":"score","score":0.75,"confidence":0.9,"legend":{"0":"Server low","1":"Server high"},"probabilities":{"0":0.25,"1":0.75}}"#,
            ).unwrap())
            .unwrap()
            .downcast::<DynScoreAnswer>()
            .unwrap();
        assert_eq!(score.legend, ["Server low", "Server high"]);
    }

    #[test]
    fn mismatched_answer_types_name_the_expected_and_actual_types() {
        let mut noul = Questions::new();
        noul.noul("urgent", "Urgent?").unwrap();
        assert_eq!(
            protocol_reason(
                &noul,
                r#"{"type":"choice","choice":"x","confidence":0.5,"probabilities":{"x":1.0}}"#
            ),
            "expected noul answer, got choice"
        );
        assert_choice_reason(
            r#"{"type":"score","score":0.0,"confidence":0.5,"legend":{"0":"x"},"probabilities":{"0":1.0}}"#,
            "expected choice answer, got score",
        );
        assert_score_reason(
            r#"{"type":"ranking","ranking":[]}"#,
            "expected score answer, got unknown",
        );
    }

    #[test]
    fn choice_probabilities_report_a_missing_key() {
        assert_choice_reason(
            r#"{"type":"choice","choice":"billing","confidence":0.8,"probabilities":{"billing":0.8,"technical":0.2}}"#,
            "probabilities missing key \"sales\"",
        );
    }

    #[test]
    fn choice_probabilities_report_an_extra_key() {
        assert_choice_reason(
            r#"{"type":"choice","choice":"billing","confidence":0.8,"probabilities":{"billing":0.8,"technical":0.1,"sales":0.1,"other":0.0}}"#,
            "probabilities has unexpected key \"other\"",
        );
    }

    #[test]
    fn choices_report_an_unknown_selected_key() {
        assert_choice_reason(
            r#"{"type":"choice","choice":"other","confidence":0.8,"probabilities":{"billing":0.8,"technical":0.1,"sales":0.1}}"#,
            "choice key \"other\" is not in the option set",
        );
    }

    #[test]
    fn score_probabilities_report_a_missing_key() {
        assert_score_reason(
            r#"{"type":"score","score":1.0,"confidence":0.9,"legend":{"0":"Calm","1":"Frustrated","2":"Very angry"},"probabilities":{"0":0.0,"1":1.0}}"#,
            "probabilities missing key \"2\"",
        );
    }

    #[test]
    fn score_probabilities_report_an_extra_key() {
        assert_score_reason(
            r#"{"type":"score","score":1.0,"confidence":0.9,"legend":{"0":"Calm","1":"Frustrated","2":"Very angry"},"probabilities":{"0":0.0,"1":1.0,"2":0.0,"3":0.0}}"#,
            "probabilities has unexpected key \"3\"",
        );
    }

    #[test]
    fn score_legends_report_a_missing_key() {
        assert_score_reason(
            r#"{"type":"score","score":1.0,"confidence":0.9,"legend":{"0":"Calm","1":"Frustrated"},"probabilities":{"0":0.0,"1":1.0,"2":0.0}}"#,
            "legend missing key \"2\"",
        );
    }

    #[test]
    fn score_legends_report_an_extra_key() {
        assert_score_reason(
            r#"{"type":"score","score":1.0,"confidence":0.9,"legend":{"0":"Calm","1":"Frustrated","2":"Very angry","3":"Other"},"probabilities":{"0":0.0,"1":1.0,"2":0.0}}"#,
            "legend has unexpected key \"3\"",
        );
    }

    #[test]
    fn static_score_legends_report_mismatched_text() {
        assert_eq!(
            protocol_reason(
                &static_score_questions(),
                r#"{"type":"score","score":1.0,"confidence":0.9,"legend":{"0":"Calm","1":"Annoyed","2":"Very angry"},"probabilities":{"0":0.0,"1":1.0,"2":0.0}}"#,
            ),
            "legend key \"1\" expected \"Frustrated\", got \"Annoyed\""
        );
    }

    #[test]
    fn scores_report_values_outside_the_scale() {
        assert_score_reason(
            r#"{"type":"score","score":3.0,"confidence":0.9,"legend":{"0":"Calm","1":"Frustrated","2":"Very angry"},"probabilities":{"0":0.0,"1":1.0,"2":0.0}}"#,
            "score 3 is outside 0..=2",
        );
    }

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
