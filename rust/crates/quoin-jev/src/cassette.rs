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
//! **Hashing.** The key is [`quoin_store::digest_canonical_value`] over
//! `{state, questions}` -- the RFC 8785 canonical-JSON/blake3 digest this
//! workspace already has one of. The ticket that opened this module named
//! `sha256` by analogy with a third-party project's own cassette; this crate
//! reuses the canonicalizer quoin already ships instead of adding a second,
//! differently-implemented one for the same job (`rust-review`'s "one fact,
//! one place"). `model` is deliberately excluded from the key: a cassette line
//! is keyed on what was asked, not on which model answered it, so a
//! model-mismatch is something [`Cassette::replay`] can detect and refuse
//! (see below) rather than something the key silently routes around.
//!
//! **Fail closed on model skew.** [`Cassette::replay`] checks every line's
//! recorded `model` against the caller's `expected_model` before returning --
//! not lazily, per request. A cassette that mixes recordings from two model
//! versions fails to load in full, rather than silently scoring some
//! fraction of a run against a model no line here actually agrees on. This is
//! PLAT-977's own acceptance language, and it is also half of PLAT-978 (fail
//! closed on unexpected Jev model version): the other half -- the same check
//! against a genuinely live, non-cassette response -- belongs to that
//! ticket's own change to the lens execution path, not to this module.
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
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use quoin_store::{JsonObject, JsonValue, digest_canonical_value, parse_strict_json_str};
use serde::{Deserialize, Serialize};
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
    /// The canonical hash of this request's `{state, questions}` -- see
    /// [`request_key`].
    key: String,
    /// The HTTP status the real call answered with. Only a `2xx` response is
    /// ever recorded (see [`Cassette::send`]), so this is always in that
    /// range for a line that made it to disk.
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

/// A [`Transport`] that records real Jev answers to a JSONL file, keyed by a
/// canonical hash of the request, or replays them with no socket at all.
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

    /// Loads `path` for replay. Every line's recorded `model` is checked
    /// against `expected_model` before this returns -- not lazily, on first
    /// use -- so a cassette recorded against a different model fails to load
    /// in full, rather than scoring some fraction of a run against a model no
    /// line here actually agrees on (PLAT-977, PLAT-978).
    ///
    /// # Errors
    /// [`JevErrorCode::CassetteInvalid`] if `path` cannot be read, is empty,
    /// or a line is not the documented shape;
    /// [`JevErrorCode::CassetteModelMismatch`] if any line's recorded model is
    /// not `expected_model`.
    pub fn replay(path: impl Into<PathBuf>, expected_model: &str) -> Result<Self> {
        let path = path.into();
        let lines = load_lines(&path)?;
        if lines.is_empty() {
            return Err(JevError::new(
                JevErrorCode::CassetteInvalid,
                format!("{}: no recorded lines to replay", path.display()),
            ));
        }
        for line in lines.values() {
            if let Some(model) = response_model(&line.response_body)
                && model != expected_model
            {
                return Err(JevError::new(
                    JevErrorCode::CassetteModelMismatch,
                    format!(
                        "{}: a line was recorded against model {model:?}, expected {expected_model:?}; refusing to load any of it",
                        path.display()
                    ),
                ));
            }
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
        let Some(body) = request.body.clone() else {
            return Err(SdkError::Invalid(format!(
                "{INVALID_PREFIX}the cassette only records/replays a request that carries a body"
            )));
        };
        let key = request_key(&body)
            .map_err(|error| SdkError::Invalid(format!("{INVALID_PREFIX}{}", error.message)))?;

        match &self.mode {
            Mode::Record { delegate, cache } => {
                if let Some(line) = cache_get(cache, &key) {
                    return Ok(to_raw_response(&line));
                }
                let response = delegate.send(request).await?;
                if (200..300).contains(&response.status) {
                    let line = CassetteLine {
                        key: key.clone(),
                        status: response.status,
                        response_body: response.body.clone(),
                    };
                    append_line(&self.path, &line).map_err(|error| {
                        SdkError::Invalid(format!("{INVALID_PREFIX}{}", error.message))
                    })?;
                    cache_insert(cache, line);
                }
                Ok(response)
            }
            Mode::Replay { lines } => {
                let line = lines.get(&key).ok_or_else(|| {
                    SdkError::Invalid(format!(
                        "{MISS_PREFIX}no recorded line matches this request (key {key})"
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

fn cache_get(cache: &Mutex<HashMap<String, CassetteLine>>, key: &str) -> Option<CassetteLine> {
    match cache.lock() {
        Ok(guard) => guard.get(key).cloned(),
        Err(poisoned) => poisoned.into_inner().get(key).cloned(),
    }
}

fn cache_insert(cache: &Mutex<HashMap<String, CassetteLine>>, line: CassetteLine) {
    match cache.lock() {
        Ok(mut guard) => {
            guard.insert(line.key.clone(), line);
        }
        Err(poisoned) => {
            poisoned.into_inner().insert(line.key.clone(), line);
        }
    }
}

/// Loads every line of `path` into a map keyed by [`CassetteLine::key`], or an
/// empty map when `path` does not exist yet (the state a fresh recording
/// starts from).
///
/// # Errors
/// [`JevErrorCode::CassetteInvalid`] if `path` exists but cannot be read, a
/// line is not valid JSON in the documented shape, or two lines share a key --
/// a duplicate has no principled "current" reading, so this refuses to guess
/// rather than silently keeping whichever line happened to parse last.
fn load_lines(path: &Path) -> Result<HashMap<String, CassetteLine>> {
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let text = fs::read_to_string(path).map_err(|error| {
        JevError::new(
            JevErrorCode::CassetteInvalid,
            format!("{}: {error}", path.display()),
        )
    })?;
    let mut lines = HashMap::new();
    for (number, raw) in text.lines().enumerate() {
        if raw.trim().is_empty() {
            continue;
        }
        let line: CassetteLine = serde_json::from_str(raw).map_err(|error| {
            JevError::new(
                JevErrorCode::CassetteInvalid,
                format!("{}:{}: {error}", path.display(), number + 1),
            )
        })?;
        if lines.insert(line.key.clone(), line).is_some() {
            return Err(JevError::new(
                JevErrorCode::CassetteInvalid,
                format!(
                    "{}:{}: duplicate key; refusing to guess which line is current",
                    path.display(),
                    number + 1
                ),
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
    let text = serde_json::to_string(line)
        .map_err(|error| JevError::new(JevErrorCode::CassetteInvalid, error.to_string()))?;
    writeln!(file, "{text}").map_err(|error| {
        JevError::new(
            JevErrorCode::CassetteInvalid,
            format!("{}: {error}", path.display()),
        )
    })
}

/// The canonical key for a request whose serialised JSON body is `body`:
/// [`digest_canonical_value`] over `{"state": ..., "questions": ...}`, both
/// taken from `body` and defaulting to `null` when the field is absent.
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

    let digest = digest_canonical_value(&JsonValue::Object(keyed))
        .map_err(|error| JevError::new(JevErrorCode::CassetteInvalid, error.to_string()))?;
    Ok(digest.as_hex().to_owned())
}

/// The `model` field of a response body, when it parses as an object that has
/// one. `None` for anything else (a non-2xx error envelope, most notably) --
/// there is nothing to check a model-pin against on a line that never
/// answered as System One in the first place.
fn response_model(body: &str) -> Option<String> {
    let value = parse_strict_json_str(body).ok()?;
    let object = value.as_object().ok()?;
    object.get("model")?.as_str().map(str::to_owned)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use std::sync::Arc;

    use tempfile::tempdir;
    use typesafe_sdk_http::{Exchange, Mock};
    use typesafe_sdk_questions::{Entry, Questions, noul};

    use super::Cassette;
    use crate::client::with_transport;
    use crate::config::resolve;
    use crate::error::{JevErrorCode, classify};
    use typesafe_sdk_client::SystemOneRequest;
    use typesafe_sdk_env::Fixed;

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
        let cassette = Cassette::replay(&path, "jev-1.13.0").expect("loads for replay");
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

        let cassette = Cassette::replay(&path, "jev-1.13.0").expect("loads");
        let config = resolve(&env()).expect("key present");
        let client = with_transport(config, Arc::new(cassette));
        let sdk_error = client
            .system_one(request("never recorded"))
            .await
            .expect_err("no line matches this request");
        assert_eq!(classify(&sdk_error).code, JevErrorCode::CassetteMiss);
    }

    /// Provenance: PLAT-977, PLAT-978. A cassette carrying a line recorded
    /// against a different model fails to load in full -- before any request
    /// is even sent -- rather than silently answering some requests from a
    /// model the caller never pinned.
    #[tokio::test]
    async fn a_cassette_with_a_mismatched_model_fails_to_load() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("fixture.jsonl");
        let body = success_body("jev-1.13.0");
        let mock = Arc::new(Mock::new(vec![Exchange::ok(&body)]));
        let cassette = Cassette::record(&path, mock).expect("opens for recording");
        let config = resolve(&env()).expect("key present");
        let client = with_transport(config, Arc::new(cassette));
        client
            .system_one(request("one"))
            .await
            .expect("records against jev-1.13.0");

        let error = Cassette::replay(&path, "jev-2.0.0")
            .err()
            .expect("the pinned model never appears");
        assert_eq!(error.code, JevErrorCode::CassetteModelMismatch);
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

        let _ = client.system_one(request("flaky")).await;
        assert!(
            !path.exists() || std::fs::read_to_string(&path).expect("readable").is_empty(),
            "a failing response leaves no recorded line"
        );
    }
}
