#![allow(missing_docs)]

use jevrs_core::{
    Answered, Answers, ArrayMap, Choice, ChoiceAnswer, Error, Handle, Indexed, Levels, Noul,
    NoulAnswer, Options, QuestionSet, Questions, Score, ScoreAnswer, Usage, decode,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Dept {
    Billing,
    Technical,
    Sales,
}

impl Indexed for Dept {
    fn all() -> &'static [Self] {
        &[Self::Billing, Self::Technical, Self::Sales]
    }

    fn index(self) -> usize {
        self as usize
    }
}

impl Options for Dept {
    const N: usize = 3;
    type Map<T: Send + Sync + 'static> = ArrayMap<Self, T, 3>;

    fn key(self) -> &'static str {
        match self {
            Self::Billing => "billing",
            Self::Technical => "technical",
            Self::Sales => "sales",
        }
    }

    fn description(self) -> Option<&'static str> {
        match self {
            Self::Billing => Some("Payments, invoicing, refunds"),
            Self::Technical => Some("Bugs, outages, integrations"),
            Self::Sales => None,
        }
    }

    fn from_key(key: &str) -> Option<Self> {
        Self::all().iter().copied().find(|value| value.key() == key)
    }

    fn map_from_fn<T: Send + Sync + 'static>(mut f: impl FnMut(Self) -> T) -> Self::Map<T> {
        ArrayMap::new([f(Self::Billing), f(Self::Technical), f(Self::Sales)])
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Frustration {
    Calm,
    Frustrated,
    VeryAngry,
}

impl Indexed for Frustration {
    fn all() -> &'static [Self] {
        &[Self::Calm, Self::Frustrated, Self::VeryAngry]
    }

    fn index(self) -> usize {
        self as usize
    }
}

impl Levels for Frustration {
    const N: usize = 3;
    type Map<T: Send + Sync + 'static> = ArrayMap<Self, T, 3>;

    fn description(self) -> &'static str {
        match self {
            Self::Calm => "Calm",
            Self::Frustrated => "Frustrated",
            Self::VeryAngry => "Very angry",
        }
    }

    fn from_index(index: usize) -> Option<Self> {
        Self::all().get(index).copied()
    }

    fn map_from_fn<T: Send + Sync + 'static>(mut f: impl FnMut(Self) -> T) -> Self::Map<T> {
        ArrayMap::new([f(Self::Calm), f(Self::Frustrated), f(Self::VeryAngry)])
    }
}

struct Triage;

struct TriageHandles {
    is_urgent: Handle<Noul>,
    department: Handle<Choice<Dept>>,
    frustration: Handle<Score<Frustration>>,
}

#[derive(Clone, Debug)]
struct TriageAnswers {
    is_urgent: NoulAnswer,
    department: ChoiceAnswer<Dept>,
    frustration: ScoreAnswer<Frustration>,
}

impl QuestionSet for Triage {
    type Handles = TriageHandles;
    type Answers = TriageAnswers;

    fn questions() -> Result<(Questions, Self::Handles), Error> {
        let mut questions = Questions::new();
        let is_urgent = questions.noul_with(
            "is_urgent",
            "Does this convey urgency?",
            "Explicitly time-sensitive",
            "No urgency expressed",
        )?;
        let department = questions.choice("department", "Which team should handle this?")?;
        let frustration = questions.score("frustration", "How frustrated is the customer?")?;
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
fn handwritten_question_set_decodes_recorded_triage_response() {
    let (questions, handles) = Triage::questions().unwrap();
    let fixture: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/triage.response.json"
    ))
    .unwrap();
    let body = serde_json::to_vec(&fixture["body"]).unwrap();
    let decoded = decode(&questions, &body).unwrap();
    let t = Answered::<Triage>::from_answers(&handles, &decoded);

    assert!((t.is_urgent.p.get() - 0.95).abs() < f64::EPSILON);
    assert_eq!(t.department.pick, Dept::Billing);
    assert_eq!(t.frustration.nearest(), Frustration::Frustrated);
    assert_eq!(t.model(), "jev-1.13.0");
    assert_eq!(
        t.usage(),
        Usage {
            input_tokens: 414,
            output_tokens: 73,
        }
    );

    let cloned = t.clone();
    assert_eq!(cloned.department.pick, Dept::Billing);
    let debug = format!("{t:?}");
    assert!(debug.contains("Answered"));
    assert!(debug.contains("TriageAnswers"));
    assert!(debug.contains("jev-1.13.0"));
}
