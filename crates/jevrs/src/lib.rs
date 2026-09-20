#![cfg_attr(docsrs, feature(doc_cfg))]
//! Rust client building blocks for `TypeSafe` AI's Jev API.
//!
//! The default `derive` feature provides [`Options`] and [`Levels`] derives for
//! static criteria:
//!
//! ```
//! use jevrs::{Levels, Model, Options, Questions, encode};
//!
//! #[derive(Clone, Copy, Eq, Options, PartialEq)]
//! enum Dept {
//!     /// Payments, invoicing, refunds
//!     Billing,
//!     #[jev(key = "tech", desc = "Bugs, outages, integrations")]
//!     Technical,
//!     Sales,
//! }
//!
//! #[derive(Clone, Copy, Eq, Levels, Ord, PartialEq, PartialOrd)]
//! enum Frustration {
//!     /// Calm
//!     Calm,
//!     /// Frustrated
//!     Frustrated,
//!     /// Very angry
//!     VeryAngry,
//! }
//!
//! let mut questions = Questions::new();
//! questions.choice::<Dept>("department", "Which team should handle this?")?;
//! questions.score::<Frustration>(
//!     "frustration_level",
//!     "How frustrated is the customer?",
//! )?;
//! let state = serde_json::json!({
//!     "message": "Help! My payouts have been failing for 3 days."
//! });
//! let request = encode(&Model::LATEST, &state, &questions)?;
//! assert!(!request.is_empty());
//! # Ok::<(), jevrs::Error>(())
//! ```
//!
//! Concrete transports are selected with crate features. With none enabled,
//! applications can provide their own [`Transport`] and [`Sleep`]
//! implementations. The `wasip2`, `wasip3`, and `web` transport features are
//! reserved for future milestones and currently add no implementation.

mod client;
#[cfg(feature = "test-util")]
mod mock;
#[cfg(any(feature = "reqwest", feature = "native-tls"))]
mod reqwest;
mod retry;
#[cfg(all(test, feature = "test-util"))]
mod test_support;
mod transport;

pub use client::{Client, ClientBuilder, ModelInfo};
pub use jevrs_core::*;
#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use jevrs_derive::{Levels, Options};
#[cfg(feature = "test-util")]
pub use mock::{MockError, MockSleep, MockTransport};
#[cfg(any(feature = "reqwest", feature = "native-tls"))]
pub use reqwest::{ReqwestTransport, TokioSleep};
pub use retry::RetryPolicy;
pub use transport::{MaybeSend, NoSleep, Sleep, Transport};
