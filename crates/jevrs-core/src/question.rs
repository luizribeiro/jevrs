use alloc::{string::String, vec::Vec};
use core::marker::PhantomData;

use crate::{Confidence, Levels, Options, Probability};

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn nearest_index(expected: f64, maximum: usize) -> usize {
    ((expected + 0.5) as usize).min(maximum)
}

/// Binds a question marker to the answer type returned for it.
pub trait Question: 'static {
    /// The answer produced for this question kind.
    type Answer: 'static;
}

/// Marker for a yes/no question.
#[derive(Clone, Copy, Debug)]
pub struct NoulQ;

/// Marker for a choice question using static options `O`.
#[derive(Clone, Copy, Debug)]
pub struct ChoiceQ<O: Options>(PhantomData<O>);

/// Marker for a score question using static levels `L`.
#[derive(Clone, Copy, Debug)]
pub struct ScoreQ<L: Levels>(PhantomData<L>);

/// Marker for a choice question using runtime-defined options.
#[derive(Clone, Copy, Debug)]
pub struct DynChoiceQ;

/// Marker for a score question using runtime-defined levels.
#[derive(Clone, Copy, Debug)]
pub struct DynScoreQ;

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
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoulAnswer {
    /// Probability that the answer is yes.
    pub p: Probability,
}

impl NoulAnswer {
    /// Reports whether the positive probability meets `threshold`.
    #[must_use]
    pub fn is_yes(&self, threshold: f64) -> bool {
        self.p.get() >= threshold
    }
}

/// The selected option and probability distribution for a static choice.
#[derive(Clone, Debug, PartialEq)]
pub struct ChoiceAnswer<O: Options> {
    /// The option selected by the model.
    pub pick: O,
    /// One probability per option.
    pub probs: O::Map<Probability>,
    /// The model's confidence in this answer.
    pub confidence: Confidence,
}

/// The expected score and probability distribution for static levels.
#[derive(Clone, Debug, PartialEq)]
pub struct ScoreAnswer<L: Levels> {
    expected: f64,
    /// One probability per level.
    pub probs: L::Map<Probability>,
    /// The model's confidence in this answer.
    pub confidence: Confidence,
}

impl<L: Levels> ScoreAnswer<L> {
    #[allow(dead_code)]
    pub(crate) fn new(expected: f64, probs: L::Map<Probability>, confidence: Confidence) -> Self {
        Self {
            expected,
            probs,
            confidence,
        }
    }

    /// Returns the probability-weighted level index reported by the API.
    #[must_use]
    pub fn expected(&self) -> f64 {
        self.expected
    }

    /// Returns the level nearest to [`ScoreAnswer::expected`].
    #[must_use]
    pub fn nearest(&self) -> L {
        let index = nearest_index(self.expected, L::N.saturating_sub(1));
        L::all()[index]
    }

    /// Returns the first level having the greatest probability.
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
#[derive(Clone, Debug, PartialEq)]
pub struct DynChoiceAnswer {
    /// The option key selected by the model.
    pub pick: String,
    /// Keyed probabilities in the originating [`crate::DynOptions`] order.
    pub probs: Vec<(String, Probability)>,
    /// The model's confidence in this answer.
    pub confidence: Confidence,
}

/// The expected score and probability distribution for dynamic levels.
#[derive(Clone, Debug, PartialEq)]
pub struct DynScoreAnswer {
    expected: f64,
    /// Probabilities in ascending score order.
    pub probs: Vec<Probability>,
    /// Level descriptions echoed by the API in ascending score order.
    pub legend: Vec<String>,
    /// The model's confidence in this answer.
    pub confidence: Confidence,
}

impl DynScoreAnswer {
    #[allow(dead_code)]
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

    /// Returns the probability-weighted level index reported by the API.
    #[must_use]
    pub fn expected(&self) -> f64 {
        self.expected
    }

    /// Returns the index nearest to [`DynScoreAnswer::expected`].
    #[must_use]
    pub fn nearest(&self) -> usize {
        nearest_index(self.expected, self.probs.len().saturating_sub(1))
    }

    /// Returns the first index having the greatest probability.
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
