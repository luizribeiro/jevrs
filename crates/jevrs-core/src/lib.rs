#![no_std]
//! Sans-I/O types, validation, and JSON codecs for the Jev API.

extern crate alloc;

mod probability;
mod types;

pub use probability::{Confidence, Probability};
pub use types::{Instructions, Model, Usage};
