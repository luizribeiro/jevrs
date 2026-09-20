use alloc::{string::String, vec::Vec};
use core::marker::PhantomData;

use crate::{Confidence, Levels, Options, Probability};

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn nearest_index(expected: f64, maximum: usize) -> usize {
    ((expected + 0.5) as usize).min(maximum)
}

/// Binds a question marker to the answer type returned for it.
///
/// Implement this only for a custom builder integration. The built-in marker
/// types cover every Jev question kind.
pub trait Question: 'static {
    /// Selects the answer type that a [`Handle`](crate::Handle) retrieves.
    type Answer: Send + Sync + 'static;
}

/// Marker to use for a yes/no question in a typed handle or [`QuestionSet`](crate::QuestionSet).
#[derive(Clone, Copy, Debug, Default)]
pub struct NoulQ;

/// Marker to use when [`Options`] define a choice at compile time.
///
/// Use [`DynChoiceQ`] when options are supplied at runtime.
#[derive(Clone, Copy, Debug)]
pub struct ChoiceQ<O: Options>(PhantomData<O>);

/// Marker to use when [`Levels`] define a score scale at compile time.
///
/// Use [`DynScoreQ`] when levels are supplied at runtime.
#[derive(Clone, Copy, Debug)]
pub struct ScoreQ<L: Levels>(PhantomData<L>);

/// Marker returned for a choice built with runtime-defined [`crate::DynOptions`].
///
/// Use [`ChoiceQ`] for static options.
#[derive(Clone, Copy, Debug, Default)]
pub struct DynChoiceQ;

/// Marker returned for a score built with runtime-defined [`crate::DynLevels`].
///
/// Use [`ScoreQ`] for static levels.
#[derive(Clone, Copy, Debug, Default)]
pub struct DynScoreQ;

impl<O: Options> Default for ChoiceQ<O> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<L: Levels> Default for ScoreQ<L> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

/// Readable alias to use for a yes/no field in a [`crate::QuestionSet`].
pub type Noul = NoulQ;

/// Readable alias to use for a static choice field in a [`crate::QuestionSet`].
pub type Choice<O> = ChoiceQ<O>;

/// Readable alias to use for a static score field in a [`crate::QuestionSet`].
pub type Score<L> = ScoreQ<L>;

impl Question for NoulQ {
    type Answer = NoulAnswer;
}

impl<O: Options> Question for ChoiceQ<O> {
    type Answer = ChoiceAnswer<O>;
}

impl<L: Levels> Question for ScoreQ<L> {
    type Answer = ScoreAnswer<L>;
}

impl Question for DynChoiceQ {
    type Answer = DynChoiceAnswer;
}

impl Question for DynScoreQ {
    type Answer = DynScoreAnswer;
}

/// The probability assigned to a yes/no question's positive outcome.
///
/// Use [`Self::is_yes`] when your application has a decision threshold, or
/// read [`Self::p`] when it needs the continuous probability.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoulAnswer {
    /// Use this probability when ranking or displaying the positive outcome.
    pub p: Probability,
}

impl NoulAnswer {
    /// Applies an application-defined inclusive threshold to the probability.
    #[must_use]
    pub fn is_yes(&self, threshold: f64) -> bool {
        self.p.get() >= threshold
    }
}

/// The selected option and probability distribution for a static choice.
///
/// Use this typed result with [`ChoiceQ`]. Use [`DynChoiceAnswer`] when option
/// keys were supplied at runtime.
#[derive(Clone, Debug, PartialEq)]
pub struct ChoiceAnswer<O: Options> {
    /// Use the selected enum value for ordinary branching.
    pub pick: O,
    /// Use the full distribution when the winning option alone is insufficient.
    pub probs: O::Map<Probability>,
    /// Use this confidence to decide whether human review is needed.
    pub confidence: Confidence,
}

/// The expected score and probability distribution for static levels.
///
/// Use this typed result with [`ScoreQ`]. Use [`DynScoreAnswer`] when level
/// descriptions were supplied at runtime.
#[derive(Clone, Debug, PartialEq)]
pub struct ScoreAnswer<L: Levels> {
    expected: f64,
    /// Use the full distribution when expected or nearest level loses detail.
    pub probs: L::Map<Probability>,
    /// Use this confidence to decide whether human review is needed.
    pub confidence: Confidence,
}

impl<L: Levels> ScoreAnswer<L> {
    pub(crate) fn new(expected: f64, probs: L::Map<Probability>, confidence: Confidence) -> Self {
        Self {
            expected,
            probs,
            confidence,
        }
    }

    /// Returns the fractional score when preserving model uncertainty matters.
    #[must_use]
    pub fn expected(&self) -> f64 {
        self.expected
    }

    /// Returns a level when the fractional [`Self::expected`] must be bucketed.
    #[must_use]
    pub fn nearest(&self) -> L {
        let index = nearest_index(self.expected, L::N.saturating_sub(1));
        L::all()[index]
    }

    /// Returns the most probable level instead of rounding [`Self::expected`].
    #[must_use]
    pub fn argmax(&self) -> L {
        let mut best = L::all()[0];
        for &level in &L::all()[1..] {
            if self.probs[level] > self.probs[best] {
                best = level;
            }
        }
        best
    }
}

/// The selected option and probability distribution for a dynamic choice.
///
/// Use this with [`DynChoiceQ`] and runtime [`crate::DynOptions`]. Use
/// [`ChoiceAnswer`] when an enum defines the choices.
#[derive(Clone, Debug, PartialEq)]
pub struct DynChoiceAnswer {
    /// Use the selected runtime key for ordinary branching.
    pub pick: String,
    /// Use the ordered distribution when the winning key alone is insufficient.
    pub probs: Vec<(String, Probability)>,
    /// Use this confidence to decide whether human review is needed.
    pub confidence: Confidence,
}

/// The expected score and probability distribution for dynamic levels.
///
/// Use this with [`DynScoreQ`] and runtime [`crate::DynLevels`]. Use
/// [`ScoreAnswer`] when an enum defines the levels.
#[derive(Clone, Debug, PartialEq)]
pub struct DynScoreAnswer {
    expected: f64,
    /// Use this distribution with [`Self::legend`] to inspect every level.
    pub probs: Vec<Probability>,
    /// Use these descriptions to label [`Self::probs`] and returned indexes.
    pub legend: Vec<String>,
    /// Use this confidence to decide whether human review is needed.
    pub confidence: Confidence,
}

impl DynScoreAnswer {
    pub(crate) fn new(
        expected: f64,
        probs: Vec<Probability>,
        legend: Vec<String>,
        confidence: Confidence,
    ) -> Self {
        Self {
            expected,
            probs,
            legend,
            confidence,
        }
    }

    /// Returns the fractional score when preserving model uncertainty matters.
    #[must_use]
    pub fn expected(&self) -> f64 {
        self.expected
    }

    /// Returns an index when the fractional [`Self::expected`] must be bucketed.
    #[must_use]
    pub fn nearest(&self) -> usize {
        nearest_index(self.expected, self.probs.len().saturating_sub(1))
    }

    /// Returns the most probable index instead of rounding [`Self::expected`].
    #[must_use]
    pub fn argmax(&self) -> usize {
        let mut best = 0;
        for index in 1..self.probs.len() {
            if self.probs[index] > self.probs[best] {
                best = index;
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use alloc::{string::ToString, vec};

    use super::{DynScoreAnswer, NoulAnswer, ScoreAnswer};
    use crate::{Confidence, Levels, Probability, test_support::Frustration};

    fn probability(value: f64) -> Probability {
        Probability::new(value).unwrap()
    }

    fn confidence() -> Confidence {
        Confidence::new(0.9).unwrap()
    }

    #[test]
    fn yes_threshold_is_inclusive() {
        let answer = NoulAnswer {
            p: probability(0.75),
        };
        assert!(answer.is_yes(0.74));
        assert!(answer.is_yes(0.75));
        assert!(!answer.is_yes(0.76));
    }

    #[test]
    fn static_score_rounds_clamps_and_uses_first_argmax() {
        let answer = ScoreAnswer::<Frustration>::new(
            1.5,
            Frustration::map_from_fn(|level| match level {
                Frustration::Calm => probability(0.1),
                Frustration::Frustrated | Frustration::VeryAngry => probability(0.45),
            }),
            confidence(),
        );
        assert!((answer.expected() - 1.5).abs() < f64::EPSILON);
        assert_eq!(answer.nearest(), Frustration::VeryAngry);
        assert_eq!(answer.argmax(), Frustration::Frustrated);

        let clamped = ScoreAnswer::<Frustration>::new(
            99.0,
            Frustration::map_from_fn(|_| probability(0.0)),
            confidence(),
        );
        assert_eq!(clamped.nearest(), Frustration::VeryAngry);
    }

    #[test]
    fn dynamic_score_rounds_clamps_and_uses_first_argmax() {
        let answer = DynScoreAnswer::new(
            0.5,
            vec![probability(0.1), probability(0.45), probability(0.45)],
            vec!["Low".to_string(), "Medium".to_string(), "High".to_string()],
            confidence(),
        );
        assert!((answer.expected() - 0.5).abs() < f64::EPSILON);
        assert_eq!(answer.nearest(), 1);
        assert_eq!(answer.argmax(), 1);

        let clamped = DynScoreAnswer::new(
            99.0,
            vec![probability(0.5), probability(0.5)],
            vec!["Low".to_string(), "High".to_string()],
            confidence(),
        );
        assert_eq!(clamped.nearest(), 1);
        assert_eq!(clamped.argmax(), 0);
    }
}
