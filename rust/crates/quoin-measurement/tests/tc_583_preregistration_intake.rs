// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! PLAT-936: a plan's `preregistration` block is tamper evidence over its own
//! "Comparison and Enforcement" section, checked at intake for a *new*
//! collection.
//!
//! # Why this exists, and why it is narrower than it sounds
//!
//! PLAT-935's design research found the obvious check — hash the bar text,
//! look up when it was first committed, compare that to the observation's own
//! timestamp — unbuildable: a commit's author-date is an environment variable
//! the author sets (`GIT_AUTHOR_DATE`), and `collection.timestamp` is read by
//! this crate as a plain unparsed string (`validate::read::non_empty`),
//! equally self-reported. Comparing one author-chosen string to another and
//! calling it "pre-registered" is the false-confidence failure PLAT-935 names.
//!
//! What is built here proves something narrower and real: the bar text has
//! not been silently altered since its digest was recorded. It hooks into the
//! plan-loading path `measurement_collection` already walks — it does not add
//! a second attestation mechanism, and it is not an ordering check. Residual
//! gaps this does not close (an evaluator reporting only a favourable
//! pre-registered variant, a repeated re-run until one passes, editing an
//! unprotected input such as the baseline instead of the bar text) are named
//! in [`quoin_measurement::types::plan::PlanPreregistration`] and are not
//! claimed to be closed by anything below.
//!
//! Trace: FR-044-AC-1
//! Provenance: PLAT-936

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::Path;

use quoin_measurement::error::MeasurementErrorCode;
use quoin_measurement::json_bridge::from_serde;
use quoin_measurement::plans::{PlanLoadOptions, load_measurement_plans};
use quoin_measurement::source::DiskMeasurement;
use quoin_measurement::types::plan::MeasurementPlan;
use quoin_measurement::validate;
use serde_json::{Value, json};

/// The digest of the exact text `BAR_TEXT` is authored with below, computed
/// once with `sha256sum` and carried as a literal — this test does not
/// re-derive its own expectation.
const BAR_DIGEST: &str = "sha256:6ceed63723cae311712ca0caacfde02e8e733a7bcfb17405503fb09fbadd5576";

const BAR_TEXT: &str = "Pass when agreement exceeds the constant-predictor baseline.";

/// A gate-stage plan carrying a `preregistration` block, and the body it
/// declares that digest over.
fn plan_document(bar_digest: &str, bar_text: &str) -> String {
    format!(
        "---\n\
         id: MP-900\n\
         title: Example gate\n\
         type: MeasurementPlan\n\
         status: active\n\
         owner: test\n\
         stage: gate\n\
         metric: quality.gate\n\
         definition_version: quality.gate-v1\n\
         preregistration:\n\
         \x20\x20bar_digest: {bar_digest}\n\
         ---\n\
         \n\
         # Example gate\n\
         \n\
         ## Comparison and Enforcement\n\
         \n\
         {bar_text}\n"
    )
}

/// A repository holding one such plan.
fn planned_repository(bar_digest: &str, bar_text: &str) -> tempfile::TempDir {
    let temporary = tempfile::tempdir().expect("a temporary directory");
    let assurance = temporary.path().join("spec").join("assurance");
    std::fs::create_dir_all(&assurance).expect("the assurance root is creatable");
    std::fs::write(
        assurance.join("MP-900.md"),
        plan_document(bar_digest, bar_text),
    )
    .expect("the plan is writable");
    temporary
}

fn authored_plans(root: &Path) -> Vec<MeasurementPlan> {
    load_measurement_plans(&DiskMeasurement::new(root), PlanLoadOptions::default())
        .expect("the authored plan loads")
}

/// A complete, otherwise-admissible schema-v2 collection against `MP-900`.
fn new_collection_json() -> Value {
    json!({
        "schemaVersion": 2,
        "collectionId": "run-900",
        "subject": "fixture",
        "scope": { "cases": 1 },
        "toolIdentity": "fixture producer",
        "toolVersion": "fixture 1 (engine a1)",
        "configDigest": "sha256:config-a",
        "timestamp": "2026-09-21T00:00:00.000Z",
        "sourceRevision": "aaaaaaaaaaaaaaaa",
        "environment": { "runner": "test" },
        "verificationStack": {
            "schemaVersion": "verification-stack-attestation-v1",
            "lockDigest": format!("sha256:{}", "1".repeat(64)),
            "executableDigest": format!("sha256:{}", "2".repeat(64)),
            "buildProfile": "release",
            "toolchains": { "node": "22.15.0", "rust": "1.94.1", "python": "3.10.12" },
            "sources": {
                "fixture": {
                    "revision": "a".repeat(40),
                    "sourceState": "clean",
                    "remote": "https://example.invalid/fixture",
                },
            },
            "capabilities": ["fixture.capability"],
            "artifacts": { "config": format!("sha256:{}", "3".repeat(64)) },
        },
        "observations": [
            {
                "metric": "quality.gate",
                "planId": "MP-900",
                "definitionVersion": "quality.gate-v1",
                "state": "measured",
                "value": 0.9,
                "unit": "fraction",
                "shape": "ratio",
            },
        ],
        "rawEvidence": { "payload": [1, 2] },
    })
}

/// A digest matching the bar text as it reads today admits the collection —
/// the block being present at all costs nothing when nothing has drifted.
///
/// Trace: FR-044-AC-1
/// Provenance: PLAT-936
#[test]
fn tc_583_a_matching_preregistered_bar_is_admitted() {
    let repository = planned_repository(BAR_DIGEST, BAR_TEXT);
    let plans = authored_plans(repository.path());
    let candidate = new_collection_json();
    let admitted = validate::measurement_collection(
        &from_serde(&candidate).expect("the candidate crosses the bridge"),
        &plans,
    )
    .expect("a matching bar digest admits the collection");
    assert_eq!(admitted.collection_id.as_str(), "run-900");
}

/// The bar text was edited after `BAR_DIGEST` was recorded, without the
/// frontmatter being updated to match — exactly the silent alteration
/// PLAT-936 exists to catch. The collection is refused, and the refusal names
/// both the plan and the fact that the bar text moved.
///
/// Trace: FR-044-AC-1
/// Provenance: PLAT-936
#[test]
fn tc_583_a_bar_edited_after_capture_is_refused() {
    let repository = planned_repository(BAR_DIGEST, "Pass when agreement exceeds 0.99.");
    let plans = authored_plans(repository.path());
    let candidate = new_collection_json();
    let refusal = validate::measurement_collection(
        &from_serde(&candidate).expect("the candidate crosses the bridge"),
        &plans,
    )
    .expect_err("a stale bar digest refuses the collection");
    assert_eq!(refusal.code(), MeasurementErrorCode::CollectionInvalid);
    assert!(
        refusal
            .findings()
            .iter()
            .any(|finding| finding.contains("MP-900") && finding.contains("pre-registered")),
        "the refusal must name the plan and the pre-registration mismatch, got {:?}",
        refusal.findings()
    );
}

/// A plan with no `preregistration` block at all is unaffected — the check is
/// additive, and every plan predating PLAT-936 keeps validating exactly as it
/// did before this landed.
///
/// Trace: FR-044-AC-1
/// Provenance: PLAT-936
#[test]
fn tc_583_a_plan_without_preregistration_is_unaffected() {
    let temporary = tempfile::tempdir().expect("a temporary directory");
    let assurance = temporary.path().join("spec").join("assurance");
    std::fs::create_dir_all(&assurance).expect("the assurance root is creatable");
    std::fs::write(
        assurance.join("MP-900.md"),
        "---\n\
         id: MP-900\n\
         title: Example gate\n\
         type: MeasurementPlan\n\
         status: active\n\
         owner: test\n\
         stage: gate\n\
         metric: quality.gate\n\
         definition_version: quality.gate-v1\n\
         ---\n\
         \n\
         # Example gate\n",
    )
    .expect("the plan is writable");
    let plans = authored_plans(temporary.path());
    let candidate = new_collection_json();
    let admitted = validate::measurement_collection(
        &from_serde(&candidate).expect("the candidate crosses the bridge"),
        &plans,
    )
    .expect("no preregistration block means nothing new to refuse on");
    assert_eq!(admitted.collection_id.as_str(), "run-900");
}
