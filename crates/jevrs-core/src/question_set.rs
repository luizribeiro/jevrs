use alloc::string::{String, ToString};
use core::{fmt, ops::Deref};

use crate::{Answers, Error, Questions, Usage};

/// Defines a reusable, statically typed set of Jev questions.
///
/// Implement this trait by hand when generated question sets are not suitable.
/// Most applications use the `Questions` derive re-exported by `jevrs`.
///
/// ```
/// use jevrs_core::{Answers, Error, Handle, Noul, NoulAnswer, QuestionSet, Questions};
///
/// struct Urgency;
/// struct UrgencyHandles { urgent: Handle<Noul> }
/// struct UrgencyAnswers { urgent: NoulAnswer }
///
/// impl QuestionSet for Urgency {
///     type Handles = UrgencyHandles;
///     type Answers = UrgencyAnswers;
///
///     fn questions() -> Result<(Questions, Self::Handles), Error> {
///         let mut questions = Questions::new();
///         let urgent = questions.noul("urgent", "Does this convey urgency?")?;
///         Ok((questions, UrgencyHandles { urgent }))
///     }
///
///     fn answers(handles: &Self::Handles, answers: &Answers) -> Self::Answers {
///         UrgencyAnswers { urgent: answers.get(handles.urgent).clone() }
///     }
/// }
///
/// let (questions, handles) = Urgency::questions()?;
/// assert_eq!(questions.len(), 1);
/// let _ = handles;
/// # Ok::<(), Error>(())
/// ```
pub trait QuestionSet: Sized {
    /// Groups the handles needed to bind decoded answers to declared fields.
    type Handles;

    /// Defines the answer struct returned for this reusable set.
    type Answers;

    /// Builds a fresh batch when this set is about to be evaluated.
    ///
    /// # Errors
    ///
    /// Returns a builder error, such as [`Error::DuplicateId`].
    fn questions() -> Result<(Questions, Self::Handles), Error>;

    /// Projects decoded slots into this set's typed answer struct.
    fn answers(handles: &Self::Handles, answers: &Answers) -> Self::Answers;
}

/// A typed question-set result together with response metadata.
///
/// Use this after evaluating a [`QuestionSet`] when direct field access is more
/// useful than retaining individual [`Handle`](crate::Handle) values. It
/// dereferences to `T::Answers`, so generated answer fields can be read
/// directly from the result. Use [`Answers`] for runtime-built batches.
///
/// ```
/// use jevrs_core::{Answered, Answers, Error, Handle, Noul, NoulAnswer, QuestionSet, Questions, decode};
///
/// struct Urgency;
/// struct UrgencyHandles { urgent: Handle<Noul> }
/// struct UrgencyAnswers { urgent: NoulAnswer }
/// impl QuestionSet for Urgency {
///     type Handles = UrgencyHandles;
///     type Answers = UrgencyAnswers;
///     fn questions() -> Result<(Questions, Self::Handles), Error> {
///         let mut questions = Questions::new();
///         let urgent = questions.noul("urgent", "Does this convey urgency?")?;
///         Ok((questions, UrgencyHandles { urgent }))
///     }
///     fn answers(handles: &Self::Handles, answers: &Answers) -> Self::Answers {
///         UrgencyAnswers { urgent: answers.get(handles.urgent).clone() }
///     }
/// }
/// let (questions, handles) = Urgency::questions()?;
/// let raw = decode(&questions, br#"{
///   "model":"jev-1.13.0",
///   "answers":{"urgent":{"type":"noul","noul":0.95}},
///   "usage":{"input_tokens":10,"output_tokens":2}
/// }"#)?;
/// let answered = Answered::<Urgency>::from_answers(&handles, &raw);
/// assert!(answered.urgent.is_yes(0.9));
/// assert_eq!(answered.model(), "jev-1.13.0");
/// # Ok::<(), Error>(())
/// ```
pub struct Answered<T: QuestionSet> {
    /// Use this field when the generated answer struct must be moved as a whole.
    pub answers: T::Answers,
    model: String,
    usage: Usage,
}

impl<T> Clone for Answered<T>
where
    T: QuestionSet,
    T::Answers: Clone,
{
    fn clone(&self) -> Self {
        Self {
            answers: self.answers.clone(),
            model: self.model.clone(),
            usage: self.usage,
        }
    }
}

impl<T> fmt::Debug for Answered<T>
where
    T: QuestionSet,
    T::Answers: fmt::Debug,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Answered")
            .field("answers", &self.answers)
            .field("model", &self.model)
            .field("usage", &self.usage)
            .finish()
    }
}

impl<T: QuestionSet> Answered<T> {
    /// Converts [`Answers`] into a [`QuestionSet`]'s declared result shape.
    #[must_use]
    pub fn from_answers(handles: &T::Handles, answers: &Answers) -> Self {
        Self {
            answers: T::answers(handles, answers),
            model: answers.model().to_string(),
            usage: answers.usage(),
        }
    }

    /// Returns the concrete model version for logs and reproducibility.
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Returns token counts for observability and cost accounting.
    #[must_use]
    pub const fn usage(&self) -> Usage {
        self.usage
    }
}

impl<T: QuestionSet> Deref for Answered<T> {
    type Target = T::Answers;

    fn deref(&self) -> &Self::Target {
        &self.answers
    }
}
