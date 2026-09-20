//! Repository automation and the fixture-backed test server.

use std::{
    convert::Infallible,
    env,
    io::{BufRead as _, BufReader, Write as _},
    path::Path,
    process::{Child, Command, Stdio},
};

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
        Some("wasi-smoke") => wasi_smoke(parse_wasi_target(&mut args)?),
        _ => {
            Err("usage: cargo xtask <mock [--port N] | wasi-smoke --target {wasip2,wasip3}>".into())
        }
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

#[derive(Clone, Copy)]
enum WasiTarget {
    Wasip2,
    Wasip3,
}

impl WasiTarget {
    const fn package(self) -> &'static str {
        match self {
            Self::Wasip2 => "wasip2-example",
            Self::Wasip3 => "wasip3-example",
        }
    }

    const fn triple(self) -> &'static str {
        match self {
            Self::Wasip2 => "wasm32-wasip2",
            Self::Wasip3 => "wasm32-wasip3",
        }
    }

    const fn binary(self) -> &'static str {
        match self {
            Self::Wasip2 => "wasip2_example",
            Self::Wasip3 => "wasip3_example",
        }
    }
}

fn parse_wasi_target(args: &mut impl Iterator<Item = String>) -> Result<WasiTarget, BoxError> {
    let flag = args.next();
    let target = args.next();
    if args.next().is_some() || flag.as_deref() != Some("--target") {
        return Err("usage: cargo xtask wasi-smoke --target {wasip2,wasip3}".into());
    }
    match target.as_deref() {
        Some("wasip2") => Ok(WasiTarget::Wasip2),
        Some("wasip3") => Ok(WasiTarget::Wasip3),
        _ => Err("target must be wasip2 or wasip3".into()),
    }
}

fn wasi_smoke(target: WasiTarget) -> Result<(), BoxError> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("xtask must be inside the workspace")?;
    let mut build = match target {
        WasiTarget::Wasip2 => Command::new("cargo"),
        WasiTarget::Wasip3 => {
            let mut command = Command::new("nix");
            command.args(["develop", ".#nightly", "-c", "cargo"]);
            command
        }
    };
    let status = build
        .args([
            "build",
            "-p",
            target.package(),
            "--target",
            target.triple(),
            "--locked",
        ])
        .current_dir(workspace)
        .status()?;
    require_success(status, &format!("building the {} example", target.triple()))?;

    let mut mock = MockChild(
        Command::new(env::current_exe()?)
            .args(["mock", "--port", "0"])
            .stdout(Stdio::piped())
            .spawn()?,
    );
    let stdout = mock.0.stdout.take().ok_or("mock stdout was not piped")?;
    let mut line = String::new();
    BufReader::new(stdout).read_line(&mut line)?;
    let base_url = line
        .trim()
        .strip_prefix("listening on ")
        .ok_or("mock did not print its listening address")?;

    let output = Command::new("wasmtime")
        .args([
            "run",
            "-S",
            "http",
            "--env",
            "TYPESAFE_API_KEY=test-key",
            "--env",
            &format!("TYPESAFE_BASE_URL={base_url}"),
        ])
        .arg(format!(
            "target/{}/debug/{}.wasm",
            target.triple(),
            target.binary()
        ))
        .current_dir(workspace)
        .output()?;
    require_success(output.status, &String::from_utf8_lossy(&output.stderr))?;
    let output = String::from_utf8(output.stdout)?;
    let (department, frustration) = recorded_picks()?;
    if !output.contains(&format!("department: {department}"))
        || !output.contains(&format!("({frustration})"))
    {
        return Err(format!(
            "{} output did not contain the recorded picks:\n{output}",
            target.triple()
        )
        .into());
    }
    print!("{output}");
    std::io::stdout().flush()?;
    Ok(())
}

fn recorded_picks() -> Result<(String, String), BoxError> {
    let fixture = fixtures::load("triage", "response");
    let body = &fixture["body"]["answers"];
    let department = body["department"]["choice"]
        .as_str()
        .ok_or("recorded department choice is missing")?;
    let score = body["frustration"]["score"]
        .as_f64()
        .ok_or("recorded frustration score is missing")?;
    let nearest = format!("{:.0}", score.round());
    let frustration = body["frustration"]["legend"][nearest]
        .as_str()
        .ok_or("recorded frustration legend is missing")?;
    Ok((department.into(), frustration.into()))
}

fn require_success(status: std::process::ExitStatus, action: &str) -> Result<(), BoxError> {
    if status.success() {
        Ok(())
    } else {
        Err(format!("{action} failed with {status}").into())
    }
}

struct MockChild(Child);

impl Drop for MockChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
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

#[cfg(test)]
mod tests {
    use super::parse_wasi_target;

    #[test]
    fn parses_each_wasi_smoke_target() {
        let mut wasip2_args = ["--target".to_owned(), "wasip2".to_owned()].into_iter();
        let mut wasip3_args = ["--target".to_owned(), "wasip3".to_owned()].into_iter();
        let wasip2 = parse_wasi_target(&mut wasip2_args).unwrap();
        let wasip3 = parse_wasi_target(&mut wasip3_args).unwrap();

        assert_eq!(wasip2.triple(), "wasm32-wasip2");
        assert_eq!(wasip3.triple(), "wasm32-wasip3");
    }

    #[test]
    fn rejects_an_unknown_wasi_smoke_target() {
        let mut args = ["--target".to_owned(), "wasip4".to_owned()].into_iter();
        let error = parse_wasi_target(&mut args).err().unwrap();

        assert_eq!(error.to_string(), "target must be wasip2 or wasip3");
    }
}
