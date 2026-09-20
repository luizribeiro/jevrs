//! Rust client building blocks for `TypeSafe` AI's Jev API.
//!
//! Concrete transports are selected with crate features. With none enabled,
//! applications can provide their own [`Transport`] and [`Sleep`]
//! implementations.

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
#[cfg(feature = "test-util")]
pub use mock::{MockError, MockSleep, MockTransport};
#[cfg(any(feature = "reqwest", feature = "native-tls"))]
pub use reqwest::{ReqwestTransport, TokioSleep};
pub use retry::RetryPolicy;
pub use transport::{MaybeSend, NoSleep, Sleep, Transport};
