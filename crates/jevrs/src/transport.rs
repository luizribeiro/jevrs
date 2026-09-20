use core::{future::Future, time::Duration};

use http::{Request, Response};

/// A portability bound for futures returned by [`Transport`] and [`Sleep`].
///
/// Use it on custom implementation return types so native futures are `Send`
/// without imposing that requirement on WebAssembly.
#[cfg(target_arch = "wasm32")]
pub trait MaybeSend {}

#[cfg(target_arch = "wasm32")]
impl<T> MaybeSend for T {}

/// A portability bound for futures returned by [`Transport`] and [`Sleep`].
///
/// Use it on custom implementation return types so native futures are `Send`
/// without imposing that requirement on WebAssembly.
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
    /// Use this type to preserve failures from the underlying HTTP stack.
    type Error: core::error::Error + Send + Sync + 'static;

    /// Sends the fully framed request when the client performs an API operation.
    fn send(
        &self,
        request: Request<Vec<u8>>,
    ) -> impl Future<Output = Result<Response<Vec<u8>>, Self::Error>> + MaybeSend;

    /// Marks transient transport failures that are safe for the client to retry.
    fn is_retryable(_error: &Self::Error) -> bool {
        false
    }
}

/// Pauses a retry loop without prescribing an async runtime.
///
/// Implement this beside a custom [`Transport`] when retries should use your
/// runtime's timer. Use [`NoSleep`] when retries must remain disabled.
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
    /// Tells [`ClientBuilder`](crate::ClientBuilder) whether retries can run.
    ///
    /// Sleepers that cannot wait set this to `false`; `ClientBuilder::build`
    /// then forces [`crate::RetryPolicy::max_retries`] to zero.
    const RETRIES_ENABLED: bool = true;

    /// Waits before the client's next retry attempt.
    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + MaybeSend;
}

/// Disables waiting and, when used by a client builder, all retries.
///
/// Use this for a custom transport when the application must return the first
/// failure immediately. Supply a real [`Sleep`] implementation to retry.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoSleep;

impl Sleep for NoSleep {
    const RETRIES_ENABLED: bool = false;

    async fn sleep(&self, _duration: Duration) {}
}
