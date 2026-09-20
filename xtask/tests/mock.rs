//! End-to-end checks for the fixture-backed mock server.

use std::{process::Stdio, time::Duration};

use reqwest::StatusCode;
use serde_json::{Value, json};
use tokio::{
    io::{AsyncBufReadExt as _, BufReader},
    process::Command,
};

#[path = "../../tests/fixtures/mod.rs"]
mod fixtures;

#[tokio::test]
async fn serves_recorded_api_paths() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["mock", "--port", "0"])
        .stdout(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut lines = BufReader::new(stdout).lines();
    let line = tokio::time::timeout(Duration::from_secs(5), lines.next_line())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let base_url = line.strip_prefix("listening on ").unwrap();
    let client = reqwest::Client::new();

    assert_response(
        client
            .post(format!("{base_url}/v1/systemone"))
            .bearer_auth("test-key")
            .json(&fixtures::load("triage", "request"))
            .send()
            .await
            .unwrap(),
        StatusCode::OK,
        fixtures::load("triage", "response")["body"].clone(),
    )
    .await;
    assert_response(
        client
            .post(format!("{base_url}/v1/systemone"))
            .json(&fixtures::load("triage", "request"))
            .send()
            .await
            .unwrap(),
        StatusCode::UNAUTHORIZED,
        fixtures::load("bad_key", "response")["body"].clone(),
    )
    .await;
    assert_response(
        client
            .post(format!("{base_url}/v1/systemone"))
            .bearer_auth("test-key")
            .json(&json!({"questions": {"other": {}}}))
            .send()
            .await
            .unwrap(),
        StatusCode::BAD_REQUEST,
        json!({"detail": "question ids must match the recorded triage request"}),
    )
    .await;
    assert_response(
        client
            .get(format!("{base_url}/v1/models"))
            .send()
            .await
            .unwrap(),
        StatusCode::OK,
        fixtures::load("models", "response")["body"].clone(),
    )
    .await;
    assert_response(
        client
            .get(format!("{base_url}/other"))
            .send()
            .await
            .unwrap(),
        StatusCode::NOT_FOUND,
        json!({"detail": "Not Found"}),
    )
    .await;

    child.kill().await.unwrap();
}

async fn assert_response(response: reqwest::Response, status: StatusCode, body: Value) {
    assert_eq!(response.status(), status);
    assert_eq!(response.json::<Value>().await.unwrap(), body);
}
