use alloc::{string::String, vec, vec::Vec};
use core::{
    fmt,
    marker::PhantomData,
    sync::atomic::{AtomicU64, Ordering},
};

use serde::{
    Serialize, Serializer,
    ser::{SerializeMap, SerializeStruct},
};

use crate::{
    ChoiceQ, DynChoiceQ, DynLevels, DynOptions, DynScoreQ, Error, Instructions, Levels, Model,
    NoulQ, Options, Question, ScoreQ,
    response::{
        AnswerSlot, Decoder, WireAnswer, decode_dynamic_choice, decode_dynamic_score, decode_noul,
        decode_static_choice, decode_static_score,
    },
};

static NEXT_BATCH_ID: AtomicU64 = AtomicU64::new(1);

/// Identifies the [`Questions`] batch that created a typed [`Handle`].
///
/// You normally see this only in [`Handle`]'s debug output or a mismatched
/// handle panic; [`Answers`](crate::Answers) checks it automatically.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BatchId(u64);

/// A typed reference to one question in a [`Questions`] batch.
///
/// Keep the handle returned by a builder method, then pass it to
/// [`Answers::get`](crate::Answers::get) after evaluation. The marker `Q`
/// selects the answer type at compile time. Passing a handle from another
/// batch is a caller error and panics when the answer is retrieved.
pub struct Handle<Q: Question> {
    pub(crate) idx: u32,
    pub(crate) batch: BatchId,
    _q: PhantomData<Q>,
}

impl<Q: Question> Copy for Handle<Q> {}

impl<Q: Question> Clone for Handle<Q> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Q: Question> fmt::Debug for Handle<Q> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Handle")
            .field("idx", &self.idx)
            .field("batch", &self.batch)
            .finish()
    }
}

/// An insertion-ordered collection of questions sent in one Jev request.
///
/// Use the returned handles to retrieve typed answers after decoding.
///
/// ```
/// use jevrs_core::{DynLevels, DynOptions, Questions};
///
/// let mut questions = Questions::new();
/// let urgent = questions.noul_with(
///     "is_urgent",
///     "Does this convey urgency?",
///     "Explicitly time-sensitive",
///     "No urgency expressed",
/// )?;
/// let department = questions.choice_dyn(
///     "department",
///     "Which team should handle this?",
///     DynOptions::new([
///         ("billing", Some("Payments, invoicing, refunds")),
///         ("technical", Some("Bugs, outages, integrations")),
///         ("sales", None),
///     ])?,
/// )?;
/// let frustration = questions.score_dyn(
///     "frustration",
///     "How frustrated is the customer?",
///     DynLevels::new(["Calm", "Frustrated", "Very angry"])?
/// )?;
///
/// assert_eq!(questions.len(), 3);
/// let _ = (urgent, department, frustration);
/// # Ok::<(), jevrs_core::Error>(())
/// ```
pub struct Questions {
    pub(crate) entries: Vec<QuestionEntry>,
    pub(crate) batch: BatchId,
}

impl Questions {
    /// Starts a batch when questions will be added individually at runtime.
    ///
    /// Use [`QuestionSet`](crate::QuestionSet) instead when the whole batch is
    /// fixed at compile time.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            batch: BatchId(NEXT_BATCH_ID.fetch_add(1, Ordering::Relaxed)),
        }
    }

    /// Adds a yes/no question when the instruction alone defines both outcomes.
    ///
    /// Use [`Self::noul_with`] when explicit yes and no criteria improve the
    /// decision boundary.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DuplicateId`] if `id` is already in this batch.
    pub fn noul(
        &mut self,
        id: impl Into<String>,
        instructions: impl Into<Instructions>,
    ) -> Result<Handle<NoulQ>, Error> {
        self.insert(id.into(), instructions.into(), "noul", None, decode_noul)
    }

    /// Adds a yes/no question when both outcomes need explicit descriptions.
    ///
    /// Use [`Self::noul`] for a simpler instruction-only question.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DuplicateId`] if `id` is already in this batch.
    pub fn noul_with(
        &mut self,
        id: impl Into<String>,
        instructions: impl Into<Instructions>,
        yes: &str,
        no: &str,
    ) -> Result<Handle<NoulQ>, Error> {
        let criteria = ChoiceCriteria(vec![
            ("true".into(), Some(yes.into())),
            ("false".into(), Some(no.into())),
        ]);
        self.insert(
            id.into(),
            instructions.into(),
            "noul",
            Some(Criteria::Map(criteria)),
            decode_noul,
        )
    }

    /// Adds a choice question whose [`Options`] are known at compile time.
    ///
    /// Use [`Self::choice_dyn`] when option keys come from configuration or a
    /// database.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DuplicateId`] if `id` is already in this batch.
    pub fn choice<O: Options>(
        &mut self,
        id: impl Into<String>,
        instructions: impl Into<Instructions>,
    ) -> Result<Handle<ChoiceQ<O>>, Error> {
        let () = O::COUNT_OK;
        let criteria = ChoiceCriteria(
            O::all()
                .iter()
                .map(|option| (option.key().into(), option.description().map(String::from)))
                .collect(),
        );
        self.insert(
            id.into(),
            instructions.into(),
            "choice",
            Some(Criteria::Map(criteria)),
            decode_static_choice::<O>,
        )
    }

    /// Adds a score question whose ordered [`Levels`] are known at compile time.
    ///
    /// Use [`Self::score_dyn`] when level descriptions are runtime data.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DuplicateId`] if `id` is already in this batch.
    pub fn score<L: Levels>(
        &mut self,
        id: impl Into<String>,
        instructions: impl Into<Instructions>,
    ) -> Result<Handle<ScoreQ<L>>, Error> {
        let () = L::COUNT_OK;
        let criteria = L::all()
            .iter()
            .map(|level| level.description().into())
            .collect();
        self.insert(
            id.into(),
            instructions.into(),
            "score",
            Some(Criteria::Levels(criteria)),
            decode_static_score::<L>,
        )
    }

    /// Adds a choice question backed by runtime-defined [`DynOptions`].
    ///
    /// Use this for options loaded from configuration or a database. Prefer
    /// [`Self::choice`] when an enum can represent the choices.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DuplicateId`] if `id` is already in this batch.
    pub fn choice_dyn(
        &mut self,
        id: impl Into<String>,
        instructions: impl Into<Instructions>,
        options: DynOptions,
    ) -> Result<Handle<DynChoiceQ>, Error> {
        let (keys, descriptions) = options.into_parts();
        let criteria = ChoiceCriteria(keys.into_iter().zip(descriptions).collect());
        self.insert(
            id.into(),
            instructions.into(),
            "choice",
            Some(Criteria::Map(criteria)),
            decode_dynamic_choice,
        )
    }

    /// Adds a score question backed by runtime-defined [`DynLevels`].
    ///
    /// Use this for a scale loaded at runtime. Prefer [`Self::score`] when an
    /// enum can represent the ordered levels.
    ///
    /// # Errors
    ///
    /// Returns [`Error::DuplicateId`] if `id` is already in this batch.
    pub fn score_dyn(
        &mut self,
        id: impl Into<String>,
        instructions: impl Into<Instructions>,
        levels: DynLevels,
    ) -> Result<Handle<DynScoreQ>, Error> {
        self.insert(
            id.into(),
            instructions.into(),
            "score",
            Some(Criteria::Levels(levels.into_inner())),
            decode_dynamic_score,
        )
    }

    /// Returns the number of questions, for validating or reporting a batch.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Reports whether [`encode`] would reject this batch as empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[allow(clippy::cast_possible_truncation)]
    fn insert<Q: Question>(
        &mut self,
        id: String,
        instructions: Instructions,
        question_type: &'static str,
        criteria: Option<Criteria>,
        decoder: Decoder,
    ) -> Result<Handle<Q>, Error> {
        if self.entries.iter().any(|entry| entry.id == id) {
            return Err(Error::DuplicateId(id));
        }
        let handle = Handle {
            idx: self.entries.len() as u32,
            batch: self.batch,
            _q: PhantomData,
        };
        self.entries.push(QuestionEntry {
            id,
            instructions,
            question_type,
            criteria,
            decoder,
        });
        Ok(handle)
    }
}

impl Default for Questions {
    fn default() -> Self {
        Self::new()
    }
}

/// Encodes a Jev request body without performing network I/O.
///
/// Use this at the boundary to a custom HTTP implementation. Pair it with
/// [`crate::decode`] for successful responses and [`crate::classify`] for
/// unsuccessful statuses.
///
/// ```
/// use jevrs_core::{Model, Questions, encode};
///
/// let mut questions = Questions::new();
/// questions.noul("is_urgent", "Does this convey urgency?")?;
/// let body = encode(
///     &Model::LATEST,
///     &"Help! My payouts have been failing for 3 days.",
///     &questions,
/// )?;
/// assert!(!body.is_empty());
/// # Ok::<(), jevrs_core::Error>(())
/// ```
///
/// # Errors
///
/// Returns [`Error::NoQuestions`] for an empty builder or [`Error::Json`] if
/// `state` cannot be serialized.
pub fn encode(
    model: &Model,
    state: &impl Serialize,
    questions: &Questions,
) -> Result<Vec<u8>, Error> {
    if questions.is_empty() {
        return Err(Error::NoQuestions);
    }
    serde_json::to_vec(&WireRequest {
        state,
        model,
        questions: WireQuestions(questions),
    })
    .map_err(Error::from)
}

pub(crate) struct WireRequest<'a, S> {
    state: &'a S,
    model: &'a Model,
    questions: WireQuestions<'a>,
}

impl<S: Serialize> Serialize for WireRequest<'_, S> {
    fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: Serializer,
    {
        let mut request = serializer.serialize_struct("Request", 3)?;
        request.serialize_field("state", self.state)?;
        request.serialize_field("model", self.model)?;
        request.serialize_field("questions", &self.questions)?;
        request.end()
    }
}

pub(crate) struct WireQuestions<'a>(&'a Questions);

impl Serialize for WireQuestions<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut questions = serializer.serialize_map(Some(self.0.entries.len()))?;
        for entry in &self.0.entries {
            questions.serialize_entry(&entry.id, &WireQuestion(entry))?;
        }
        questions.end()
    }
}

pub(crate) struct WireQuestion<'a>(&'a QuestionEntry);

impl Serialize for WireQuestion<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let field_count = if self.0.criteria.is_some() { 3 } else { 2 };
        let mut question = serializer.serialize_struct("Question", field_count)?;
        question.serialize_field("type", self.0.question_type)?;
        question.serialize_field("instructions", &self.0.instructions)?;
        if let Some(criteria) = &self.0.criteria {
            question.serialize_field("criteria", criteria)?;
        }
        question.end()
    }
}

pub(crate) struct QuestionEntry {
    pub(crate) id: String,
    instructions: Instructions,
    question_type: &'static str,
    criteria: Option<Criteria>,
    decoder: Decoder,
}

impl QuestionEntry {
    pub(crate) fn decode(&self, answer: WireAnswer) -> Result<AnswerSlot, Error> {
        (self.decoder)(&self.id, self.criteria.as_ref(), answer)
    }
}

pub(crate) enum Criteria {
    Map(ChoiceCriteria),
    Levels(Vec<String>),
}

impl Serialize for Criteria {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Map(criteria) => criteria.serialize(serializer),
            Self::Levels(criteria) => criteria.serialize(serializer),
        }
    }
}

pub(crate) struct ChoiceCriteria(pub(crate) Vec<(String, Option<String>)>);

impl Serialize for ChoiceCriteria {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut criteria = serializer.serialize_map(Some(self.0.len()))?;
        for (key, description) in &self.0 {
            criteria.serialize_entry(key, description)?;
        }
        criteria.end()
    }
}

#[cfg(test)]
mod tests {
    use alloc::{format, string::String};

    use serde_json::{Value, json};

    use super::{Handle, Questions, encode};
    use crate::{
        DynLevels, DynOptions, Error, Model, NoulQ, Question,
        test_support::{Dept, Frustration},
    };

    struct NotDebugQ;

    impl Question for NotDebugQ {
        type Answer = ();
    }

    fn encoded_value(state: &impl serde::Serialize, questions: &Questions) -> Value {
        serde_json::from_slice(&encode(&Model::LATEST, state, questions).unwrap()).unwrap()
    }

    #[test]
    fn triage_request_matches_the_spec() {
        let mut questions = Questions::new();
        questions
            .noul_with(
                "is_urgent",
                "Does this convey urgency?",
                "Explicitly time-sensitive",
                "No urgency expressed",
            )
            .unwrap();
        questions
            .choice::<Dept>("department", "Which team should handle this?")
            .unwrap();
        questions
            .score::<Frustration>("frustration", "How frustrated is the customer?")
            .unwrap();

        assert_eq!(
            encoded_value(
                &"Help! My payouts have been failing for 3 days.",
                &questions
            ),
            json!({
                "state": "Help! My payouts have been failing for 3 days.",
                "model": "jev-latest",
                "questions": {
                    "is_urgent": {
                        "type": "noul",
                        "instructions": "Does this convey urgency?",
                        "criteria": {
                            "true": "Explicitly time-sensitive",
                            "false": "No urgency expressed"
                        }
                    },
                    "department": {
                        "type": "choice",
                        "instructions": "Which team should handle this?",
                        "criteria": {
                            "billing": "Payments, invoicing, refunds",
                            "technical": "Bugs, outages, integrations",
                            "sales": null
                        }
                    },
                    "frustration": {
                        "type": "score",
                        "instructions": "How frustrated is the customer?",
                        "criteria": ["Calm", "Frustrated", "Very angry"]
                    }
                }
            })
        );
    }

    #[test]
    fn noul_omits_or_emits_criteria_as_requested() {
        let mut plain = Questions::new();
        plain.noul("urgent", "Is this urgent?").unwrap();
        assert_eq!(
            encoded_value(&"state", &plain)["questions"]["urgent"],
            json!({"type": "noul", "instructions": "Is this urgent?"})
        );

        let mut described = Questions::new();
        described
            .noul_with("urgent", "Is this urgent?", "Urgent", "Not urgent")
            .unwrap();
        assert_eq!(
            encoded_value(&"state", &described)["questions"]["urgent"]["criteria"],
            json!({"true": "Urgent", "false": "Not urgent"})
        );
    }

    #[test]
    fn structured_instructions_encode_verbatim() {
        let object = json!({"task": "Assess urgency", "signals": ["deadline"]});
        let array = json!(["Choose a team", {"prefer": "billing"}]);
        let mut questions = Questions::new();
        questions.noul("urgent", object.clone()).unwrap();
        questions
            .choice_dyn(
                "department",
                array.clone(),
                DynOptions::new([("billing", None::<&str>), ("sales", None)]).unwrap(),
            )
            .unwrap();

        let encoded = encoded_value(&"state", &questions);
        assert_eq!(encoded["questions"]["urgent"]["instructions"], object);
        assert_eq!(encoded["questions"]["department"]["instructions"], array);
    }

    #[test]
    fn dynamic_questions_encode_their_criteria() {
        let mut questions = Questions::new();
        questions
            .choice_dyn(
                "department",
                "Choose a team",
                DynOptions::new([("billing", Some("Payments")), ("sales", None)]).unwrap(),
            )
            .unwrap();
        questions
            .score_dyn(
                "frustration",
                "Rate frustration",
                DynLevels::new(["Calm", "Angry"]).unwrap(),
            )
            .unwrap();

        let value = encoded_value(&"state", &questions);
        assert_eq!(
            value["questions"]["department"]["criteria"],
            json!({"billing": "Payments", "sales": null})
        );
        assert_eq!(
            value["questions"]["frustration"]["criteria"],
            json!(["Calm", "Angry"])
        );
    }

    #[test]
    fn state_accepts_string_object_and_array() {
        let mut questions = Questions::new();
        questions.noul("urgent", "Is this urgent?").unwrap();

        assert_eq!(encoded_value(&"text", &questions)["state"], json!("text"));
        assert_eq!(
            encoded_value(&json!({"message": "text"}), &questions)["state"],
            json!({"message": "text"})
        );
        assert_eq!(
            encoded_value(&json!(["first", "second"]), &questions)["state"],
            json!(["first", "second"])
        );
    }

    #[test]
    fn duplicate_ids_are_rejected_without_adding_an_entry() {
        let mut questions = Questions::new();
        questions.noul("urgent", "First").unwrap();
        let error = questions.noul("urgent", "Second").err().unwrap();
        assert!(matches!(error, Error::DuplicateId(id) if id == "urgent"));
        assert_eq!(questions.len(), 1);
    }

    #[test]
    fn empty_batches_are_rejected_at_encode_time() {
        assert!(matches!(
            encode(&Model::LATEST, &"state", &Questions::new()),
            Err(Error::NoQuestions)
        ));
    }

    #[test]
    fn builders_have_distinct_batch_ids() {
        let mut first = Questions::new();
        let first_handle = first.noul("first", "First").unwrap();
        let mut second = Questions::new();
        let second_handle = second.noul("second", "Second").unwrap();
        assert_ne!(first_handle.batch, second_handle.batch);
    }

    #[test]
    fn handles_are_copy_and_debug_without_marker_bounds() {
        fn assert_copy<T: Copy>() {}
        fn assert_debug<T: core::fmt::Debug>() {}

        assert_copy::<Handle<NoulQ>>();
        assert_debug::<Handle<NotDebugQ>>();

        let mut questions = Questions::new();
        let handle = questions.noul("urgent", "Is this urgent?").unwrap();
        let debug = format!("{handle:?}");
        assert!(debug.contains("idx: 0"));
        assert!(debug.contains("batch: BatchId("));
    }

    #[test]
    fn choice_criteria_keep_option_order_on_the_wire() {
        let mut questions = Questions::new();
        questions
            .choice::<Dept>("department", "Choose a team")
            .unwrap();
        let json =
            String::from_utf8(encode(&Model::LATEST, &"state", &questions).unwrap()).unwrap();
        let billing = json.find("billing").unwrap();
        let technical = json.find("technical").unwrap();
        let sales = json.find("sales").unwrap();
        assert!(billing < technical && technical < sales);
    }
}
