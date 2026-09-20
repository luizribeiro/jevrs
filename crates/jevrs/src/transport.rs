use core::{future::Future, time::Duration};

use http::{Request, Response};

/// A bound that requires `Send` except on WebAssembly targets.
#[cfg(target_arch = "wasm32")]
pub trait MaybeSend {}

#[cfg(target_arch = "wasm32")]
impl<T> MaybeSend for T {}

/// A bound that requires `Send` except on WebAssembly targets.
#[cfg(not(target_arch = "wasm32"))]
pub trait MaybeSend: Send {}

#[cfg(not(target_arch = "wasm32"))]
impl<T: Send> MaybeSend for T {}

/// Sends fully framed HTTP requests for the Jev client.
///
/// Implement this trait to use an HTTP stack that `jevrs` does not provide.
///
/// ```
/// use std::{convert::Infallible, future::Future};
/// use http::{Request, Response};
/// use jevrs::{MaybeSend, Transport};
///
/// struct ToyTransport;
///
/// impl Transport for ToyTransport {
///     type Error = Infallible;
///
///     fn send(
///         &self,
///         _request: Request<Vec<u8>>,
///     ) -> impl Future<Output = Result<Response<Vec<u8>>, Self::Error>> + MaybeSend {
///         async { Ok(Response::new(Vec::new())) }
///     }
/// }
/// ```
pub trait Transport {
    /// The error returned when a request cannot be completed.
    type Error: core::error::Error + Send + Sync + 'static;

    /// Sends one HTTP request and returns its complete response.
    fn send(
        &self,
        request: Request<Vec<u8>>,
    ) -> impl Future<Output = Result<Response<Vec<u8>>, Self::Error>> + MaybeSend;

    /// Reports whether a failed request is safe to retry.
    fn is_retryable(_error: &Self::Error) -> bool {
        false
    }
}

/// Pauses a retry loop without prescribing an async runtime.
///
/// ```
/// use std::{future::Future, time::Duration};
/// use jevrs::{MaybeSend, Sleep};
///
/// struct Immediate;
///
/// impl Sleep for Immediate {
///     fn sleep(&self, _duration: Duration) -> impl Future<Output = ()> + MaybeSend {
///         async {}
///     }
/// }
/// ```
pub trait Sleep {
    /// Whether this sleeper can support retry delays.
    ///
    /// Sleepers that cannot wait set this to `false`; `ClientBuilder::build`
    /// then forces [`crate::RetryPolicy::max_retries`] to zero.
    const RETRIES_ENABLED: bool = true;

    /// Waits for `duration` before completing.
    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + MaybeSend;
}

/// Disables waiting and, when used by a client builder, all retries.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoSleep;

impl Sleep for NoSleep {
    const RETRIES_ENABLED: bool = false;

    async fn sleep(&self, _duration: Duration) {}
}
