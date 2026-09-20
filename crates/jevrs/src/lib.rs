#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../docs/guide.md")]

mod client;
#[cfg(feature = "test-util")]
mod mock;
#[cfg(any(feature = "reqwest", feature = "native-tls"))]
mod reqwest;
mod retry;
#[cfg(all(test, feature = "test-util"))]
mod test_support;
mod transport;
#[cfg(any(
    test,
    all(feature = "wasip2", target_arch = "wasm32", target_env = "p2")
))]
mod wasi_common;
#[cfg(all(feature = "wasip2", target_arch = "wasm32", target_env = "p2"))]
mod wasip2;
#[cfg(all(feature = "wasip3", target_arch = "wasm32", target_env = "p3"))]
mod wasip3;

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
#[cfg(all(feature = "wasip2", target_arch = "wasm32", target_env = "p2"))]
#[cfg_attr(docsrs, doc(cfg(feature = "wasip2")))]
pub use wasip2::{Wasip2Error, Wasip2Sleep, Wasip2Transport};
#[cfg(all(feature = "wasip3", target_arch = "wasm32", target_env = "p3"))]
#[cfg_attr(docsrs, doc(cfg(feature = "wasip3")))]
pub use wasip3::{Wasip3Error, Wasip3Sleep, Wasip3Transport};
