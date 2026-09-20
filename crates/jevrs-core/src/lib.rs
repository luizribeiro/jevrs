#![no_std]
#![doc = include_str!("../docs/guide.md")]

extern crate alloc;

mod builder;
mod error;
mod options;
mod probability;
mod question;
mod question_set;
mod response;
#[cfg(test)]
pub(crate) mod test_support;
mod types;

pub use builder::{Handle, Questions, encode};
pub use error::{Error, ErrorDetail, classify};
pub use options::{ArrayMap, DynLevels, DynOptions, Indexed, Levels, Options};
pub use probability::{Confidence, Probability};
pub use question::{
    Choice, ChoiceAnswer, ChoiceQ, DynChoiceAnswer, DynChoiceQ, DynScoreAnswer, DynScoreQ, Noul,
    NoulAnswer, NoulQ, Question, Score, ScoreAnswer, ScoreQ,
};
pub use question_set::{Answered, QuestionSet};
pub use response::{Answers, decode};
pub use types::{Instructions, Model, Usage};
