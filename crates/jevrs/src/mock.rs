use std::{
    collections::VecDeque,
    fmt,
    future::{Future, ready},
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};

use http::{Request, Response};

use crate::{MaybeSend, Sleep, Transport};

type Canned = Result<Response<Vec<u8>>, MockError>;

/// A transport failure returned by [`MockTransport`].
#[cfg_attr(docsrs, doc(cfg(feature = "test-util")))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MockError {
    /// Whether a client should retry the failed request.
    pub retryable: bool,
}

impl fmt::Display for MockError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("mock transport error")
    }
}

impl core::error::Error for MockError {}

/// An in-memory transport that records requests and replays canned outcomes.
///
/// Use it to test HTTP framing and retry behavior without network access.
/// Outcomes are returned in insertion order.
///
/// ```
/// # use std::{future::Future, pin::pin, task::{Context, Poll, Waker}};
/// use http::{Request, Response};
/// use jevrs::{MockError, MockTransport, Transport};
///
/// # fn block_on<F: Future>(future: F) -> F::Output {
/// #     let waker = Waker::noop();
/// #     let mut context = Context::from_waker(waker);
/// #     let mut future = pin!(future);
/// #     loop {
/// #         match future.as_mut().poll(&mut context) {
/// #             Poll::Ready(output) => return output,
/// #             Poll::Pending => std::thread::yield_now(),
/// #         }
/// #     }
/// # }
/// let transport = MockTransport::new([Ok::<_, MockError>(Response::new(b"ok".to_vec()))]);
/// let response = block_on(transport.send(Request::new(Vec::new()))).unwrap();
/// assert_eq!(response.body(), b"ok");
/// assert_eq!(transport.take_requests().len(), 1);
/// ```
#[cfg_attr(docsrs, doc(cfg(feature = "test-util")))]
#[derive(Clone, Default)]
pub struct MockTransport {
    state: Arc<MockState>,
}

#[derive(Default)]
struct MockState {
    requests: Mutex<Vec<Request<Vec<u8>>>>,
    outcomes: Mutex<VecDeque<Canned>>,
}

impl MockTransport {
    /// Creates a mock that replays `outcomes` in order.
    pub fn new(outcomes: impl IntoIterator<Item = Canned>) -> Self {
        Self {
            state: Arc::new(MockState {
                requests: Mutex::new(Vec::new()),
                outcomes: Mutex::new(outcomes.into_iter().collect()),
            }),
        }
    }

    /// Adds a successful response to the back of the replay queue.
    pub fn push_response(&self, response: Response<Vec<u8>>) {
        lock(&self.state.outcomes).push_back(Ok(response));
    }

    /// Adds a transport failure to the back of the replay queue.
    pub fn push_error(&self, error: MockError) {
        lock(&self.state.outcomes).push_back(Err(error));
    }

    /// Removes and returns all requests recorded so far.
    #[must_use]
    pub fn take_requests(&self) -> Vec<Request<Vec<u8>>> {
        core::mem::take(&mut *lock(&self.state.requests))
    }
}

impl Transport for MockTransport {
    type Error = MockError;

    fn send(
        &self,
        request: Request<Vec<u8>>,
    ) -> impl Future<Output = Result<Response<Vec<u8>>, Self::Error>> + MaybeSend {
        lock(&self.state.requests).push(request);
        ready(match lock(&self.state.outcomes).pop_front() {
            Some(outcome) => outcome,
            None => Err(MockError { retryable: false }),
        })
    }

    fn is_retryable(error: &Self::Error) -> bool {
        error.retryable
    }
}

/// A sleeper that records delays and completes immediately.
#[cfg_attr(docsrs, doc(cfg(feature = "test-util")))]
#[derive(Clone, Default)]
pub struct MockSleep {
    durations: Arc<Mutex<Vec<Duration>>>,
}

impl MockSleep {
    /// Removes and returns all requested sleep durations.
    #[must_use]
    pub fn take_durations(&self) -> Vec<Duration> {
        core::mem::take(&mut *lock(&self.durations))
    }
}

impl Sleep for MockSleep {
    async fn sleep(&self, duration: Duration) {
        lock(&self.durations).push(duration);
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(test)]
mod tests {
    use http::{Request, Response};

    use super::{MockError, MockTransport};
    use crate::{Transport, test_support::block_on};

    #[test]
    fn records_requests_and_replays_in_order() {
        let transport = MockTransport::new([
            Ok(Response::new(b"first".to_vec())),
            Ok(Response::new(b"second".to_vec())),
        ]);

        let first = block_on(transport.send(Request::new(b"one".to_vec()))).unwrap();
        let second = block_on(transport.send(Request::new(b"two".to_vec()))).unwrap();

        assert_eq!(first.body(), b"first");
        assert_eq!(second.body(), b"second");
        let requests = transport.take_requests();
        assert_eq!(requests[0].body(), b"one");
        assert_eq!(requests[1].body(), b"two");
    }

    #[test]
    fn exhaustion_returns_an_error() {
        let transport = MockTransport::default();
        let error = block_on(transport.send(Request::new(Vec::new()))).unwrap_err();
        assert_eq!(error, MockError { retryable: false });
    }
}
