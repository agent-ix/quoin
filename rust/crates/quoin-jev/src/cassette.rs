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
//! **Fail closed on model skew.** Both [`Cassette::record`] and
//! [`Cassette::replay`] take the caller's `expected_model` and check every
//! line's recorded `model` against it when the file is loaded -- not lazily,
//! per request. A cassette that mixes recordings from two model versions
//! fails to load in full, rather than silently scoring some fraction of a run
//! against a model no line here actually agrees on. A line whose body carries
//! no readable `model` at all fails the same way (as `CassetteInvalid`): record
//! mode only ever persists a `2xx` System One answer, so such a line was not
//! written by this module. Record mode also checks each fresh answer before
//! appending it, so a session cannot write a line its own next load would
//! refuse. This is PLAT-977's own acceptance language, and it is also half of
//! PLAT-978 (fail closed on unexpected Jev model version): the other half --
//! the same check against a genuinely live, non-cassette response -- belongs
//! to that ticket's own change to the lens execution path, not to this module.
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
use quoin_store::{JsonObject, JsonValue, digest_canonical_value, parse_strict_json_str};
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

/// As [`MISS_PREFIX`], for a fresh answer that record mode refuses to append
/// because it names a model other than the one the session was pinned to.
pub(crate) const MODEL_MISMATCH_PREFIX: &str = "JEV_CASSETTE_MODEL_MISMATCH: ";

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
        expected_model: String,
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
    /// writes no duplicate lines. Every loaded line's recorded `model` is
    /// checked against `expected_model` exactly as [`Cassette::replay`]
    /// checks it, so a session re-run after the pinned model moved refuses to
    /// serve stale answers from the old one; and every fresh answer is checked
    /// the same way before it is appended.
    ///
    /// # Errors
    /// [`JevErrorCode::CassetteInvalid`] if `path` exists but cannot be read,
    /// or a line in it is not the documented shape;
    /// [`JevErrorCode::CassetteModelMismatch`] if any line's recorded model is
    /// not `expected_model`.
    pub fn record(
        path: impl Into<PathBuf>,
        delegate: Arc<dyn Transport>,
        expected_model: &str,
    ) -> Result<Self> {
        let path = path.into();
        let cache = load_lines(&path, expected_model)?;
        Ok(Self {
            path,
            mode: Mode::Record {
                delegate,
                expected_model: expected_model.to_owned(),
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
    /// [`JevErrorCode::CassetteInvalid`] if `path` does not exist, cannot be
    /// read, is empty, or a line is not the documented shape (including a
    /// line whose body carries no readable `model`);
    /// [`JevErrorCode::CassetteModelMismatch`] if any line's recorded model is
    /// not `expected_model`.
    pub fn replay(path: impl Into<PathBuf>, expected_model: &str) -> Result<Self> {
        let path = path.into();
        let lines = load_lines(&path, expected_model)?;
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
            Mode::Record {
                delegate,
                expected_model,
                cache,
            } => {
                // Held until this arm returns -- across the delegate call and
                // the append -- so one key reaches the delegate at most once.
                let mut cache = cache.lock().await;
                if let Some(line) = cache.get(&key) {
                    return Ok(to_raw_response(line));
                }
                let response = delegate.send(request).await?;
                if (200..300).contains(&response.status) {
                    check_model(&response.body, expected_model, &key).map_err(to_sdk_error)?;
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

/// Carries a [`JevError`] raised inside [`Cassette::send`] across the
/// `Transport` boundary under the prefix [`crate::error::classify`] maps back
/// to the same code. Only the cassette's own codes have a prefix; nothing in
/// this module raises any other, and the exhaustive `match` makes a new code
/// a decision here rather than a silent fold into `CassetteInvalid`.
fn to_sdk_error(error: JevError) -> SdkError {
    let JevError { code, message } = error;
    let prefix = match code {
        JevErrorCode::CassetteModelMismatch => MODEL_MISMATCH_PREFIX,
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
        | JevErrorCode::SchemaUnreadable => INVALID_PREFIX,
    };
    SdkError::Invalid(format!("{prefix}{message}"))
}

/// Loads every line of `path` into a map keyed by [`CassetteLine::key`], or an
/// empty map when `path` does not exist yet (the state a fresh recording
/// starts from). The one load path [`Cassette::record`] and
/// [`Cassette::replay`] share, so both apply the same status and model
/// checks.
///
/// # Errors
/// [`JevErrorCode::CassetteInvalid`] if `path` exists but cannot be read (any
/// I/O error other than `NotFound`, permission denied included -- that is not
/// "no cassette yet"), a line is not valid JSON in the documented shape, a
/// line's status is not `2xx`, a line's body carries no readable `model`, or
/// two lines share a key -- a duplicate has no principled "current" reading,
/// so this refuses to guess rather than silently keeping whichever line
/// happened to parse last.
/// [`JevErrorCode::CassetteModelMismatch`] if a line's recorded model is not
/// `expected_model`.
fn load_lines(path: &Path, expected_model: &str) -> Result<HashMap<String, CassetteLine>> {
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
        check_model(&line.response_body, expected_model, &at)?;
        if lines.insert(line.key.clone(), line).is_some() {
            return Err(JevError::new(
                JevErrorCode::CassetteInvalid,
                format!("{at}: duplicate key; refusing to guess which line is current"),
            ));
        }
    }
    Ok(lines)
}

/// Checks that `body`, a System One response body, names `expected_model`.
/// `at` locates the body in the error message (a `path:line`, or a request
/// key for a fresh answer).
///
/// # Errors
/// [`JevErrorCode::CassetteInvalid`] if `body` is not strict JSON, not an
/// object, or has no string `model` -- nothing record mode persists looks like
/// that, so it fails closed rather than skipping the check;
/// [`JevErrorCode::CassetteModelMismatch`] if the model differs.
fn check_model(body: &str, expected_model: &str, at: &str) -> Result<()> {
    let invalid = |why: &str| {
        JevError::new(
            JevErrorCode::CassetteInvalid,
            format!("{at}: response body {why}; cannot check it against model {expected_model:?}"),
        )
    };
    let value = parse_strict_json_str(body).map_err(|_| invalid("is not strict JSON"))?;
    let object = value
        .as_object()
        .map_err(|_| invalid("is not a JSON object"))?;
    let model = object
        .get("model")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| invalid("has no string `model` field"))?;
    if model != expected_model {
        return Err(JevError::new(
            JevErrorCode::CassetteModelMismatch,
            format!(
                "{at}: recorded against model {model:?}, expected {expected_model:?}; refusing to load any of it"
            ),
        ));
    }
    Ok(())
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

    fn replay_error(path: &Path, expected_model: &str) -> JevError {
        Cassette::replay(path, expected_model)
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
        let cassette =
            Cassette::record(&path, mock.clone(), "jev-1.13.0").expect("opens for recording");
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
        let cassette =
            Cassette::record(&path, mock.clone(), "jev-1.13.0").expect("opens for recording");
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
        let cassette = Cassette::record(&path, mock, "jev-1.13.0").expect("opens for recording");
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
        let cassette = Cassette::record(&path, mock, "jev-1.13.0").expect("opens for recording");
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
        let cassette =
            Cassette::record(&path, mock.clone(), "jev-1.13.0").expect("opens for recording");
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
        let cassette = Cassette::record(&path, first.clone(), "jev-1.13.0").expect("opens");
        let client = with_transport(resolve(&env()).expect("key"), Arc::new(cassette));
        let recorded = client.system_one(request("again")).await.expect("records");
        assert_eq!(first.attempts(), 1);
        drop(client);

        let second = Arc::new(Mock::new(vec![]));
        let cassette = Cassette::record(&path, second.clone(), "jev-1.13.0").expect("reopens");
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

    /// Provenance: PLAT-977. A recording session re-run after the pinned
    /// model moved refuses the old file instead of silently serving stale
    /// answers from the previous model.
    #[test]
    fn recording_over_a_cassette_from_another_model_fails_to_load() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("fixture.jsonl");
        seed(&path, &[line("k1", 200, &success_body("jev-1.13.0"))]);

        let error = Cassette::record(&path, Arc::new(Mock::new(vec![])), "jev-2.0.0")
            .err()
            .expect("the file names another model");
        assert_eq!(error.code, JevErrorCode::CassetteModelMismatch);
    }

    /// Provenance: PLAT-977. A fresh answer from a model other than the
    /// session's pin is refused and never appended, so a session cannot
    /// write a line its own next load would refuse.
    #[tokio::test]
    async fn a_fresh_answer_from_another_model_is_refused_and_not_recorded() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("fixture.jsonl");
        let mock = Arc::new(Mock::new(vec![Exchange::ok(&success_body("jev-2.0.0"))]));
        let cassette = Cassette::record(&path, mock, "jev-1.13.0").expect("opens");
        let client = with_transport(resolve(&env()).expect("key"), Arc::new(cassette));

        let sdk_error = client
            .system_one(request("skewed"))
            .await
            .expect_err("the answer names another model");
        assert_eq!(
            classify(&sdk_error).code,
            JevErrorCode::CassetteModelMismatch
        );
        assert!(!path.exists(), "nothing was appended");
    }

    /// Provenance: PLAT-977. A line with no `model` in its body fails
    /// closed rather than skipping the model check.
    #[test]
    fn a_line_with_no_model_fails_to_replay() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("fixture.jsonl");
        let body = serde_json::json!({"answers": {}, "usage": {}}).to_string();
        seed(&path, &[line("k1", 200, &body)]);

        assert_eq!(
            replay_error(&path, "jev-1.13.0").code,
            JevErrorCode::CassetteInvalid
        );
    }

    /// Provenance: PLAT-977. A line whose body is not strict JSON fails
    /// closed too -- a duplicate member name is what `parse_strict_json_str`
    /// rejects and a lenient parser would have let through.
    #[test]
    fn a_line_whose_body_is_not_strict_json_fails_to_replay() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("fixture.jsonl");
        seed(
            &path,
            &[line(
                "k1",
                200,
                r#"{"model":"jev-1.13.0","model":"jev-1.13.0"}"#,
            )],
        );

        assert_eq!(
            replay_error(&path, "jev-1.13.0").code,
            JevErrorCode::CassetteInvalid
        );
    }

    /// Provenance: PLAT-977. Two lines sharing a key have no principled
    /// "current" reading; the whole file refuses to load.
    #[test]
    fn a_duplicate_key_fails_to_load() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("fixture.jsonl");
        let body = success_body("jev-1.13.0");
        seed(&path, &[line("k1", 200, &body), line("k1", 200, &body)]);

        let error = replay_error(&path, "jev-1.13.0");
        assert_eq!(error.code, JevErrorCode::CassetteInvalid);
        assert!(error.message.contains("duplicate key"), "{}", error.message);
    }

    /// Provenance: PLAT-977. A line that is not JSON at all fails to load.
    #[test]
    fn a_line_that_is_not_json_fails_to_load() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("fixture.jsonl");
        std::fs::write(&path, "not json\n").expect("seed");

        assert_eq!(
            replay_error(&path, "jev-1.13.0").code,
            JevErrorCode::CassetteInvalid
        );
    }

    /// Provenance: PLAT-977. Valid JSON missing a required field
    /// (`response_body`) fails to load.
    #[test]
    fn a_line_missing_a_required_field_fails_to_load() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("fixture.jsonl");
        seed(&path, &[serde_json::json!({"key": "k1", "status": 200})]);

        assert_eq!(
            replay_error(&path, "jev-1.13.0").code,
            JevErrorCode::CassetteInvalid
        );
    }

    /// Provenance: PLAT-977. A hand-edited non-2xx line never replays as a
    /// success.
    #[test]
    fn a_line_with_a_non_2xx_status_fails_to_load() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("fixture.jsonl");
        seed(&path, &[line("k1", 500, &success_body("jev-1.13.0"))]);

        let error = replay_error(&path, "jev-1.13.0");
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

        let error = Cassette::record(dir.path(), Arc::new(Mock::new(vec![])), "jev-1.13.0")
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
        let cassette =
            Cassette::record(&path, mock, "jev-1.13.0").expect("a missing file is empty");
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
            Cassette::record(dir.path().join("fixture.jsonl"), mock.clone(), "jev-1.13.0")
                .expect("opens");
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
