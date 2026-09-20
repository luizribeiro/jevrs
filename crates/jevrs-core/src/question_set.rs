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
    /// Typed handles used to bind the response to the declared fields.
    type Handles;

    /// The fully typed answers returned for this set.
    type Answers;

    /// Builds the question batch and its corresponding typed handles.
    ///
    /// # Errors
    ///
    /// Returns a builder error, such as [`Error::DuplicateId`].
    fn questions() -> Result<(Questions, Self::Handles), Error>;

    /// Clones the declared answers out of a decoded response.
    fn answers(handles: &Self::Handles, answers: &Answers) -> Self::Answers;
}

/// A typed question-set result together with response metadata.
///
/// It dereferences to `T::Answers`, so generated answer fields can be read
/// directly from the result.
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
    /// Typed answers in the shape declared by `T`.
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
    /// Builds a typed result from decoded answers and their matching handles.
    #[must_use]
    pub fn from_answers(handles: &T::Handles, answers: &Answers) -> Self {
        Self {
            answers: T::answers(handles, answers),
            model: answers.model().to_string(),
            usage: answers.usage(),
        }
    }

    /// Returns the concrete model version reported by the API.
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Returns the token counts reported by the API.
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
