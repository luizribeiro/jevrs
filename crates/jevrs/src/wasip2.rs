//! Synchronous WASI HTTP 0.2 transport for WebAssembly components.
//!
//! Choose this transport for components hosted by Wasmtime, Spin, wasmCloud,
//! or any other runtime providing `wasi:http` 0.2. Build it with:
//!
//! ```text
//! cargo build -p jevrs --target wasm32-wasip2
//! ```

use core::{fmt, future, future::Future};

use http::{HeaderMap, HeaderName, HeaderValue, Request, Response, StatusCode};
use wasi::{
    clocks::monotonic_clock::subscribe_duration,
    http::{
        outgoing_handler,
        types::{
            ErrorCode, Fields, IncomingBody, IncomingResponse, Method, OutgoingBody,
            OutgoingRequest, Scheme,
        },
    },
    io::{
        poll::{Pollable, poll},
        streams::{InputStream, OutputStream, StreamError},
    },
};

use crate::{Sleep, Transport, wasi_common};

const READ_CHUNK_SIZE: u64 = 64 * 1024;

/// An error reported while sending a request through WASI HTTP 0.2.
///
/// This type owns only plain data, so it can cross the client's `Send` and
/// `Sync` error boundary without retaining a WASI resource handle.
#[cfg_attr(docsrs, doc(cfg(all(target_arch = "wasm32", target_env = "p2"))))]
#[derive(Debug)]
#[non_exhaustive]
pub enum Wasip2Error {
    /// Match this to inspect a host-reported WASI HTTP failure.
    Http(ErrorCode),
    /// Match this when correcting an invalid configured base URL.
    BadUrl(String),
    /// Match this when diagnosing a header rejected by the WASI host.
    HeaderConversion(String),
    /// Match this when diagnosing a WASI stream failure.
    Stream(String),
    /// Match this when the host rejects or consumes a request resource.
    Request(String),
    /// Match this when the host response cannot be consumed.
    Response(String),
}

impl fmt::Display for Wasip2Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http(error) => write!(formatter, "WASI HTTP error: {error}"),
            Self::BadUrl(message) => write!(formatter, "invalid request URL: {message}"),
            Self::HeaderConversion(message) => {
                write!(formatter, "HTTP header conversion failed: {message}")
            }
            Self::Stream(message) => write!(formatter, "WASI stream error: {message}"),
            Self::Request(message) => write!(formatter, "WASI request error: {message}"),
            Self::Response(message) => write!(formatter, "WASI response error: {message}"),
        }
    }
}

impl core::error::Error for Wasip2Error {}

impl From<ErrorCode> for Wasip2Error {
    fn from(error: ErrorCode) -> Self {
        Self::Http(error)
    }
}

/// Sends Jev requests through a host's synchronous WASI HTTP 0.2 interface.
///
/// Use this transport in a `wasm32-wasip2` component hosted by Wasmtime,
/// Spin, wasmCloud, or another `wasi:http` 0.2 runtime.
///
/// ```no_run
/// use jevrs::{Client, Error, Wasip2Transport};
///
/// # fn configured() -> Result<(), Error> {
/// let client = Client::builder(Wasip2Transport)
///     .from_env()?
///     .build()?;
/// # let _ = client;
/// # Ok(())
/// # }
/// ```
#[cfg_attr(docsrs, doc(cfg(all(target_arch = "wasm32", target_env = "p2"))))]
#[derive(Clone, Copy, Debug, Default)]
pub struct Wasip2Transport;

impl Transport for Wasip2Transport {
    type Error = Wasip2Error;

    fn send(
        &self,
        request: Request<Vec<u8>>,
    ) -> impl Future<Output = Result<Response<Vec<u8>>, Self::Error>> + crate::MaybeSend {
        future::ready(send(request))
    }

    fn is_retryable(error: &Self::Error) -> bool {
        matches!(
            error,
            Wasip2Error::Http(
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
/// Pair this sleeper with [`Wasip2Transport`] when a `wasi:clocks` 0.2 host
/// should provide retry delays.
///
/// ```no_run
/// use jevrs::{Client, Wasip2Sleep, Wasip2Transport};
///
/// let builder = Client::builder(Wasip2Transport).sleep(Wasip2Sleep);
/// # let _ = builder;
/// ```
#[cfg_attr(docsrs, doc(cfg(all(target_arch = "wasm32", target_env = "p2"))))]
#[derive(Clone, Copy, Debug, Default)]
pub struct Wasip2Sleep;

impl Sleep for Wasip2Sleep {
    fn sleep(&self, duration: core::time::Duration) -> impl Future<Output = ()> + crate::MaybeSend {
        let nanoseconds = u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX);
        let pollable = subscribe_duration(nanoseconds);
        wait(pollable);
        future::ready(())
    }
}

fn send(request: Request<Vec<u8>>) -> Result<Response<Vec<u8>>, Wasip2Error> {
    let (parts, body) = request.into_parts();
    let url =
        wasi_common::url(&parts.uri).map_err(|message| Wasip2Error::BadUrl(message.into()))?;
    let fields = Fields::from_list(&wasi_common::headers(&parts.headers))
        .map_err(|error| Wasip2Error::HeaderConversion(error.to_string()))?;
    let request = OutgoingRequest::new(fields);
    request
        .set_method(&wasi_method(wasi_common::method(&parts.method)))
        .map_err(|()| Wasip2Error::Request("host rejected the HTTP method".into()))?;
    request
        .set_scheme(Some(&wasi_scheme(url.scheme)))
        .map_err(|()| Wasip2Error::BadUrl("host rejected the URL scheme".into()))?;
    request
        .set_authority(Some(&url.authority))
        .map_err(|()| Wasip2Error::BadUrl("host rejected the URL authority".into()))?;
    request
        .set_path_with_query(Some(&url.path_with_query))
        .map_err(|()| Wasip2Error::BadUrl("host rejected the URL path and query".into()))?;

    let outgoing_body = request
        .body()
        .map_err(|()| Wasip2Error::Request("request body was already consumed".into()))?;
    let response = outgoing_handler::handle(request, None)?;
    write_body(outgoing_body, &body)?;

    wait(response.subscribe());
    let response = response
        .get()
        .ok_or_else(|| Wasip2Error::Response("response was not ready after polling".into()))?
        .map_err(|()| Wasip2Error::Response("response was already consumed".into()))??;
    into_http_response(&response)
}

fn wasi_method(method: wasi_common::WasiMethod) -> Method {
    use wasi_common::WasiMethod;

    match method {
        WasiMethod::Get => Method::Get,
        WasiMethod::Head => Method::Head,
        WasiMethod::Post => Method::Post,
        WasiMethod::Put => Method::Put,
        WasiMethod::Delete => Method::Delete,
        WasiMethod::Connect => Method::Connect,
        WasiMethod::Options => Method::Options,
        WasiMethod::Trace => Method::Trace,
        WasiMethod::Patch => Method::Patch,
        WasiMethod::Other(value) => Method::Other(value),
    }
}

fn wasi_scheme(scheme: wasi_common::WasiScheme) -> Scheme {
    use wasi_common::WasiScheme;

    match scheme {
        WasiScheme::Http => Scheme::Http,
        WasiScheme::Https => Scheme::Https,
    }
}

fn write_body(body: OutgoingBody, bytes: &[u8]) -> Result<(), Wasip2Error> {
    let stream = body
        .write()
        .map_err(|()| Wasip2Error::Request("request body stream was already consumed".into()))?;
    let result = write_all(&stream, bytes).and_then(|()| flush(&stream));
    drop(stream);
    result?;
    OutgoingBody::finish(body, None)?;
    Ok(())
}

fn write_all(stream: &OutputStream, bytes: &[u8]) -> Result<(), Wasip2Error> {
    let mut offset = 0;
    while offset < bytes.len() {
        let permitted = stream
            .check_write()
            .map_err(|error| stream_error("checking request body write", error))?;
        if permitted == 0 {
            wait(stream.subscribe());
            continue;
        }
        let permitted = match usize::try_from(permitted) {
            Ok(value) => value,
            Err(_) => usize::MAX,
        };
        let end = offset + permitted.min(bytes.len() - offset);
        stream
            .write(&bytes[offset..end])
            .map_err(|error| stream_error("writing request body", error))?;
        offset = end;
    }
    Ok(())
}

fn flush(stream: &OutputStream) -> Result<(), Wasip2Error> {
    stream
        .flush()
        .map_err(|error| stream_error("flushing request body", error))?;
    wait(stream.subscribe());
    let _ = stream
        .check_write()
        .map_err(|error| stream_error("checking request body flush", error))?;
    Ok(())
}

fn into_http_response(response: &IncomingResponse) -> Result<Response<Vec<u8>>, Wasip2Error> {
    let status = StatusCode::from_u16(response.status())
        .map_err(|error| Wasip2Error::Response(error.to_string()))?;
    let headers = incoming_headers(&response.headers())?;
    let body = response
        .consume()
        .map_err(|()| Wasip2Error::Response("response body was already consumed".into()))?;
    let body = read_body(body)?;
    let mut response = Response::new(body);
    *response.status_mut() = status;
    *response.headers_mut() = headers;
    Ok(response)
}

fn incoming_headers(fields: &Fields) -> Result<HeaderMap, Wasip2Error> {
    let mut headers = HeaderMap::new();
    for (name, value) in fields.entries() {
        let name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|error| Wasip2Error::HeaderConversion(error.to_string()))?;
        let value = HeaderValue::from_bytes(&value)
            .map_err(|error| Wasip2Error::HeaderConversion(error.to_string()))?;
        headers.append(name, value);
    }
    Ok(headers)
}

fn read_body(body: IncomingBody) -> Result<Vec<u8>, Wasip2Error> {
    let stream = body
        .stream()
        .map_err(|()| Wasip2Error::Response("response body stream was already consumed".into()))?;
    let result = read_to_end(&stream);
    drop(stream);
    drop(IncomingBody::finish(body));
    result
}

fn read_to_end(stream: &InputStream) -> Result<Vec<u8>, Wasip2Error> {
    let mut body = Vec::new();
    loop {
        match stream.blocking_read(READ_CHUNK_SIZE) {
            Ok(chunk) if chunk.is_empty() => return Ok(body),
            Ok(chunk) => body.extend_from_slice(&chunk),
            Err(StreamError::Closed) => return Ok(body),
            Err(error) => return Err(stream_error("reading response body", error)),
        }
    }
}

fn wait(pollable: Pollable) {
    drop(poll(&[&pollable]));
    drop(pollable);
}

fn stream_error(context: &str, error: StreamError) -> Wasip2Error {
    let detail = match error {
        StreamError::Closed => "stream closed".into(),
        StreamError::LastOperationFailed(error) => error.to_debug_string(),
    };
    Wasip2Error::Stream(format!("{context}: {detail}"))
}
