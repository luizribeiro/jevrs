#![allow(missing_docs)]
#![deny(dead_code)]

use jevrs::{Answered, Choice, Noul, QuestionSet, Score, Usage, decode, encode};
use jevrs_derive::{Levels, Options, Questions};

#[path = "../../../tests/fixtures/mod.rs"]
mod fixtures;

#[derive(Clone, Copy, Debug, Eq, Options, PartialEq)]
pub enum Dept {
    /// Payments, invoicing, refunds
    Billing,
    /// Bugs, outages, integrations
    Technical,
    Sales,
}

#[derive(Clone, Copy, Debug, Eq, Levels, Ord, PartialEq, PartialOrd)]
pub enum Frustration {
    /// Calm
    Calm,
    /// Frustrated
    Frustrated,
    /// Very angry
    VeryAngry,
}

#[derive(Questions)]
pub struct Triage {
    #[jev(
        noul = "Does this convey urgency?",
        yes = "Explicitly time-sensitive",
        no = "No urgency expressed"
    )]
    is_urgent: Noul,
    #[jev(choice = "Which team should handle this?")]
    department: Choice<Dept>,
    #[jev(score = "How frustrated is the customer?", id = "frustration")]
    frustration_score: Score<Frustration>,
}

#[test]
fn derives_triage_request_and_typed_answers() {
    let (questions, handles) = Triage::questions().unwrap();
    let encoded = encode(&jevrs::Model::LATEST, &fixtures::STATE, &questions).unwrap();
    let actual: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    let expected = fixtures::load("triage", "request");
    assert_eq!(actual, expected);
    assert!(actual["questions"]["is_urgent"]["instructions"].is_string());
    assert!(actual["questions"]["department"]["instructions"].is_string());
    assert!(actual["questions"]["frustration"]["instructions"].is_string());

    let fixture = fixtures::load("triage", "response");
    let body = serde_json::to_vec(&fixture["body"]).unwrap();
    let raw = decode(&questions, &body).unwrap();
    let t = Answered::<Triage>::from_answers(&handles, &raw);

    assert!((t.is_urgent.p.get() - 0.95).abs() < f64::EPSILON);
    assert_eq!(t.department.pick, Dept::Billing);
    assert_eq!(t.frustration_score.nearest(), Frustration::Frustrated);
    assert_eq!(t.model(), "jev-1.13.0");
    assert_eq!(
        t.usage(),
        Usage {
            input_tokens: 414,
            output_tokens: 73,
        }
    );

    let handles_debug = format!("{handles:?}");
    let answers_debug = format!("{:?}", t.answers);
    assert!(handles_debug.contains("TriageHandles"));
    assert!(answers_debug.contains("TriageAnswers"));
    let _cloned = (handles.clone(), t.answers.clone());
}

mod core_path {
    use jevrs_core::{Noul, QuestionSet};
    use jevrs_derive::Questions;

    #[derive(Questions)]
    #[jev(crate = "::jevrs_core")]
    pub struct CoreQuestions {
        #[jev(noul = "Is this urgent?")]
        urgent: Noul,
    }

    #[test]
    fn overrides_the_emitted_crate_path() {
        assert_eq!(CoreQuestions::questions().unwrap().0.len(), 1);
    }
}

mod restricted_visibility {
    use jevrs::{Choice, QuestionSet};
    use jevrs_derive::{Options, Questions};

    #[derive(Clone, Copy, Debug, Eq, Options, PartialEq)]
    pub(crate) enum Department {
        /// Payments
        Billing,
        Sales,
    }

    #[derive(Questions)]
    pub(crate) struct Triage {
        #[jev(choice = "Which team should handle this?")]
        department: Choice<Department>,
    }

    #[test]
    fn preserves_restricted_visibility() {
        assert_eq!(Triage::questions().unwrap().0.len(), 1);
    }
}
