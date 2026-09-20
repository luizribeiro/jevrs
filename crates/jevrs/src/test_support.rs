use std::{
    future::Future,
    pin::pin,
    task::{Context, Poll, Waker},
};

#[path = "../../../tests/fixtures/mod.rs"]
pub(crate) mod fixtures;

pub(crate) fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}
