//! Repository automation and the fixture-backed test server.

use std::{convert::Infallible, env, io::Write as _};

use bytes::Bytes;
use http_body_util::{BodyExt as _, Full};
use hyper::{Method, Request, Response, StatusCode, body::Incoming, service::service_fn};
use hyper_util::rt::TokioIo;
use serde_json::{Value, json};
use tokio::net::{TcpListener, TcpStream};

#[path = "../../tests/fixtures/mod.rs"]
mod fixtures;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("mock") => serve(parse_port(&mut args)?).await,
        _ => Err("usage: cargo xtask mock [--port N]".into()),
    }
}

fn parse_port(args: &mut impl Iterator<Item = String>) -> Result<u16, BoxError> {
    match (args.next().as_deref(), args.next()) {
        (None, None) => Ok(3000),
        (Some("--port"), Some(port)) if args.next().is_none() => Ok(port.parse()?),
        _ => Err("usage: cargo xtask mock [--port N]".into()),
    }
}

async fn serve(port: u16) -> Result<(), BoxError> {
    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    let address = listener.local_addr()?;
    println!("listening on http://{address}");
    std::io::stdout().flush()?;

    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(serve_connection(stream));
    }
}

async fn serve_connection(stream: TcpStream) {
    let result = hyper::server::conn::http1::Builder::new()
        .serve_connection(
            TokioIo::new(stream),
            service_fn(|request| async move { Ok::<_, Infallible>(handle(request).await) }),
        )
        .await;
    if let Err(error) = result {
        eprintln!("mock connection failed: {error}");
    }
}

async fn handle(request: Request<Incoming>) -> Response<Full<Bytes>> {
    match (request.method(), request.uri().path()) {
        (&Method::POST, "/v1/systemone") => systemone(request).await,
        (&Method::GET, "/v1/models") => fixture_response("models"),
        _ => json_response(StatusCode::NOT_FOUND, &json!({"detail": "Not Found"})),
    }
}

async fn systemone(request: Request<Incoming>) -> Response<Full<Bytes>> {
    if request
        .headers()
        .get(hyper::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        != Some("Bearer test-key")
    {
        return fixture_response("bad_key");
    }

    let body = match request.into_body().collect().await {
        Ok(body) => body.to_bytes(),
        Err(error) => return bad_request(format!("invalid request body: {error}")),
    };
    let request: Value = match serde_json::from_slice(&body) {
        Ok(request) => request,
        Err(error) => return bad_request(format!("invalid JSON: {error}")),
    };
    if question_ids(&request) != question_ids(&fixtures::load("triage", "request")) {
        return bad_request("question ids must match the recorded triage request");
    }
    fixture_response("triage")
}

fn question_ids(request: &Value) -> Option<Vec<&str>> {
    let mut ids = request
        .get("questions")?
        .as_object()?
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    Some(ids)
}

fn bad_request(detail: impl Into<String>) -> Response<Full<Bytes>> {
    json_response(StatusCode::BAD_REQUEST, &json!({"detail": detail.into()}))
}

fn fixture_response(name: &str) -> Response<Full<Bytes>> {
    let fixture = fixtures::load(name, "response");
    let status = fixture["status"]
        .as_u64()
        .and_then(|status| u16::try_from(status).ok())
        .and_then(|status| StatusCode::from_u16(status).ok())
        .unwrap_or_else(|| panic!("fixture {name}.response.json has an invalid status"));
    json_response(status, &fixture["body"])
}

fn json_response(status: StatusCode, body: &Value) -> Response<Full<Bytes>> {
    let mut response = Response::new(Full::new(Bytes::from(body.to_string())));
    *response.status_mut() = status;
    response.headers_mut().insert(
        hyper::header::CONTENT_TYPE,
        hyper::header::HeaderValue::from_static("application/json"),
    );
    response
}
