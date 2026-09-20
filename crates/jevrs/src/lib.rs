#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../docs/guide.md")]

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;

mod client;
#[cfg(feature = "test-util")]
mod mock;
#[cfg(all(
    any(feature = "reqwest", feature = "native-tls"),
    not(target_arch = "wasm32")
))]
mod reqwest;
mod retry;
#[cfg(all(test, feature = "test-util"))]
mod test_support;
mod transport;
#[cfg(any(test, all(target_arch = "wasm32", target_env = "p2")))]
mod wasi_common;
#[cfg(all(target_arch = "wasm32", target_env = "p2"))]
mod wasip2;
#[cfg(all(target_arch = "wasm32", target_env = "p3"))]
mod wasip3;

pub use client::{Client, ClientBuilder, ModelInfo};
pub use jevrs_core::*;
#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use jevrs_derive::{Levels, Options, Questions};
#[cfg(feature = "test-util")]
#[cfg_attr(docsrs, doc(cfg(feature = "test-util")))]
pub use mock::{MockError, MockSleep, MockTransport};
#[cfg(all(
    any(feature = "reqwest", feature = "native-tls"),
    not(target_arch = "wasm32")
))]
#[cfg_attr(
    docsrs,
    doc(cfg(all(
        any(feature = "reqwest", feature = "native-tls"),
        not(target_arch = "wasm32")
    )))
)]
pub use reqwest::{ReqwestTransport, TokioSleep};
pub use retry::RetryPolicy;
pub use transport::{MaybeSend, NoSleep, Sleep, Transport};
#[cfg(all(target_arch = "wasm32", target_env = "p2"))]
#[cfg_attr(docsrs, doc(cfg(all(target_arch = "wasm32", target_env = "p2"))))]
pub use wasip2::{Wasip2Error, Wasip2Sleep, Wasip2Transport};
#[cfg(all(target_arch = "wasm32", target_env = "p3"))]
#[cfg_attr(docsrs, doc(cfg(all(target_arch = "wasm32", target_env = "p3"))))]
pub use wasip3::{Wasip3Error, Wasip3Sleep, Wasip3Transport};
