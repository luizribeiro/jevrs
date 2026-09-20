#![no_std]
//! Sans-I/O types, validation, and JSON codecs for the Jev API.

extern crate alloc;

mod error;
mod options;
mod probability;
mod question;
#[cfg(test)]
pub(crate) mod test_support;
mod types;

pub use error::{Error, ErrorDetail, classify};
pub use options::{ArrayMap, DynLevels, DynOptions, Indexed, Levels, Options};
pub use probability::{Confidence, Probability};
pub use question::{
    ChoiceAnswer, ChoiceQ, DynChoiceAnswer, DynChoiceQ, DynScoreAnswer, DynScoreQ, NoulAnswer,
    NoulQ, Question, ScoreAnswer, ScoreQ,
};
pub use types::{Instructions, Model, Usage};
