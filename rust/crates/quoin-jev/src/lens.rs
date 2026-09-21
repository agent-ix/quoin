// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The execution path: one FR in, one Jev call, one [`FrVerdict`] out
//! (PLAT-837).
//!
//! "Questions batch within one request, so one FR's whole question block
//! should be a single call" -- this module is that call. It builds the
//! request from [`FrContext`] and [`QuestionSet`], sends it through a
//! [`typesafe_sdk_client::Client`] (production or mocked; this module does
//! not care which), and hands the response to [`crate::verdict::extract`].

use typesafe_sdk_client::{Client, SystemOneRequest};
use typesafe_sdk_questions::Entry;

use crate::context::FrContext;
use crate::error::{JevError, classify};
use crate::question_set::QuestionSet;
use crate::verdict::{FrVerdict, extract};

/// Builds the request for one FR, without sending it.
///
/// Exposed separately from [`run`] so a caller (or a test) can inspect the
/// exact request shape -- the questions map, the state -- before or instead
/// of sending it.
#[must_use]
pub fn build_request(context: &FrContext, question_set: &QuestionSet) -> SystemOneRequest {
    let state: Entry = context.into();
    let questions = question_set.questions_for_fr(context.ac_ids().iter().map(String::as_str));
    SystemOneRequest::new(state, questions)
}

/// Runs the lens for one FR: builds the request, sends it, and extracts
/// findings.
///
/// `confidence_threshold` is threaded straight to
/// [`crate::verdict::extract`] -- see that function's doc for why this crate
/// does not bake in a value of its own.
///
/// # Errors
/// Any [`typesafe_sdk_error::Error`] the client raises (auth, validation,
/// rate limit, connection, timeout), classified through
/// [`crate::error::classify`].
pub async fn run(
    client: &Client,
    context: &FrContext,
    question_set: &QuestionSet,
    confidence_threshold: f64,
) -> Result<FrVerdict, JevError> {
    let request = build_request(context, question_set);
    let response = client
        .system_one(request)
        .await
        .map_err(|error| classify(&error))?;
    Ok(extract(
        &response,
        question_set,
        &context.ac_ids(),
        confidence_threshold,
    ))
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

    use typesafe_sdk_env::Fixed;
    use typesafe_sdk_http::{Exchange, Mock};

    use super::{build_request, run};
    use crate::client::with_transport;
    use crate::config::resolve;
    use crate::context::{AcRow, FrContext};
    use crate::error::JevErrorCode;
    use crate::question_set::QuestionSet;

    const ASSET: &str = include_str!(
        "../../../../skills/spec-criterion-strength-analysis/assets/question-set.json"
    );

    fn context() -> FrContext {
        FrContext {
            fr_id: "FR-900".to_owned(),
            statement: "The system SHALL emit a report.".to_owned(),
            description: Some("Emits a report after each run.".to_owned()),
            behaviour: None,
            constraints: None,
            acceptance_criteria: vec![AcRow {
                id: "FR-900-AC-1".to_owned(),
                text: "A report file exists after the run completes.".to_owned(),
            }],
        }
    }

    /// Provenance: PLAT-837. One FR, one call: the built request carries a
    /// question for every `noul` id plus `weakness_kind` for the one AC row,
    /// and exactly one `adverse_case_coverage` question for the whole FR.
    #[test]
    fn one_fr_produces_one_batched_request() {
        let set = QuestionSet::parse(ASSET).expect("parses");
        let request = build_request(&context(), &set);
        assert_eq!(request.questions.len(), 7); // 5 noul + 1 choice + 1 score
    }

    /// Provenance: PLAT-837. End to end with NO network and NO key beyond a
    /// fixed test value: the mock transport answers, the classifier recorded
    /// is the concrete model version the mocked response carried, and the
    /// scripted `noul` values are carried all the way through to the
    /// produced finding (PLAT-837 review finding 5: a mutation to
    /// `QuestionSet::noul_key` that drops the `noul` wiring must fail this
    /// test, not just a lower-level unit test).
    #[tokio::test]
    async fn the_lens_runs_end_to_end_against_the_mock_transport() {
        let env = Fixed::new(&[("TYPESAFE_API_KEY", "sk_test_key")]);
        let config = resolve(&env).expect("the key is present");
        let set = QuestionSet::parse(ASSET).expect("parses");
        let body = serde_json::json!({
            "model": "jev-1.13.0",
            "answers": {
                "FR-900-AC-1::falsifiable": {"type": "noul", "noul": 0.9},
                "FR-900-AC-1::states_observable_outcome": {"type": "noul", "noul": 0.9},
                "FR-900-AC-1::threshold_present": {"type": "noul", "noul": 0.1},
                "FR-900-AC-1::restates_requirement": {"type": "noul", "noul": 0.05},
                "FR-900-AC-1::implementation_coupled": {"type": "noul", "noul": 0.02},
                "FR-900-AC-1::weakness_kind": {
                    "type": "choice", "choice": "happy_path_only", "confidence": 0.92,
                    "probabilities": {"happy_path_only": 0.92, "sound": 0.05}
                },
                "adverse_case_coverage": {
                    "type": "score", "score": 1.0, "confidence": 0.8,
                    "legend": {}, "probabilities": {}
                }
            },
            "usage": {"input_tokens": 120, "output_tokens": 0}
        });
        let mock = Arc::new(Mock::new(vec![Exchange::ok(&body.to_string())]));
        let client = with_transport(config, mock.clone());

        let verdict = run(&client, &context(), &set, 0.5)
            .await
            .expect("mocked 200");
        assert_eq!(verdict.classifier, "jev-1.13.0");
        assert!(
            verdict.sound.is_empty(),
            "happy_path_only is a finding, not sound"
        );
        assert_eq!(
            verdict.findings.len(),
            1,
            "happy_path_only maps to exactly one Low finding"
        );
        let noul = &verdict.findings[0].noul.values;
        assert!(
            noul.iter()
                .any(|(id, value)| id == "falsifiable" && (*value - 0.9).abs() < f64::EPSILON),
            "the scripted noul value is carried through to the finding: {noul:?}"
        );
        assert_eq!(mock.attempts(), 1);
    }

    /// Provenance: PLAT-837. A 401 (missing/invalid key) surfaces as the
    /// named `JEV_UNAUTHORIZED` code, not a generic failure -- and this needs
    /// no real key or network to prove, only the mock transport.
    #[tokio::test]
    async fn an_unauthorized_response_classifies_by_its_named_code() {
        let env = Fixed::new(&[("TYPESAFE_API_KEY", "sk_test_key")]);
        let config = resolve(&env).expect("the key is present");
        let set = QuestionSet::parse(ASSET).expect("parses");
        let mock = Arc::new(Mock::new(vec![Exchange::status(
            401,
            r#"{"error": "invalid API key"}"#,
        )]));
        let client = with_transport(config, mock);

        let error = run(&client, &context(), &set, 0.5)
            .await
            .expect_err("401 must not read as success");
        assert_eq!(error.code, JevErrorCode::Unauthorized);
    }

    /// Provenance: PLAT-837. Same for 422 validation and 429 rate-limit,
    /// against the SDK's real classifier rather than an assumption about it.
    #[tokio::test]
    async fn a_422_and_a_429_classify_distinctly() {
        let env = Fixed::new(&[("TYPESAFE_API_KEY", "sk_test_key")]);
        let set = QuestionSet::parse(ASSET).expect("parses");

        let config = resolve(&env).expect("present");
        let mock = Arc::new(Mock::new(vec![Exchange::status(422, r#"{"detail": []}"#)]));
        let client = with_transport(config, mock);
        let error = run(&client, &context(), &set, 0.5).await.expect_err("422");
        assert_eq!(error.code, JevErrorCode::Validation);

        let config = resolve(&env).expect("present");
        // The SDK's default RetryPolicy retries every 429 (`500..600`, `408`
        // and `429` are retried by default), so a 429 that never changes
        // costs the default `max_retries` + 1 attempts before it surfaces --
        // scripted here explicitly rather than assuming one call ends it.
        let mock = Arc::new(Mock::new(vec![
            Exchange::status(429, ""),
            Exchange::status(429, ""),
            Exchange::status(429, ""),
        ]));
        let client = with_transport(config, mock.clone());
        let error = run(&client, &context(), &set, 0.5).await.expect_err("429");
        assert_eq!(error.code, JevErrorCode::RateLimited);
        assert_eq!(mock.attempts(), 3, "the default policy retries a 429 twice");
    }
}
