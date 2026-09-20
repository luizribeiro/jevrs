//! Asynchronous WASI HTTP 0.3 transport for WebAssembly components.
//!
//! Choose this transport for components hosted by a runtime providing the
//! draft WASI 0.3 interfaces. Build it with the pinned nightly shell:
//!
//! ```text
//! nix develop .#nightly -c cargo build -p jevrs --no-default-features --features wasip3 --target wasm32-wasip3
//! ```

use core::fmt;

use bytes::Bytes;
use http::{Request, Response};
use http_body_util::{BodyExt as _, Full};
use wasip3::{
    clocks::monotonic_clock,
    http::{client, types::ErrorCode},
    http_compat::{http_from_wasi_response, http_into_wasi_request},
};

use crate::{Sleep, Transport};

/// An error reported while sending a request through WASI HTTP 0.3.
///
/// This type owns only plain data, so it can cross the client's `Send` and
/// `Sync` error boundary without retaining a WASI resource handle.
#[derive(Debug)]
#[non_exhaustive]
pub enum Wasip3Error {
    /// Match this to inspect a host-reported WASI HTTP failure.
    Http(ErrorCode),
}

impl fmt::Display for Wasip3Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http(error) => write!(formatter, "WASI HTTP error: {error}"),
        }
    }
}

impl core::error::Error for Wasip3Error {}

impl From<ErrorCode> for Wasip3Error {
    fn from(error: ErrorCode) -> Self {
        Self::Http(error)
    }
}

/// Sends Jev requests through a host's asynchronous WASI HTTP 0.3 interface.
///
/// Use this transport in a `wasm32-wasip3` component hosted by Wasmtime or
/// another runtime implementing the WASI 0.3 draft.
///
/// ```no_run
/// use jevrs::{Client, Error, Wasip3Transport};
///
/// # fn configured() -> Result<(), Error> {
/// let client = Client::builder(Wasip3Transport)
///     .from_env()?
///     .build()?;
/// # let _ = client;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct Wasip3Transport;

impl Transport for Wasip3Transport {
    type Error = Wasip3Error;

    async fn send(&self, request: Request<Vec<u8>>) -> Result<Response<Vec<u8>>, Self::Error> {
        send(request).await
    }

    fn is_retryable(error: &Self::Error) -> bool {
        matches!(
            error,
            Wasip3Error::Http(
                ErrorCode::DnsTimeout
                    | ErrorCode::ConnectionRefused
                    | ErrorCode::ConnectionTerminated
                    | ErrorCode::ConnectionTimeout
                    | ErrorCode::ConnectionReadTimeout
                    | ErrorCode::ConnectionWriteTimeout
                    | ErrorCode::ConnectionLimitReached
                    | ErrorCode::HttpResponseTimeout
            )
        )
    }
}

/// Waits between retry attempts using a host's WASI monotonic clock.
///
/// Pair this sleeper with [`Wasip3Transport`] when a `wasi:clocks` 0.3 host
/// should provide retry delays.
///
/// ```no_run
/// use jevrs::{Client, Wasip3Sleep, Wasip3Transport};
///
/// let builder = Client::builder(Wasip3Transport).sleep(Wasip3Sleep);
/// # let _ = builder;
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct Wasip3Sleep;

impl Sleep for Wasip3Sleep {
    async fn sleep(&self, duration: core::time::Duration) {
        let nanoseconds = u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX);
        monotonic_clock::wait_for(nanoseconds).await;
    }
}

async fn send(request: Request<Vec<u8>>) -> Result<Response<Vec<u8>>, Wasip3Error> {
    let (parts, body) = request.into_parts();
    let request = Request::from_parts(parts, Full::new(Bytes::from(body)));
    let response = client::send(http_into_wasi_request(request)?).await?;
    let (parts, body) = http_from_wasi_response(response)?.into_parts();
    let body = body.collect().await?.to_bytes().to_vec();
    Ok(Response::from_parts(parts, body))
}
