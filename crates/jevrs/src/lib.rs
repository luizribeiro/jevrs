#![cfg_attr(docsrs, feature(doc_cfg))]
//! Rust client building blocks for `TypeSafe` AI's Jev API.
//!
//! The default `derive` feature provides a concise, typed path from question
//! declarations to answers:
//!
//! ```no_run
//! use jevrs::{Choice, Client, Levels, Noul, Options, Questions, Score};
//!
//! #[derive(Clone, Copy, Debug, Eq, Options, PartialEq)]
//! enum Department {
//!     /// Payments, invoicing, refunds
//!     Billing,
//!     /// Bugs, outages, integrations
//!     Technical,
//!     Sales,
//! }
//!
//! #[derive(Clone, Copy, Debug, Eq, Levels, Ord, PartialEq, PartialOrd)]
//! enum Frustration {
//!     /// Calm
//!     Calm,
//!     /// Frustrated
//!     Frustrated,
//!     /// Very angry
//!     VeryAngry,
//! }
//!
//! #[derive(Questions)]
//! struct Triage {
//!     #[jev(
//!         noul = "Does this convey urgency?",
//!         yes = "Explicitly time-sensitive",
//!         no = "No urgency expressed"
//!     )]
//!     is_urgent: Noul,
//!     #[jev(choice = "Which team should handle this?")]
//!     department: Choice<Department>,
//!     #[jev(score = "How frustrated is the customer?")]
//!     frustration: Score<Frustration>,
//! }
//!
//! # async fn run() -> Result<(), jevrs::Error> {
//! let client = Client::reqwest().from_env()?.build()?;
//! let triage = client
//!     .ask::<Triage>(&"Help! My payouts have been failing for 3 days.")
//!     .await?;
//! println!("department: {:?}", triage.department.pick);
//! # Ok(())
//! # }
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
pub use jevrs_derive::{Levels, Options, Questions};
#[cfg(feature = "test-util")]
pub use mock::{MockError, MockSleep, MockTransport};
#[cfg(any(feature = "reqwest", feature = "native-tls"))]
pub use reqwest::{ReqwestTransport, TokioSleep};
pub use retry::RetryPolicy;
pub use transport::{MaybeSend, NoSleep, Sleep, Transport};
