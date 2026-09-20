#![no_std]
//! Sans-I/O types, validation, and JSON codecs for the Jev API.

extern crate alloc;

mod probability;

pub use probability::{Confidence, Probability};
