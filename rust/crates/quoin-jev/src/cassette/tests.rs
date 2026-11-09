// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Unit tests for `cassette`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
use std::path::Path;
use std::sync::Arc;

use tempfile::tempdir;
use typesafe_sdk_headers::Headers;
use typesafe_sdk_http::{Exchange, Method, Mock, Request, Transport};
use typesafe_sdk_questions::{Entry, Questions, noul};

use super::{Cassette, request_key};
use crate::client::with_transport;
use crate::config::resolve;
use crate::error::{JevError, JevErrorCode, classify};
use typesafe_sdk_client::SystemOneRequest;
use typesafe_sdk_env::Fixed;

/// Writes `lines` to `path` as JSONL, one value per line.
fn seed(path: &Path, lines: &[serde_json::Value]) {
    let mut text = String::new();
    for line in lines {
        text.push_str(&line.to_string());
        text.push('\n');
    }
    std::fs::write(path, text).expect("seed the cassette");
}

fn line(key: &str, status: u16, response_body: &str) -> serde_json::Value {
    serde_json::json!({"key": key, "status": status, "response_body": response_body})
}

fn replay_error(path: &Path) -> JevError {
    Cassette::replay(path)
        .err()
        .expect("the cassette must refuse to load")
}

fn env() -> Fixed {
    Fixed::new(&[("TYPESAFE_API_KEY", "sk_test_key")])
}

fn request(state: &str) -> SystemOneRequest {
    let mut questions = Questions::new();
    questions.insert("q1".to_owned(), noul("is this true?"));
    SystemOneRequest::new(Entry::from(state), questions)
}

fn success_body(model: &str) -> String {
    serde_json::json!({
        "model": model,
        "answers": {"q1": {"type": "noul", "noul": 0.9}},
        "usage": {"input_tokens": 10, "output_tokens": 0}
    })
    .to_string()
}

/// Provenance: PLAT-977. The acceptance shape itself: record a real call,
/// then replay it with no delegate and no network at all, and get back
/// the identical answer.
#[tokio::test]
async fn a_recorded_call_replays_identically_with_no_delegate() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("fixture.jsonl");
    let body = success_body("jev-1.13.0");

    let mock = Arc::new(Mock::new(vec![Exchange::ok(&body)]));
    let cassette = Cassette::record(&path, mock.clone()).expect("opens for recording");
    let config = resolve(&env()).expect("key present");
    let client = with_transport(config, Arc::new(cassette));
    let recorded = client
        .system_one(request("first"))
        .await
        .expect("the mocked call succeeds");
    assert_eq!(mock.attempts(), 1);

    // No delegate at all now -- a real run would also have no API key.
    let cassette = Cassette::replay(&path).expect("loads for replay");
    let config = resolve(&env()).expect("key present");
    let client = with_transport(config, Arc::new(cassette));
    let replayed = client
        .system_one(request("first"))
        .await
        .expect("replays without a network call");

    assert_eq!(recorded.model, replayed.model);
    assert_eq!(recorded.usage.input_tokens, replayed.usage.input_tokens);
    assert_eq!(recorded.answers, replayed.answers);
}

/// Provenance: PLAT-977. A request whose key is already on file is
/// answered from the cache during recording, not sent again -- re-running
/// a recording session costs no repeat calls.
#[tokio::test]
async fn recording_the_same_request_twice_calls_the_delegate_once() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("fixture.jsonl");
    let body = success_body("jev-1.13.0");
    let mock = Arc::new(Mock::new(vec![Exchange::ok(&body)]));
    let cassette = Cassette::record(&path, mock.clone()).expect("opens for recording");
    let config = resolve(&env()).expect("key present");
    let client = with_transport(config, Arc::new(cassette));

    client
        .system_one(request("same"))
        .await
        .expect("first call");
    client
        .system_one(request("same"))
        .await
        .expect("second call, from cache");

    assert_eq!(mock.attempts(), 1, "the delegate is asked only once");
    let lines = std::fs::read_to_string(&path).expect("file exists");
    assert_eq!(
        lines.lines().count(),
        1,
        "one recording, not a duplicate line"
    );
}

/// Provenance: PLAT-977. A request during replay with no matching
/// recorded line is a named, loud failure, never a silent pass-through.
#[tokio::test]
async fn a_replay_miss_is_a_named_error() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("fixture.jsonl");
    std::fs::write(
        &path,
        format!(
            "{}\n",
            serde_json::json!({
                "key": "some-other-key",
                "status": 200,
                "response_body": success_body("jev-1.13.0"),
            })
        ),
    )
    .expect("seed the cassette");

    let cassette = Cassette::replay(&path).expect("loads");
    let config = resolve(&env()).expect("key present");
    let client = with_transport(config, Arc::new(cassette));
    let sdk_error = client
        .system_one(request("never recorded"))
        .await
        .expect_err("no line matches this request");
    assert_eq!(classify(&sdk_error).code, JevErrorCode::CassetteMiss);
}

/// Provenance: PLAT-977. Two independent inputs hash to two independent
/// keys, so replay cannot cross-answer one request with another's
/// recording.
#[tokio::test]
async fn distinct_requests_record_as_distinct_lines() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("fixture.jsonl");
    let mock = Arc::new(Mock::new(vec![
        Exchange::ok(&success_body("jev-1.13.0")),
        Exchange::ok(&success_body("jev-1.13.0")),
    ]));
    let cassette = Cassette::record(&path, mock).expect("opens for recording");
    let config = resolve(&env()).expect("key present");
    let client = with_transport(config, Arc::new(cassette));

    client.system_one(request("a")).await.expect("first");
    client.system_one(request("b")).await.expect("second");

    let lines = std::fs::read_to_string(&path).expect("file exists");
    assert_eq!(lines.lines().count(), 2, "two distinct requests, two lines");
}

/// Provenance: PLAT-977. A transient failure is returned to the caller
/// but never cached, so a re-run of the recording session retries it
/// instead of replaying the failure forever.
#[tokio::test]
async fn a_failed_response_is_not_recorded() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("fixture.jsonl");
    let mock = Arc::new(Mock::new(vec![
        Exchange::status(500, "{}"),
        Exchange::status(500, "{}"),
        Exchange::status(500, "{}"),
        Exchange::status(500, "{}"),
    ]));
    let cassette = Cassette::record(&path, mock.clone()).expect("opens for recording");
    let config = resolve(&env()).expect("key present");
    let client = with_transport(config, Arc::new(cassette));

    let sdk_error = client
        .system_one(request("flaky"))
        .await
        .expect_err("a repeated 500 surfaces as an error");
    assert_eq!(classify(&sdk_error).code, JevErrorCode::ApiError);
    // The SDK's default RetryPolicy retries every `500..600` status
    // `max_retries` (2) times: three attempts, all reaching the delegate.
    assert_eq!(
        mock.attempts(),
        3,
        "no cached answer short-circuits a retry"
    );
    assert!(!path.exists(), "a failing response leaves no recorded line");
}

/// Provenance: PLAT-977. `record`'s own promise -- re-running a recording
/// session costs no repeat calls -- across two separate `Cassette`
/// instances: the second answers from the file and never touches its
/// (empty, so any call would fail) delegate.
#[tokio::test]
async fn a_second_recording_session_answers_from_the_file_without_the_delegate() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("fixture.jsonl");
    let first = Arc::new(Mock::new(vec![Exchange::ok(&success_body("jev-1.13.0"))]));
    let cassette = Cassette::record(&path, first.clone()).expect("opens");
    let client = with_transport(resolve(&env()).expect("key"), Arc::new(cassette));
    let recorded = client.system_one(request("again")).await.expect("records");
    assert_eq!(first.attempts(), 1);
    drop(client);

    let second = Arc::new(Mock::new(vec![]));
    let cassette = Cassette::record(&path, second.clone()).expect("reopens");
    let client = with_transport(resolve(&env()).expect("key"), Arc::new(cassette));
    let reanswered = client
        .system_one(request("again"))
        .await
        .expect("answered from the file");

    assert_eq!(second.attempts(), 0, "the fresh delegate is never called");
    assert_eq!(recorded.answers, reanswered.answers);
    let text = std::fs::read_to_string(&path).expect("file exists");
    assert_eq!(text.lines().count(), 1, "no duplicate line appended");
}

/// Provenance: PLAT-977. Two lines sharing a key have no principled
/// "current" reading; the whole file refuses to load.
#[test]
fn a_duplicate_key_fails_to_load() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("fixture.jsonl");
    let body = success_body("jev-1.13.0");
    seed(&path, &[line("k1", 200, &body), line("k1", 200, &body)]);

    let error = replay_error(&path);
    assert_eq!(error.code, JevErrorCode::CassetteInvalid);
    assert!(error.message.contains("duplicate key"), "{}", error.message);
}

/// Provenance: PLAT-977. A line that is not JSON at all fails to load.
#[test]
fn a_line_that_is_not_json_fails_to_load() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("fixture.jsonl");
    std::fs::write(&path, "not json\n").expect("seed");

    assert_eq!(replay_error(&path).code, JevErrorCode::CassetteInvalid);
}

/// Provenance: PLAT-977. Valid JSON missing a required field
/// (`response_body`) fails to load.
#[test]
fn a_line_missing_a_required_field_fails_to_load() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("fixture.jsonl");
    seed(&path, &[serde_json::json!({"key": "k1", "status": 200})]);

    assert_eq!(replay_error(&path).code, JevErrorCode::CassetteInvalid);
}

/// Provenance: PLAT-977. A hand-edited non-2xx line never replays as a
/// success.
#[test]
fn a_line_with_a_non_2xx_status_fails_to_load() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("fixture.jsonl");
    seed(&path, &[line("k1", 500, &success_body("jev-1.13.0"))]);

    let error = replay_error(&path);
    assert_eq!(error.code, JevErrorCode::CassetteInvalid);
    assert!(error.message.contains("500"), "{}", error.message);
}

/// Provenance: PLAT-977. Only `NotFound` means "no cassette yet"; any
/// other read failure is `CassetteInvalid`, never an empty cassette.
/// A directory stands in for an unreadable path: `read_to_string` on it
/// fails with a non-`NotFound` kind on every platform, where a
/// `chmod 000` file is still readable when the suite runs as root.
#[test]
fn an_unreadable_path_is_invalid_not_an_empty_cassette() {
    let dir = tempdir().expect("tempdir");

    let error = Cassette::record(dir.path(), Arc::new(Mock::new(vec![])))
        .err()
        .expect("a directory is not an empty cassette");
    assert_eq!(error.code, JevErrorCode::CassetteInvalid);
}

/// Provenance: PLAT-977. A missing file is the state a fresh recording
/// starts from -- and the first append creates its parent directory.
#[tokio::test]
async fn recording_into_a_missing_directory_creates_it() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("nested").join("fixture.jsonl");
    let mock = Arc::new(Mock::new(vec![Exchange::ok(&success_body("jev-1.13.0"))]));
    let cassette = Cassette::record(&path, mock).expect("a missing file is empty");
    let client = with_transport(resolve(&env()).expect("key"), Arc::new(cassette));

    client.system_one(request("fresh")).await.expect("records");
    let text = std::fs::read_to_string(&path).expect("created");
    assert_eq!(text.lines().count(), 1);
}

/// Provenance: PLAT-977. A request with no body cannot be keyed and is
/// refused as `CassetteInvalid`, not sent.
#[tokio::test]
async fn a_request_with_no_body_is_invalid() {
    let dir = tempdir().expect("tempdir");
    let mock = Arc::new(Mock::new(vec![]));
    let cassette = Cassette::record(dir.path().join("fixture.jsonl"), mock.clone()).expect("opens");
    let request = Request {
        method: Method::Get,
        url: "https://example.invalid/".to_owned(),
        headers: Headers::new(),
        body: None,
        timeout_ms: 1_000,
    };

    let sdk_error = cassette.send(request).await.expect_err("no body to key");
    assert_eq!(classify(&sdk_error).code, JevErrorCode::CassetteInvalid);
    assert_eq!(mock.attempts(), 0);
}

/// Provenance: PLAT-977. A body that parses as JSON but is not an object
/// has no `state`/`questions` to key on.
#[test]
fn a_request_body_that_is_not_an_object_is_invalid() {
    for body in ["[1, 2]", r#""just a string""#] {
        let error = request_key(body).expect_err(body);
        assert_eq!(error.code, JevErrorCode::CassetteInvalid, "{body}");
    }
}
