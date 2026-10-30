// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Record/replay for Jev responses, so a live-API test suite can run for free
//! after one real recording (PLAT-977).
//!
//! **The gap this closes.** Every existing Jev mock in this crate
//! (`Exchange::ok`/`Exchange::status` in `client.rs`/`lens.rs`) is hand-written
//! per test. There has been no way to record a real API answer once and replay
//! it for free afterward -- every live-API test either spends a call or
//! carries a fixture someone typed by hand. [`Cassette`] is a
//! [`Transport`] that does that: it sits where [`typesafe_sdk_http::Reqwest`]
//! or [`typesafe_sdk_http::Mock`] would, keyed by a canonical hash of the
//! request's `state`/`questions`, and either records a real call once or
//! replays a prior recording with no socket at all.
//!
//! **Keying.** The key is the RFC 8785 canonical-JSON text of
//! `{state, questions}` itself ([`quoin_store::canonical_json`]), compared as a
//! string: nothing is hashed, so there is no digest to record or to drift.
//! `model` is deliberately excluded from the key: a cassette line is keyed on
//! what was asked, not on which model answered it.
//!
//! A cassette records no model pin and checks none: which model answered is
//! whatever the recorded response body says, and a caller that cares reads
//! `response.model` from it.
//!
//! **This is not how to measure repetition.** M1
//! (`spec/assurance/MP-225-jev-disagreement-under-repetition.md`) repeats an
//! identical request against the *live* service N times specifically to see
//! whether the answer changes. A cassette holds exactly one recorded answer
//! per distinct request, by construction -- replaying it N times would report
//! 0% disagreement because the transport cannot vary, not because the model
//! did not. A caller measuring repetition stays on the live transport.

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write as _};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use quoin_store::{JsonObject, JsonValue, canonical_json, parse_strict_json_str};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use typesafe_sdk_error::{Error as SdkError, Result as SdkResult};
use typesafe_sdk_headers::Headers;
use typesafe_sdk_http::{RawResponse, Request, Transport};

use crate::error::{JevError, JevErrorCode, Result};

/// The prefix [`crate::error::classify`] recognises to report a cassette miss
/// under its own code rather than the generic `Connection` fallback every
/// other `Invalid` message classifies as.
///
/// `typesafe_sdk_error::Error` is a closed, foreign enum with no variant of
/// its own for "a `Transport` outside the SDK refused this request" --
/// `Invalid(String)` is the only shape available at the `Transport` boundary.
/// Prefixing the message is the only seam through which this crate's own
/// named codes can still reach a caller from inside [`Cassette::send`].
pub(crate) const MISS_PREFIX: &str = "JEV_CASSETTE_MISS: ";

/// As [`MISS_PREFIX`], for a malformed cassette file or an unrecordable
/// request discovered inside [`Cassette::send`] itself (load-time problems go
/// through [`Cassette::record`]/[`Cassette::replay`]'s own `Result` instead,
/// and never need this prefix at all).
pub(crate) const INVALID_PREFIX: &str = "JEV_CASSETTE_INVALID: ";

/// One recorded request/response pair, as one line of the cassette's JSONL
/// file.
///
/// `response_body` is the exact bytes the service returned, not a
/// re-serialisation of a parsed [`typesafe_sdk_answers::SystemOneResponse`]:
/// replay must hand back what was actually received. The two happen to agree
/// today (`SystemOneResponse` round-trips through serde), but nothing here
/// depends on that continuing to be true.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CassetteLine {
    /// The canonical text of this request's `{state, questions}` -- see
    /// [`request_key`].
    key: String,
    /// The HTTP status the real call answered with. Only a `2xx` response is
    /// ever recorded (see [`Cassette::send`]), and [`load_lines`] refuses a
    /// line outside that range, so a hand-edited `500` cannot replay as a
    /// success.
    status: u16,
    /// The response body, byte for byte.
    response_body: String,
}

/// What [`Cassette::send`] does when a request's key is not already on file.
enum Mode {
    /// Sends the real request through `delegate`, then appends the exchange
    /// so the next call with the same key needs neither the network nor the
    /// key. Only a `2xx` response is ever appended -- a transient failure is
    /// returned to the caller but never cached, so a re-run of the recording
    /// session retries it rather than replaying the failure forever.
    ///
    /// `cache` is an async mutex held across the whole
    /// check-miss/call-delegate/append sequence: with the lock released
    /// before the delegate call, two concurrent requests for one key both
    /// missed, both spent a live call, and both appended -- and the duplicate
    /// key then made the whole file refuse to load. That serialises every
    /// record-mode call, which is the right trade for a recording harness.
    Record {
        delegate: Arc<dyn Transport>,
        cache: Mutex<HashMap<String, CassetteLine>>,
    },
    /// Answers only from what [`Cassette::replay`] already loaded. A miss is
    /// [`JevErrorCode::CassetteMiss`], never a silent fall-through -- there is
    /// no delegate here to fall through to.
    Replay {
        lines: HashMap<String, CassetteLine>,
    },
}

/// A [`Transport`] that records real Jev answers to a JSONL file, keyed by the
/// canonical text of the request, or replays them with no socket at all.
///
/// Install it exactly where a real or mocked transport goes:
/// `client::with_transport(config, Arc::new(cassette))`.
pub struct Cassette {
    path: PathBuf,
    mode: Mode,
}

impl Cassette {
    /// Opens `path` for recording, wrapping `delegate` -- typically
    /// [`typesafe_sdk_http::Reqwest`] for a genuine live recording, though any
    /// transport works (including another [`Cassette`], for re-recording a
    /// subset of a larger one).
    ///
    /// Existing lines in `path`, if any, are loaded first: a request whose
    /// key is already on file is answered from that line rather than sent
    /// again, so re-running a recording session costs no repeat calls and
    /// writes no duplicate lines.
    ///
    /// # Errors
    /// [`JevErrorCode::CassetteInvalid`] if `path` exists but cannot be read,
    /// or a line in it is not the documented shape.
    pub fn record(path: impl Into<PathBuf>, delegate: Arc<dyn Transport>) -> Result<Self> {
        let path = path.into();
        let cache = load_lines(&path)?;
        Ok(Self {
            path,
            mode: Mode::Record {
                delegate,
                cache: Mutex::new(cache),
            },
        })
    }

    /// Loads `path` for replay.
    ///
    /// # Errors
    /// [`JevErrorCode::CassetteInvalid`] if `path` does not exist, cannot be
    /// read, is empty, or a line is not the documented shape.
    pub fn replay(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let lines = load_lines(&path)?;
        if lines.is_empty() {
            return Err(JevError::new(
                JevErrorCode::CassetteInvalid,
                format!("{}: no recorded lines to replay", path.display()),
            ));
        }
        Ok(Self {
            path,
            mode: Mode::Replay { lines },
        })
    }
}

#[async_trait]
impl Transport for Cassette {
    async fn send(&self, request: Request) -> SdkResult<RawResponse> {
        let Some(body) = request.body.as_deref() else {
            return Err(SdkError::Invalid(format!(
                "{INVALID_PREFIX}the cassette only records/replays a request that carries a body"
            )));
        };
        let key = request_key(body).map_err(to_sdk_error)?;

        match &self.mode {
            Mode::Record { delegate, cache } => {
                // Held until this arm returns -- across the delegate call and
                // the append -- so one key reaches the delegate at most once.
                let mut cache = cache.lock().await;
                if let Some(line) = cache.get(&key) {
                    return Ok(to_raw_response(line));
                }
                let response = delegate.send(request).await?;
                if (200..300).contains(&response.status) {
                    let line = CassetteLine {
                        key,
                        status: response.status,
                        response_body: response.body.clone(),
                    };
                    append_line(&self.path, &line).map_err(to_sdk_error)?;
                    cache.insert(line.key.clone(), line);
                }
                Ok(response)
            }
            Mode::Replay { lines } => {
                let line = lines.get(&key).ok_or_else(|| {
                    SdkError::Invalid(format!(
                        "{MISS_PREFIX}no recorded line matches this request"
                    ))
                })?;
                Ok(to_raw_response(line))
            }
        }
    }
}

/// Builds the [`RawResponse`] a cassette line replays as. Cassette lines
/// carry no headers (nothing this crate reads comes from one -- see
/// `client.rs`'s `Client::system_one`, the short form every caller here
/// uses, which discards them), so replay always answers with an empty set.
fn to_raw_response(line: &CassetteLine) -> RawResponse {
    RawResponse {
        status: line.status,
        headers: Headers::new(),
        body: line.response_body.clone(),
    }
}

/// Carries a [`JevError`] raised inside [`Cassette::send`] across the
/// `Transport` boundary under the prefix [`crate::error::classify`] maps back
/// to the same code. Only the cassette's own codes have a prefix; nothing in
/// this module raises any other, and the exhaustive `match` makes a new code
/// a decision here rather than a silent fold into `CassetteInvalid`.
fn to_sdk_error(error: JevError) -> SdkError {
    let JevError { code, message } = error;
    let prefix = match code {
        JevErrorCode::CassetteMiss => MISS_PREFIX,
        JevErrorCode::CassetteInvalid
        | JevErrorCode::MissingKey
        | JevErrorCode::InvalidConfig
        | JevErrorCode::TransportInit
        | JevErrorCode::Unauthorized
        | JevErrorCode::Validation
        | JevErrorCode::RateLimited
        | JevErrorCode::ApiError
        | JevErrorCode::Connection
        | JevErrorCode::SchemaUnavailable
        | JevErrorCode::SchemaUnreadable
        | JevErrorCode::SpanBallotEmpty
        | JevErrorCode::SpanBallotTooLarge => INVALID_PREFIX,
    };
    SdkError::Invalid(format!("{prefix}{message}"))
}

/// Loads every line of `path` into a map keyed by [`CassetteLine::key`], or an
/// empty map when `path` does not exist yet (the state a fresh recording
/// starts from). The one load path [`Cassette::record`] and
/// [`Cassette::replay`] share, so both apply the same status check.
///
/// # Errors
/// [`JevErrorCode::CassetteInvalid`] if `path` exists but cannot be read (any
/// I/O error other than `NotFound`, permission denied included -- that is not
/// "no cassette yet"), a line is not valid JSON in the documented shape, a
/// line's status is not `2xx`, or two lines share a key -- a duplicate has no principled "current" reading,
/// so this refuses to guess rather than silently keeping whichever line
/// happened to parse last.
fn load_lines(path: &Path) -> Result<HashMap<String, CassetteLine>> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(HashMap::new()),
        Err(error) => {
            return Err(JevError::new(
                JevErrorCode::CassetteInvalid,
                format!("{}: {error}", path.display()),
            ));
        }
    };
    let mut lines = HashMap::new();
    for (index, raw) in text.lines().enumerate() {
        if raw.trim().is_empty() {
            continue;
        }
        let at = format!("{}:{}", path.display(), index + 1);
        let line: CassetteLine = serde_json::from_str(raw).map_err(|error| {
            JevError::new(JevErrorCode::CassetteInvalid, format!("{at}: {error}"))
        })?;
        if !(200..300).contains(&line.status) {
            return Err(JevError::new(
                JevErrorCode::CassetteInvalid,
                format!(
                    "{at}: recorded status {} is not 2xx; only a success is ever recorded",
                    line.status
                ),
            ));
        }
        if lines.insert(line.key.clone(), line).is_some() {
            return Err(JevError::new(
                JevErrorCode::CassetteInvalid,
                format!("{at}: duplicate key; refusing to guess which line is current"),
            ));
        }
    }
    Ok(lines)
}

/// Appends one line to `path`, creating the file (and its parent directory)
/// if this is the first recording.
fn append_line(path: &Path, line: &CassetteLine) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| {
            JevError::new(
                JevErrorCode::CassetteInvalid,
                format!("{}: {error}", parent.display()),
            )
        })?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| {
            JevError::new(
                JevErrorCode::CassetteInvalid,
                format!("{}: {error}", path.display()),
            )
        })?;
    // The whole line, newline included, goes out in ONE `write_all` on an
    // `O_APPEND` handle. `writeln!` issued the JSON and the `\n` as separate
    // writes, which two concurrent appenders could interleave into a
    // malformed line.
    let mut bytes = serde_json::to_vec(line)
        .map_err(|error| JevError::new(JevErrorCode::CassetteInvalid, error.to_string()))?;
    bytes.push(b'\n');
    file.write_all(&bytes).map_err(|error| {
        JevError::new(
            JevErrorCode::CassetteInvalid,
            format!("{}: {error}", path.display()),
        )
    })
}

/// The canonical key for a request whose serialised JSON body is `body`:
/// the canonical JSON text of `{"state": ..., "questions": ...}`, both taken
/// from `body` and defaulting to `null` when the field is absent.
///
/// `model` and `extra` (`request.rs`'s pass-through field) are deliberately
/// excluded -- see this module's doc for why the key is what was asked, not
/// which model or transport option answered it.
fn request_key(body: &str) -> Result<String> {
    let value = parse_strict_json_str(body).map_err(|error| {
        JevError::new(
            JevErrorCode::CassetteInvalid,
            format!("request body is not strict JSON: {error}"),
        )
    })?;
    let object = value
        .as_object()
        .map_err(|error| JevError::new(JevErrorCode::CassetteInvalid, error.to_string()))?;

    let mut keyed = JsonObject::new();
    keyed
        .insert(
            "state",
            object.get("state").cloned().unwrap_or(JsonValue::Null),
        )
        .map_err(|error| JevError::new(JevErrorCode::CassetteInvalid, error.to_string()))?;
    keyed
        .insert(
            "questions",
            object.get("questions").cloned().unwrap_or(JsonValue::Null),
        )
        .map_err(|error| JevError::new(JevErrorCode::CassetteInvalid, error.to_string()))?;

    canonical_json(&JsonValue::Object(keyed))
        .map_err(|error| JevError::new(JevErrorCode::CassetteInvalid, error.to_string()))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
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
        let cassette =
            Cassette::record(dir.path().join("fixture.jsonl"), mock.clone()).expect("opens");
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
}
