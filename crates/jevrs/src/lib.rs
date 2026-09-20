//! Rust client building blocks for `TypeSafe` AI's Jev API.
//!
//! Concrete transports are selected with crate features. With none enabled,
//! applications can provide their own [`Transport`] and [`Sleep`]
//! implementations.

mod retry;
mod transport;

pub use jevrs_core::*;
pub use retry::RetryPolicy;
pub use transport::{MaybeSend, NoSleep, Sleep, Transport};
