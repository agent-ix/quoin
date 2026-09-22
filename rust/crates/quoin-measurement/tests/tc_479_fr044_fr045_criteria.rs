// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Every FR-044 and FR-045 acceptance criterion the deleted
//! `tests/measurement.test.ts` and `tests/portfolio-report.test.ts` carried,
//! restated against this crate, plus the FR-067-AC-8 assertion that
//! `tests/measurement-store-results.test.ts` holds.
//!
//! Trace: FR-044-AC-1, FR-044-AC-2, FR-044-AC-3, FR-044-AC-4
//! Trace: FR-045-AC-1, FR-045-AC-2, FR-045-AC-3, FR-045-AC-4
//! Trace: FR-067-AC-8
//! Provenance: quoin#479
//!
//! # Why this file exists
//!
//! `src/measurement/` is deleted at cutover (FR-101) and its tests go with it.
//! `tc_473_reporting.rs` already proves this crate writes the retained
//! TypeScript's bytes over the committed fixture tree, but it is tagged
//! `FR-100-AC-4` — a byte-for-byte agreement gate is not a per-criterion one,
//! and a criterion whose only tagged home is a file about to be deleted is a
//! criterion that leaves no test behind. This file is that home: one test per
//! criterion, named by the criterion, so a criterion cannot be lost without a
//! named test disappearing with it.
//!
//! # Where the inputs come from
//!
//! `tests/fixtures/portfolio-tree/` — the committed tree `tc_473_reporting.rs`
//! captures the TypeScript's answers over, not a shape this file invented. Its
//! seven repositories already state every population FR-045 names: a current
//! store (`alpha`), an empty one (`bravo`), an absent one (`charlie`), a stale
//! one (`delta`), an unreadable collection (`golf`), a location that is not a
//! directory (`echo-not-a-directory`) and a location that does not exist
//! (`zulu-does-not-exist`). Where a criterion needs an input the tree does not
//! hold — an intake refusal, a corpus-oriented root `assurance` layout, a
//! definition that moved between two collections — the base case is taken from
//! the tree and mutated **here**, in Rust, exactly as
//! `tc_480_fr066_criteria.rs` does. Constructing an input to exercise a
//! refusal path is not a fixture tautology: nothing below compares this
//! crate's output against bytes this file asked this crate to produce.
//!
//! No node process is spawned; `tc_468_boundary.rs` asserts none can be.
//!
//! # Two TypeScript assertions carried no criterion tag
//!
//! `tests/measurement.test.ts:131` and `:154` — the toolchain-identity refusal
//! and the historical-envelope split — are load-bearing and were untagged.
//! Neither `verificationStack` nor the read-only historical envelope is named
//! in any FR-044 criterion's text: AC-1's member census predates schema v2, and
//! the historical half is stated only in FR-045's **Constraints**, which are
//! not criteria. Both are restated below under `FR-044-AC-1`, the criterion
//! that owns *what a collection must carry for the write to be admitted*,
//! because that is the decision each one makes. Each test's doc comment says
//! so; the criterion text is a gap worth closing in the spec, not here.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

use quoin_measurement::error::MeasurementErrorCode;
use quoin_measurement::json_bridge::from_serde;
use quoin_measurement::plans::{PlanLoadOptions, load_measurement_plans};
use quoin_measurement::portfolio::{
    PORTFOLIO_STALE_AFTER_DAYS, PortfolioCollectionSnapshot, PortfolioRepositoryReport,
    RepositoryStatus, StoreState, build_portfolio_report, build_portfolio_report_from_collections,
    render_portfolio_report, render_portfolio_report_json,
};
use quoin_measurement::report::comparison::{
    MeasurementCollectionReference, MeasurementComparisonReport, MeasurementComparisonStatus,
};
use quoin_measurement::report::{
    build_measurement_report, comparison_for, render_measurement_comparison,
    render_measurement_report,
};
use quoin_measurement::source::DiskMeasurement;
use quoin_measurement::store::{
    measurements_root, read_measurement_collection_results, write_measurement_collection,
};
use quoin_measurement::types::collection::MeasurementCollection;
use quoin_measurement::types::comparison::{
    ComparisonReasonCode, ComparisonStatus, MeasurementComparison,
};
use quoin_measurement::types::plan::MeasurementPlan;
use quoin_measurement::{compare_measurement_collections, validate};
use serde_json::{Value, json};

// ------------------------------------------------------- the committed tree

/// The fixture tree's absolute root in this checkout.
///
/// Unresolved, for `tc_473_reporting.rs`'s reason: the capture's paths came
/// from node's lexical `path.resolve`, and `canonicalize` would follow
/// symlinks it cannot.
fn tree() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("portfolio-tree")
}

/// One repository of the committed tree.
fn repo_in_tree(name: &str) -> PathBuf {
    tree().join(name)
}

/// The seven locations the capture was taken over, in its own order.
fn every_location() -> Vec<PathBuf> {
    [
        "golf",
        "delta",
        "alpha",
        "bravo",
        "charlie",
        "echo-not-a-directory",
        "zulu-does-not-exist",
    ]
    .iter()
    .map(|name| repo_in_tree(name))
    .collect()
}

/// One repository's entry in a portfolio report, by name.
fn repository<'a>(
    report: &'a quoin_measurement::PortfolioReport,
    name: &str,
) -> &'a PortfolioRepositoryReport {
    report
        .repositories
        .iter()
        .find(|entry| entry.name == name)
        .unwrap_or_else(|| panic!("the portfolio omitted `{name}`"))
}

/// One committed collection, read back as the JSON it is stored as.
fn stored_json(repository_name: &str, collection: &str) -> Value {
    let path = repo_in_tree(repository_name)
        .join("spec")
        .join("evidence")
        .join("measurements")
        .join(format!("{collection}.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()));
    serde_json::from_str(&text).expect("a committed collection is JSON")
}

/// Admit a JSON document as a historical stored collection.
fn admit(value: &Value) -> MeasurementCollection {
    let stored = from_serde(value).expect("the document crosses the JSON bridge");
    validate::stored_measurement_collection(&stored)
        .unwrap_or_else(|error| panic!("the document is an admissible collection: {error}"))
}

// ------------------------------------------------- a new collection, minted

/// The `MeasurementPlan` `tests/measurement.test.ts:24-51` authored.
const PLAN: &str = "---\n\
id: MP-001\n\
title: Example metric\n\
type: MeasurementPlan\n\
status: active\n\
owner: test\n\
stage: branch-comparison\n\
metric: quality.example\n\
definition_version: quality.example-v1\n\
---\n\
\n\
# Example metric\n";

/// A repository holding that plan and nothing else.
fn planned_repository() -> tempfile::TempDir {
    let temporary = tempfile::tempdir().expect("a temporary directory");
    let assurance = temporary.path().join("spec").join("assurance");
    std::fs::create_dir_all(&assurance).expect("the assurance root is creatable");
    std::fs::write(assurance.join("MP-001.md"), PLAN).expect("the plan is writable");
    temporary
}

/// The plans that repository authors.
fn authored_plans(root: &Path) -> Vec<MeasurementPlan> {
    load_measurement_plans(&DiskMeasurement::new(root), PlanLoadOptions::default())
        .expect("the authored plan loads")
}

/// The complete schema-v2 collection `tests/measurement.test.ts:71-122` built,
/// restated member for member so a member the intake check stops requiring is
/// visible as a test that stops failing.
fn new_collection_json() -> Value {
    json!({
        "schemaVersion": 2,
        "collectionId": "run-001",
        "subject": "fixture",
        "scope": { "cases": 1 },
        "toolIdentity": "fixture producer",
        "toolVersion": "fixture 1 (engine a1)",
        "configDigest": format!("sha256:{}", "a".repeat(64)),
        "timestamp": "2026-08-26T00:00:00.000Z",
        "sourceRevision": "aaaaaaaaaaaaaaaa",
        "corpusRevision": "cccccccccccccccc",
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
                "metric": "quality.example",
                "planId": "MP-001",
                "definitionVersion": "quality.example-v1",
                "state": "measured",
                "value": 0.5,
                "unit": "fraction",
                "shape": "ratio",
                "population": {
                    "examined": 2,
                    "matched": 1,
                    "complete": true,
                    "identity": ["a", "b"],
                },
            },
        ],
        "rawEvidence": { "payload": [1, 2] },
    })
}

/// Delete `value` at a path of member names, panicking if it is not there.
///
/// A numeric step indexes an array, so an observation's own members can be
/// named without a second helper.
fn delete_at(value: &mut Value, path: &[&str]) {
    let Some((last, parents)) = path.split_last() else {
        panic!("an empty path deletes nothing");
    };
    let mut cursor = value;
    for step in parents {
        cursor = match step.parse::<usize>() {
            Ok(index) => cursor.get_mut(index).unwrap_or_else(|| {
                panic!("the base case has no `[{index}]` on the way to `{last}`")
            }),
            Err(_) => cursor
                .get_mut(*step)
                .unwrap_or_else(|| panic!("the base case has no `{step}` on the way to `{last}`")),
        };
    }
    let object = cursor
        .as_object_mut()
        .unwrap_or_else(|| panic!("`{last}`'s parent is not an object"));
    assert!(
        object.remove(*last).is_some(),
        "the base case carries no `{last}` to delete"
    );
}

/// The "including changed definitions" half of FR-045-AC-3, called from
/// `tc_479_015`.
///
/// The committed pair does not move a definition, so `delta`'s collection is
/// re-dated and re-versioned here to make one that does — base case from the
/// capture, mutation in Rust.
fn a_definition_move_reaches_the_portfolio() {
    let before = admit(&stored_json("delta", "d-2025-11"));
    let mut drifted = stored_json("delta", "d-2025-11");
    drifted["collectionId"] = json!("d-2026-01");
    drifted["timestamp"] = json!("2026-01-01T00:00:00.000Z");
    drifted["observations"][0]["definitionVersion"] = json!("2");
    let after = admit(&drifted);
    let report = build_portfolio_report_from_collections(&[PortfolioCollectionSnapshot {
        root: repo_in_tree("delta"),
        collections: vec![before, after],
    }]);
    let comparison = repository(&report, "delta")
        .comparison
        .as_ref()
        .expect("the drifted pair is still compared");
    let row = comparison
        .observations
        .iter()
        .find(|row| row.metric == "finding_recall")
        .expect("the drifted metric is still a row");
    assert_eq!(row.status, ComparisonStatus::Incomparable);
    assert_eq!(row.delta, None, "a changed definition withdraws the delta");
    assert!(
        row.reasons
            .iter()
            .any(|reason| reason.code == ComparisonReasonCode::DefinitionChanged),
        "the portfolio must state the definition move, got {:?}",
        row.reasons
            .iter()
            .map(|reason| reason.code)
            .collect::<Vec<_>>()
    );
}

// ------------------------------------------------------------- the gates

/// An observation whose metric no authored `MeasurementPlan` governs is
/// refused by name rather than stored as an untyped number.
///
/// FR-044-AC-1: "Every observation resolves to an active plan or the write is
/// refused naming the metric."
///
/// Trace: FR-044-AC-1
/// Provenance: quoin#479
#[test]
fn tc_479_001_an_unplanned_observation_is_refused_naming_the_metric() {
    let candidate = from_serde(&new_collection_json()).expect("the candidate crosses the bridge");

    let refusal = validate::measurement_collection(&candidate, &[])
        .expect_err("a metric with no authored plan is refused");
    assert_eq!(refusal.code(), MeasurementErrorCode::CollectionInvalid);
    let message = refusal.to_string();
    assert!(
        message.contains("quality.example"),
        "the refusal must name the metric it refused; it said {message:?}"
    );
    assert!(
        message.contains("has no MeasurementPlan") && message.contains("record refused"),
        "the refusal must say the record is refused for want of a plan; it said {message:?}"
    );

    // The same candidate, under the plan the repository actually authors, is
    // admitted — so the refusal above is the plan's absence and not a second
    // defect in the document.
    let temporary = planned_repository();
    let plans = authored_plans(temporary.path());
    assert_eq!(plans.len(), 1, "the fixture repository authors one plan");
    let admitted = validate::measurement_collection(&candidate, &plans)
        .expect("the governed candidate is admissible");
    assert_eq!(admitted.collection_id.as_str(), "run-001");
    assert_eq!(admitted.observations.len(), 1);

    // A plan that is not active, and a plan at a different definition version,
    // are each their own refusal rather than a silent admission.
    let mut retired = plans.clone();
    retired[0].status = quoin_measurement::types::plan::LifecycleStatus::Retired;
    assert!(
        validate::measurement_collection(&candidate, &retired)
            .expect_err("a retired plan governs nothing")
            .to_string()
            .contains("not active"),
        "a retired plan must be refused as retired"
    );
    let mut moved = plans;
    moved[0].definition_version = quoin_measurement::types::ids::NonEmptyText::parse(
        "quality.example-v2",
        MeasurementErrorCode::PlanInvalid,
        "definition_version",
    )
    .expect("a non-empty definition version");
    assert!(
        validate::measurement_collection(&candidate, &moved)
            .expect_err("a definition that moved is refused")
            .to_string()
            .contains("does not match"),
        "an observation at a definition the plan no longer states must be refused"
    );
}

/// Every member FR-044-AC-1 names is required, one at a time.
///
/// FR-044-AC-1: "A collection carries schema and collection identity, subject,
/// scope, tool identity/version, configuration digest, timestamp, source
/// revision, environment, observations with definition version and units, and
/// attached raw evidence."
///
/// Trace: FR-044-AC-1
/// Provenance: quoin#479
#[test]
fn tc_479_002_every_member_the_criterion_names_is_required() {
    /// The twelve members the criterion enumerates, each named the way the
    /// wire spells it. A numeric step indexes the observation list.
    const REQUIRED: [&[&str]; 12] = [
        &["schemaVersion"],
        &["collectionId"],
        &["subject"],
        &["scope"],
        &["toolIdentity"],
        &["toolVersion"],
        &["configDigest"],
        &["timestamp"],
        &["sourceRevision"],
        &["environment"],
        &["observations"],
        &["rawEvidence"],
    ];

    /// The two members the criterion names *inside* an observation.
    const REQUIRED_IN_OBSERVATION: [&[&str]; 2] = [
        &["observations", "0", "definitionVersion"],
        &["observations", "0", "unit"],
    ];

    let temporary = planned_repository();
    let plans = authored_plans(temporary.path());

    // The positive control. Without it a mutation loop proves only that a
    // broken base case is refused fourteen times.
    let base = new_collection_json();
    validate::measurement_collection(
        &from_serde(&base).expect("the base case crosses the bridge"),
        &plans,
    )
    .expect("the unmutated base case is admissible");

    let mut measured = 0_usize;
    for path in REQUIRED.iter().chain(REQUIRED_IN_OBSERVATION.iter()) {
        let mut mutated = base.clone();
        delete_at(&mut mutated, path);
        let refusal = validate::measurement_collection(
            &from_serde(&mutated).expect("the mutated case crosses the bridge"),
            &plans,
        )
        .err()
        .unwrap_or_else(|| panic!("a collection with no `{}` was admitted", path.join(".")));
        assert_eq!(
            refusal.code(),
            MeasurementErrorCode::CollectionInvalid,
            "`{}` must be refused as an invalid collection",
            path.join(".")
        );
        measured += 1;
    }
    assert_eq!(
        measured,
        REQUIRED.len() + REQUIRED_IN_OBSERVATION.len(),
        "every member the criterion names must be measured, not a subset"
    );
    assert!(
        measured >= 14,
        "the census measured {measured} members, below the criterion's own count"
    );
}

/// A schema-v2 attestation that carries no `toolchains` member at all is
/// refused; omitting one language inside it is accepted as "not applicable"
/// (PLAT-930).
///
/// `tests/measurement.test.ts:131`, which carried no criterion tag. **No
/// FR-044 criterion's text names `verificationStack`**: AC-1's member census
/// was written for schema v1 and never grew the attestation. It is tagged
/// AC-1 here because AC-1 is the criterion that decides what a collection must
/// carry for the write to be admitted, which is the decision this refusal
/// makes. The criterion text is the gap, and closing it is a spec change.
///
/// Trace: FR-044-AC-1
/// Provenance: quoin#479, PLAT-930
#[test]
fn tc_479_003_a_new_collection_requires_toolchains_and_a_release_build_but_not_every_language() {
    let temporary = planned_repository();
    let plans = authored_plans(temporary.path());

    let mut drifted = new_collection_json();
    delete_at(&mut drifted, &["verificationStack", "toolchains"]);
    let refusal = validate::measurement_collection(
        &from_serde(&drifted).expect("the drifted case crosses the bridge"),
        &plans,
    )
    .expect_err("an attestation with no `toolchains` member at all is refused");
    assert_eq!(refusal.code(), MeasurementErrorCode::CollectionInvalid);
    assert!(
        refusal
            .to_string()
            .contains("verificationStack.toolchains must be present"),
        "the refusal must say the `toolchains` member itself is missing; it said {refusal}"
    );

    // A measurement rarely touches every language: omitting one is "not
    // applicable" for it, not a refusal. Each is tried on its own, so this is
    // three acceptances rather than one presence check on the object.
    for toolchain in ["node", "rust", "python"] {
        let mut partial = new_collection_json();
        delete_at(
            &mut partial,
            &["verificationStack", "toolchains", toolchain],
        );
        let admitted = validate::measurement_collection(
            &from_serde(&partial).expect("the partial case crosses the bridge"),
            &plans,
        )
        .unwrap_or_else(|error| {
            panic!("an attestation with no `{toolchain}` identity must be admitted: {error}")
        });
        let toolchains = admitted
            .verification_stack
            .as_ref()
            .expect("the attestation is still parsed")
            .toolchains
            .as_ref()
            .expect("the toolchains member itself is still present");
        let observed = match toolchain {
            "node" => &toolchains.node,
            "rust" => &toolchains.rust,
            "python" => &toolchains.python,
            _ => unreachable!("the loop only names the three languages above"),
        };
        assert!(
            observed.is_none(),
            "`{toolchain}` was omitted, so it must read as not applicable, not invented"
        );
    }

    // A build profile other than release is a new collection's own refusal,
    // and so is its absence — `tests/measurement.test.ts:141-152`.
    for profile in [Some("debug"), None] {
        let mut drifted = new_collection_json();
        match profile {
            Some(value) => {
                drifted["verificationStack"]["buildProfile"] = Value::String(value.to_owned());
            }
            None => delete_at(&mut drifted, &["verificationStack", "buildProfile"]),
        }
        let refusal = validate::measurement_collection(
            &from_serde(&drifted).expect("the drifted case crosses the bridge"),
            &plans,
        )
        .err()
        .unwrap_or_else(|| panic!("a {profile:?} build profile was admitted for a new collection"));
        assert!(
            refusal
                .to_string()
                .contains("buildProfile must be release for new collections"),
            "{profile:?}: the refusal must name the profile it wanted; it said {refusal}"
        );
    }
}

/// Omitting a toolchain identity is "not applicable" (`tc_479_003`), but a
/// genuinely malformed one — present and the wrong shape — is still refused.
///
/// PLAT-930: `Option<NonEmptyText>` gives absence exactly one spelling. An
/// empty string is not that spelling; it is a value someone supplied and got
/// wrong, and it must be named rather than silently treated as "not
/// applicable" too.
///
/// Trace: FR-044-AC-1
/// Provenance: PLAT-930
#[test]
fn tc_479_018_a_malformed_toolchain_identity_is_still_refused() {
    let temporary = planned_repository();
    let plans = authored_plans(temporary.path());

    for (toolchain, malformed) in [
        ("node", Value::String(String::new())),
        ("rust", Value::Number(1.into())),
        ("python", Value::Array(Vec::new())),
    ] {
        let mut candidate = new_collection_json();
        candidate["verificationStack"]["toolchains"][toolchain] = malformed.clone();
        let refusal = validate::measurement_collection(
            &from_serde(&candidate).expect("the malformed case crosses the bridge"),
            &plans,
        )
        .err()
        .unwrap_or_else(|| {
            panic!("a malformed `{toolchain}` identity ({malformed:?}) was admitted")
        });
        assert_eq!(refusal.code(), MeasurementErrorCode::CollectionInvalid);
        assert!(
            refusal
                .to_string()
                .contains(&format!("verificationStack.toolchains.{toolchain}")),
            "the refusal must name the malformed identity; it said {refusal}"
        );
    }

    // `null` is explicit absence, not malformed — it is accepted exactly as
    // an omitted key is, and reads back as not applicable.
    let mut explicit_null = new_collection_json();
    explicit_null["verificationStack"]["toolchains"]["node"] = Value::Null;
    let admitted = validate::measurement_collection(
        &from_serde(&explicit_null).expect("the null case crosses the bridge"),
        &plans,
    )
    .expect("an explicit `null` toolchain identity is accepted as not applicable");
    assert!(
        admitted
            .verification_stack
            .as_ref()
            .expect("the attestation is parsed")
            .toolchains
            .as_ref()
            .expect("the toolchains member is present")
            .node
            .is_none(),
        "an explicit `null` must read as not applicable, not as an empty identity"
    );
}

/// Two independent defects inside `verificationStack` are both named by one
/// call, not discovered one round trip at a time.
///
/// PLAT-929: an evaluator who fixed a malformed `lockDigest`, resubmitted, and
/// was then told `executableDigest` was ALSO wrong is the exact failure mode
/// this closes — every member of the attestation is checked regardless of an
/// earlier member's outcome, and every failure lands in one refusal.
///
/// Trace: FR-044-AC-1
/// Provenance: PLAT-929
#[test]
fn tc_479_019_two_verification_stack_defects_are_both_named_in_one_refusal() {
    let temporary = planned_repository();
    let plans = authored_plans(temporary.path());

    let mut candidate = new_collection_json();
    candidate["verificationStack"]["lockDigest"] = json!("not-a-digest");
    candidate["verificationStack"]["executableDigest"] = json!("also-not-a-digest");
    let refusal = validate::measurement_collection(
        &from_serde(&candidate).expect("the candidate crosses the bridge"),
        &plans,
    )
    .expect_err("two malformed digests are refused");
    assert_eq!(refusal.code(), MeasurementErrorCode::CollectionInvalid);
    assert!(
        refusal
            .findings()
            .iter()
            .any(|finding| finding.contains("lockDigest")),
        "the one refusal must name `lockDigest`, got {:?}",
        refusal.findings()
    );
    assert!(
        refusal
            .findings()
            .iter()
            .any(|finding| finding.contains("executableDigest")),
        "the SAME refusal must also name `executableDigest`, not just the first defect found; \
         got {:?}",
        refusal.findings()
    );
}

/// Two independent defects in the intake's own checks — beyond
/// `verificationStack` — are also both named by one call.
///
/// PLAT-929: a non-release build and an unplanned metric are unrelated facts
/// about one candidate; fixing the build profile alone must not require a
/// second submission to learn about the unplanned metric.
///
/// Trace: FR-044-AC-1
/// Provenance: PLAT-929
#[test]
fn tc_479_020_a_bad_build_profile_and_an_unplanned_metric_are_both_named_in_one_refusal() {
    let temporary = planned_repository();
    let plans = authored_plans(temporary.path());

    let mut candidate = new_collection_json();
    candidate["verificationStack"]["buildProfile"] = json!("debug");
    candidate["observations"][0]["metric"] = json!("quality.unplanned");
    let refusal = validate::measurement_collection(
        &from_serde(&candidate).expect("the candidate crosses the bridge"),
        &plans,
    )
    .expect_err("a debug build and an unplanned metric are each refused");
    assert_eq!(refusal.code(), MeasurementErrorCode::CollectionInvalid);
    assert!(
        refusal
            .findings()
            .iter()
            .any(|finding| finding.contains("buildProfile must be release")),
        "the one refusal must name the build profile defect, got {:?}",
        refusal.findings()
    );
    assert!(
        refusal
            .findings()
            .iter()
            .any(|finding| finding.contains("quality.unplanned")
                && finding.contains("has no MeasurementPlan")),
        "the SAME refusal must also name the unplanned metric, not just the build profile; \
         got {:?}",
        refusal.findings()
    );
}

/// An empty `toolchains` object satisfies "the member is present" but names
/// no language, which would let an attestation claim zero pinned toolchains —
/// exactly the gap PLAT-930's optional fields opened. It is refused, the same
/// as an omitted `toolchains` member.
///
/// Trace: FR-044-AC-1
/// Provenance: PLAT-930
#[test]
fn tc_479_021_an_empty_toolchains_object_names_no_language_and_is_refused() {
    let temporary = planned_repository();
    let plans = authored_plans(temporary.path());

    let mut candidate = new_collection_json();
    candidate["verificationStack"]["toolchains"] = json!({});
    let refusal = validate::measurement_collection(
        &from_serde(&candidate).expect("the candidate crosses the bridge"),
        &plans,
    )
    .expect_err("a toolchains object naming no language is refused, not treated as present");
    assert_eq!(refusal.code(), MeasurementErrorCode::CollectionInvalid);
    assert!(
        refusal
            .to_string()
            .contains("verificationStack.toolchains must name at least one language"),
        "the refusal must say no language was named; it said {refusal}"
    );
}

/// A `verificationStack` defect that fails the stack's own parse (as opposed
/// to one merely holding a wrong-but-well-formed value, `tc_479_020`'s case)
/// must not swallow every collection-level check below it.
///
/// PLAT-929: an evaluator who fixed a malformed `lockDigest` (which refuses
/// inside `stack::verification_stack` itself, not merely inside
/// `measurement_collection`'s own checks) and resubmitted, only to be told an
/// unplanned metric was ALSO wrong, is the exact two-round-trip failure this
/// closes — the outer collection-vs-stack boundary accumulates too, not just
/// each side of it on its own (review finding #5(a) on quoin#580).
///
/// Trace: FR-044-AC-1
/// Provenance: PLAT-929
#[test]
fn tc_479_022_a_stack_parse_defect_and_an_unplanned_metric_are_both_named_in_one_refusal() {
    let temporary = planned_repository();
    let plans = authored_plans(temporary.path());

    let mut candidate = new_collection_json();
    candidate["verificationStack"]["lockDigest"] = json!("not-a-digest");
    candidate["observations"][0]["metric"] = json!("quality.unplanned");
    let refusal = validate::measurement_collection(
        &from_serde(&candidate).expect("the candidate crosses the bridge"),
        &plans,
    )
    .expect_err("a malformed lockDigest and an unplanned metric are each refused");
    assert_eq!(refusal.code(), MeasurementErrorCode::CollectionInvalid);
    assert!(
        refusal
            .findings()
            .iter()
            .any(|finding| finding.contains("lockDigest")),
        "the one refusal must name the stack's own defect, got {:?}",
        refusal.findings()
    );
    assert!(
        refusal
            .findings()
            .iter()
            .any(|finding| finding.contains("quality.unplanned")
                && finding.contains("has no MeasurementPlan")),
        "the SAME refusal must also name the unplanned metric, not just the stack defect; \
         got {:?}",
        refusal.findings()
    );
}

/// Historical evidence stays readable after the attestation grew members it
/// does not carry, while intake still requires them.
///
/// `tests/measurement.test.ts:154`, which carried no criterion tag. The
/// property is stated in FR-045's **Constraints** — "Historical collections are
/// structurally validated but remain readable" — and Constraints are not
/// criteria, so it has no criterion of its own. It is tagged FR-044-AC-1 for
/// the same reason as `tc_479_003`: the split it asserts is exactly the split
/// between the stored envelope and what AC-1 requires for a write.
///
/// Trace: FR-044-AC-1
/// Provenance: quoin#479
#[test]
fn tc_479_004_historical_evidence_reads_before_the_profile_and_toolchain_fields() {
    let temporary = planned_repository();
    let plans = authored_plans(temporary.path());

    let mut historical = new_collection_json();
    delete_at(&mut historical, &["verificationStack", "buildProfile"]);
    delete_at(&mut historical, &["verificationStack", "toolchains"]);
    let stored = from_serde(&historical).expect("the historical case crosses the bridge");

    let read_back = validate::stored_measurement_collection(&stored)
        .expect("retained evidence stays readable after the attestation grew");
    assert_eq!(read_back.collection_id.as_str(), "run-001");
    let stack = read_back
        .verification_stack
        .as_ref()
        .expect("the attestation it does carry is still parsed");
    assert!(
        stack.build_profile.is_none() && stack.toolchains.is_none(),
        "the two absent members must read as absent, not as invented defaults"
    );

    assert!(
        validate::measurement_collection(&stored, &plans)
            .expect_err("the same document is not admissible as a NEW collection")
            .to_string()
            .contains("buildProfile must be release for new collections"),
        "intake must still require what the historical read forgives"
    );

    // The 48 retained schema-v1 collections are the population this split
    // exists for: they carry no attestation at all and must still read.
    let v1 = admit(&stored_json("delta", "d-2025-11"));
    assert_eq!(v1.schema_version, 1);
    assert!(
        v1.verification_stack.is_none(),
        "a schema-v1 collection carries no attestation and is still readable"
    );
}

/// One producer invocation lands atomically, an identical republication is
/// idempotent, and an invalid one leaves no collection behind.
///
/// FR-044-AC-2: "A producer invocation lands by one same-directory atomic
/// rename after validation. A partial or invalid invocation leaves no
/// collection, and writing identical bytes for the same id is idempotent."
///
/// Trace: FR-044-AC-2
/// Provenance: quoin#479
#[test]
fn tc_479_005_a_collection_lands_atomically_and_an_identical_rewrite_is_idempotent() {
    let temporary = planned_repository();
    let root = temporary.path();
    let candidate = from_serde(&new_collection_json()).expect("the candidate crosses the bridge");

    let path = write_measurement_collection(root, &candidate).expect("the collection is published");
    let before = std::fs::read(&path).expect("the published collection is readable");
    assert!(!before.is_empty(), "an empty file is not a collection");

    // Read-back is the same collection, through the store's own reader.
    let source = DiskMeasurement::new(root);
    let collections =
        quoin_measurement::store::read_measurement_collections(&source).expect("the store reads");
    assert_eq!(collections.len(), 1, "one invocation, one collection");
    assert_eq!(collections[0].collection_id.as_str(), "run-001");

    // Identical bytes for the same id are idempotent: same path, same bytes.
    let again = write_measurement_collection(root, &candidate)
        .expect("an identical republication is idempotent");
    assert_eq!(again, path, "the same id must resolve to the same file");
    assert_eq!(
        std::fs::read(&path).expect("the collection is still readable"),
        before,
        "an idempotent republication must not rewrite the bytes"
    );

    // Differing bytes for the same id are a named collision, not an overwrite.
    let mut clashing = new_collection_json();
    clashing["observations"][0]["value"] = json!(0.75);
    let refusal = write_measurement_collection(
        root,
        &from_serde(&clashing).expect("the clashing case crosses the bridge"),
    )
    .expect_err("differing bytes under one id are refused");
    assert_eq!(refusal.code(), MeasurementErrorCode::CollectionIdCollision);
    assert_eq!(
        std::fs::read(&path).expect("the collection survives the refusal"),
        before,
        "a refused write must not have touched the retained bytes"
    );

    // An invalid invocation leaves nothing behind at all.
    let mut invalid = new_collection_json();
    invalid["collectionId"] = json!("run-002");
    invalid["observations"][0]["metric"] = json!("quality.unplanned");
    let refusal = write_measurement_collection(
        root,
        &from_serde(&invalid).expect("the invalid case crosses the bridge"),
    )
    .expect_err("an unplanned observation is refused before anything is written");
    assert_eq!(refusal.code(), MeasurementErrorCode::CollectionInvalid);
    let names: Vec<String> = std::fs::read_dir(measurements_root(root))
        .expect("the store directory is listable")
        .map(|entry| {
            entry
                .expect("a store entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(
        names,
        vec!["run-001.json".to_owned()],
        "a refused invocation must leave no partial collection; the store holds {names:?}"
    );
}

/// A named `verificationStack.artifacts` entry that resolves to a real local
/// file under `repo` is truth-checked automatically, not merely trusted
/// because it has the right shape.
///
/// PLAT-931: server-side verification is not opt-in — no `--digest-from-file`
/// flag is passed here at all. `write_measurement_collection` is the intake
/// every caller of `measurement.record` goes through, so this is the real
/// path, not the CLI convenience layered on top of it.
///
/// Trace: FR-044-AC-1
/// Provenance: PLAT-931
#[test]
fn tc_479_023_a_locally_reachable_artifact_digest_is_verified_automatically() {
    let temporary = planned_repository();
    let root = temporary.path();
    let matching = "sha256:4659fc0570122b0e0aa14f4ff7c261b1fe51795a01ba79963f462ebf40d7520d";
    std::fs::create_dir_all(root.join("dist"))
        .unwrap_or_else(|error| panic!("cannot create the local artifact fixture dir: {error}"));
    std::fs::write(root.join("dist/quoin"), b"artifact bytes")
        .unwrap_or_else(|error| panic!("cannot write the local artifact fixture: {error}"));

    // A submitted digest that matches the local bytes is admitted.
    let mut agreeing = new_collection_json();
    agreeing["verificationStack"]["artifacts"] = json!({ "dist/quoin": matching });
    write_measurement_collection(
        root,
        &from_serde(&agreeing).expect("the agreeing case crosses the bridge"),
    )
    .expect("a submitted digest matching the local file is admitted");

    // A submitted digest that disagrees with the local bytes is refused,
    // without ever being told to check by a flag.
    let mut disagreeing = new_collection_json();
    disagreeing["collectionId"] = json!("run-002");
    disagreeing["verificationStack"]["artifacts"] =
        json!({ "dist/quoin": format!("sha256:{}", "9".repeat(64)) });
    let refusal = write_measurement_collection(
        root,
        &from_serde(&disagreeing).expect("the disagreeing case crosses the bridge"),
    )
    .expect_err("a submitted digest that disagrees with the local file is refused");
    assert_eq!(refusal.code(), MeasurementErrorCode::CollectionInvalid);
    assert!(
        refusal.to_string().contains("artifacts.dist/quoin"),
        "the refusal must name the artifact; it said {refusal}"
    );

    // A name the record does not carry a matching local file for is not
    // verifiable here, so the submitted digest is still only checked for
    // shape, exactly as before this check existed.
    let mut unreachable = new_collection_json();
    unreachable["collectionId"] = json!("run-003");
    unreachable["verificationStack"]["artifacts"] =
        json!({ "no-such-file": format!("sha256:{}", "9".repeat(64)) });
    write_measurement_collection(
        root,
        &from_serde(&unreachable).expect("the unreachable case crosses the bridge"),
    )
    .expect("an artifact name with no local file is trusted on shape alone, as before");
}

/// `configDigest` is not a member of `verificationStack` itself, but it is
/// checked for sha256 shape at the same schemaVersion-2 gate as `lockDigest`
/// and `executableDigest`, and it joins the same accumulation rather than
/// hiding behind whichever defect the code happens to check first.
///
/// PLAT-939: `configDigest` was accepted on any non-empty string, unlike its
/// two `verificationStack` siblings, which already required a full sha256
/// digest (PLAT-929/930/931). Closing that gap, without starting to enforce
/// it retroactively on schemaVersion 1's historical evidence, is this test.
///
/// Trace: FR-044-AC-1
/// Provenance: PLAT-939
#[test]
fn tc_479_024_config_digest_must_be_a_full_sha256_digest() {
    let temporary = planned_repository();
    let plans = authored_plans(temporary.path());

    let mut malformed = new_collection_json();
    malformed["configDigest"] = json!("not-a-digest");
    let refusal = validate::measurement_collection(
        &from_serde(&malformed).expect("the malformed case crosses the bridge"),
        &plans,
    )
    .expect_err("a configDigest that is not a full sha256 digest is refused");
    assert_eq!(refusal.code(), MeasurementErrorCode::CollectionInvalid);
    assert!(
        refusal
            .findings()
            .iter()
            .any(|finding| finding == "collection.configDigest must be a full sha256 digest"),
        "the refusal must name `collection.configDigest`, not `verificationStack.configDigest` — \
         configDigest lives on the collection envelope, not inside verificationStack; got {:?}",
        refusal.findings()
    );

    // It joins the same accumulation as its `verificationStack` siblings
    // (PLAT-929's rule, extended to this member), rather than only the first
    // defect surfacing.
    let mut two_defects = new_collection_json();
    two_defects["configDigest"] = json!("not-a-digest");
    two_defects["verificationStack"]["lockDigest"] = json!("also-not-a-digest");
    let refusal = validate::measurement_collection(
        &from_serde(&two_defects).expect("the two-defect case crosses the bridge"),
        &plans,
    )
    .expect_err("a malformed configDigest and a malformed lockDigest are both refused");
    assert!(
        refusal
            .findings()
            .iter()
            .any(|finding| finding == "collection.configDigest must be a full sha256 digest"),
        "the one refusal must name `collection.configDigest`, got {:?}",
        refusal.findings()
    );
    assert!(
        refusal
            .findings()
            .iter()
            .any(|finding| finding.contains("lockDigest")),
        "the SAME refusal must also name `lockDigest`, not just the first defect found; got {:?}",
        refusal.findings()
    );

    // schemaVersion 1 is historical, read-only evidence: it never required
    // `verificationStack`, and this fix does not start requiring
    // `configDigest`'s shape there either — only new (schemaVersion 2)
    // collections are checked, exactly as `lockDigest` and `executableDigest`
    // already are.
    let historical = json!({
        "schemaVersion": 1,
        "collectionId": "historical-001",
        "subject": "fixture",
        "scope": {},
        "toolIdentity": "fixture producer",
        "toolVersion": "fixture 1",
        "configDigest": "not-a-digest",
        "timestamp": "2026-01-01T00:00:00.000Z",
        "sourceRevision": "aaaaaaaaaaaaaaaa",
        "environment": {},
        "rawEvidence": [],
        "observations": [
            {
                "metric": "quality.example",
                "planId": "MP-001",
                "definitionVersion": "quality.example-v1",
                "state": "measured",
                "value": 0.5,
                "unit": "fraction",
                "shape": "ratio",
            },
        ],
    });
    validate::stored_measurement_collection(
        &from_serde(&historical).expect("the historical case crosses the bridge"),
    )
    .expect("a schemaVersion-1 collection keeps accepting any non-empty configDigest");
}

/// A moved definition and a moved producer configuration each block the delta,
/// and the refusal names both.
///
/// FR-044-AC-3: "Comparison refuses changed definitions, changed producer
/// configuration, and incomplete populations with actionable reasons […]".
///
/// Trace: FR-044-AC-3
/// Provenance: quoin#479
#[test]
fn tc_479_006_a_moved_definition_and_configuration_refuse_a_delta_and_name_both() {
    let before = admit(&new_collection_json());
    let mut after_json = new_collection_json();
    after_json["collectionId"] = json!("run-002");
    after_json["configDigest"] = json!(format!("sha256:{}", "b".repeat(64)));
    after_json["observations"][0]["definitionVersion"] = json!("quality.example-v2");
    after_json["observations"][0]["value"] = json!(0.75);
    let after = admit(&after_json);

    let rows =
        compare_measurement_collections(&before, &after).expect("the two collections compare");
    assert_eq!(rows.len(), 1, "one slice on each side is one compared row");
    let row = &rows[0];
    assert_eq!(row.status, ComparisonStatus::Incomparable);
    assert_eq!(row.delta, None, "a blocked row carries no delta");
    assert_eq!(
        row.reasons
            .iter()
            .map(|reason| reason.code)
            .collect::<Vec<_>>(),
        vec![
            ComparisonReasonCode::DefinitionChanged,
            ComparisonReasonCode::ConfigurationChanged,
        ],
        "both causes must be named, in the order the oracle names them"
    );
    let said = row
        .reasons
        .iter()
        .map(|reason| reason.message.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        said.contains("quality.example") && said.contains("config"),
        "the reasons must be actionable — they said {said:?}"
    );
    assert!(
        row.reasons.iter().all(|reason| reason.blocking),
        "a definition or configuration move blocks; neither is advisory"
    );

    // The third refusal the criterion names, on its own: an incomplete
    // population blocks even when definition and configuration agree.
    let mut incomplete = new_collection_json();
    incomplete["collectionId"] = json!("run-003");
    incomplete["observations"][0]["population"]["complete"] = json!(false);
    let rows = compare_measurement_collections(&before, &admit(&incomplete))
        .expect("the incomplete pair compares");
    assert_eq!(rows[0].status, ComparisonStatus::Incomparable);
    assert_eq!(rows[0].delta, None);
    assert!(
        rows[0]
            .reasons
            .iter()
            .any(|reason| reason.code == ComparisonReasonCode::IncompletePopulation),
        "an incomplete population must name itself, got {:?}",
        rows[0]
            .reasons
            .iter()
            .map(|reason| reason.code)
            .collect::<Vec<_>>()
    );
}

/// Population movement is reported beside the delta, and nothing is graded.
///
/// FR-044-AC-3: "[…] surfaces tool and population movement […] and emits no
/// quality verdict or severity."
///
/// Trace: FR-044-AC-3
/// Provenance: quoin#479
#[test]
fn tc_479_007_population_movement_sits_beside_the_delta_and_nothing_is_graded() {
    let before = admit(&new_collection_json());
    let mut after_json = new_collection_json();
    after_json["collectionId"] = json!("run-002");
    after_json["observations"][0]["value"] = json!(0.75);
    after_json["observations"][0]["population"] = json!({
        "examined": 4,
        "matched": 3,
        "complete": true,
        "identity": ["a", "b", "c", "d"],
    });
    let after = admit(&after_json);

    let rows =
        compare_measurement_collections(&before, &after).expect("the two collections compare");
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.status, ComparisonStatus::Comparable);
    assert_eq!(
        row.delta,
        Some(0.25),
        "a population change warns beside the delta rather than withdrawing it"
    );
    assert!(
        row.reasons
            .iter()
            .any(|reason| reason.code == ComparisonReasonCode::PopulationChanged),
        "the movement must be stated, got {:?}",
        row.reasons
            .iter()
            .map(|reason| reason.code)
            .collect::<Vec<_>>()
    );
    assert!(
        row.reasons
            .iter()
            .filter(|reason| reason.code == ComparisonReasonCode::PopulationChanged)
            .all(|reason| !reason.blocking),
        "a population change is context, not a block"
    );

    // The tool half of "tool and population movement", on its own.
    let mut retooled = new_collection_json();
    retooled["collectionId"] = json!("run-003");
    retooled["toolVersion"] = json!("fixture 2 (engine a2)");
    let rows = compare_measurement_collections(&before, &admit(&retooled))
        .expect("the retooled pair compares");
    assert!(
        rows[0]
            .reasons
            .iter()
            .any(|reason| reason.code == ComparisonReasonCode::ToolChanged),
        "a moved tool version must be surfaced"
    );
    assert_eq!(
        rows[0].status,
        ComparisonStatus::Comparable,
        "a moved tool version is context and does not block"
    );

    // Nothing in the comparison vocabulary is a grade. Every reason code and
    // every status is checked, not the two this test happened to produce.
    assert_eq!(
        ComparisonReasonCode::ALL.len(),
        5,
        "every reason code is measured, not a subset"
    );
    for code in ComparisonReasonCode::ALL {
        let spelling = code.as_str();
        for graded in ["verdict", "severity", "pass", "fail", "score"] {
            assert!(
                !spelling.contains(graded),
                "`{spelling}` reads as a grade: it contains `{graded}`"
            );
        }
    }
    for status in ComparisonStatus::ALL {
        let spelling = status.as_str();
        for graded in ["verdict", "severity", "pass", "fail"] {
            assert!(
                !spelling.contains(graded),
                "`{spelling}` reads as a grade: it contains `{graded}`"
            );
        }
    }
}

/// A metric present in only one collection is `not_computed`, never an
/// unchanged zero.
///
/// FR-044-AC-3: "[…] reports a missing metric as `not_computed` […]".
///
/// Trace: FR-044-AC-3
/// Provenance: quoin#479
#[test]
fn tc_479_008_a_metric_in_only_one_collection_is_not_computed() {
    let before = admit(&new_collection_json());
    // `observations` cannot be empty, so the metric is dropped by REPLACING
    // the slice rather than by emptying the list — which is what a producer
    // that stopped computing one metric and kept computing another does.
    let mut after_json = new_collection_json();
    after_json["collectionId"] = json!("run-002");
    after_json["observations"][0]["metric"] = json!("quality.other");
    after_json["observations"][0]["planId"] = json!("MP-002");
    let after = admit(&after_json);

    let rows =
        compare_measurement_collections(&before, &after).expect("the two collections compare");
    assert_eq!(rows.len(), 2, "neither side's slice is dropped");
    let dropped = rows
        .iter()
        .find(|row| row.metric == "quality.example")
        .expect("the metric only the baseline computed is still a row");
    assert_eq!(dropped.status, ComparisonStatus::NotComputed);
    assert_eq!(dropped.before, Some(0.5), "the side that measured is kept");
    assert_eq!(
        dropped.after, None,
        "the side that did not measure is absent, not zero"
    );
    assert_eq!(
        dropped.delta, None,
        "an absence has no delta, not a zero one"
    );

    let arrived = rows
        .iter()
        .find(|row| row.metric == "quality.other")
        .expect("the metric only the current collection computed is a row");
    assert_eq!(arrived.status, ComparisonStatus::NotComputed);
    assert_eq!(arrived.before, None);
    assert_eq!(arrived.after, Some(0.5));
}

/// `quoin report` is a deterministic store view that keeps an authored plan
/// with no record visible as `not_computed`.
///
/// FR-044-AC-4: "`quoin report` is a deterministic store view. It shows every
/// active plan, a plan without a record as `not_computed`, all dimensional
/// observations, corpus gap count, full producer provenance, and factual
/// attention items."
///
/// Trace: FR-044-AC-4
/// Provenance: quoin#479
#[test]
fn tc_479_009_the_report_is_deterministic_and_keeps_an_unmeasured_plan_visible() {
    // `bravo` authors one plan and has never recorded a collection — the exact
    // shape `tests/measurement.test.ts:244` built by hand.
    let bravo = repo_in_tree("bravo");
    let source = DiskMeasurement::new(&bravo);
    let report = build_measurement_report(&source, &bravo).expect("bravo's report builds");
    let first = render_measurement_report(&report).expect("the report renders");
    assert_eq!(
        render_measurement_report(
            &build_measurement_report(&source, &bravo).expect("bravo's report builds again")
        )
        .expect("the report renders again"),
        first,
        "two renders of an unchanged store must be byte-identical"
    );
    for literal in [
        "not_computed: no record",
        "Corpus gaps: not_computed",
        "finding_recall",
    ] {
        assert!(
            first.contains(literal),
            "the report must state {literal:?}; it said:\n{first}"
        );
    }

    // `alpha` is the populated half of the criterion: every active plan, every
    // dimensional observation, the gap count, the provenance line and the
    // factual attention items.
    let alpha = repo_in_tree("alpha");
    let report = build_measurement_report(&DiskMeasurement::new(&alpha), &alpha)
        .expect("alpha's report builds");
    assert!(
        report.plans.len() >= 5,
        "alpha authors {} active plans, below the tree's own census",
        report.plans.len()
    );
    let rendered = render_measurement_report(&report).expect("alpha's report renders");
    for literal in [
        "| Metric | Plan | Stage | Current |",
        "finding_recall [language=python] | ap-recall (spec/assurance/10-recall.md) | observe | 0.5 ratio |",
        "finding_recall [language=rust, tier=2] | ap-recall (spec/assurance/10-recall.md) | observe | 1e+21 ratio |",
        "unmeasured_metric | ap-unmeasured (spec/assurance/50-unmeasured.md) | gate | not_computed: no record |",
        "Corpus gaps: 3",
        "2026-03-01T00:00:00.000Z — quoin 0.9.1; source revision-three; corpus n/a; config \
         sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "- unmeasured_metric: no collection has computed this authored plan.",
    ] {
        assert!(
            rendered.contains(literal),
            "the report must state {literal:?}; it said:\n{rendered}"
        );
    }
}

/// The comparison view renders the report object it is handed and nothing it
/// went and computed for itself.
///
/// FR-044-AC-4: "JSON, series, and since-revision views read the same store and
/// accept no typed value."
///
/// Trace: FR-044-AC-4
/// Provenance: quoin#479
#[test]
fn tc_479_010_the_comparison_view_renders_the_report_object_it_is_handed() {
    // The object `tests/measurement.test.ts:252-288` built by hand, restated.
    // Nothing here reads a store: that is the point — the renderer is handed a
    // report and must render exactly it.
    let reference = |id: &str, timestamp: &str, tool_version: &str, revision: &str| {
        MeasurementCollectionReference {
            collection_id: id.to_owned(),
            timestamp: timestamp.to_owned(),
            tool_identity: "fixture".to_owned(),
            tool_version: tool_version.to_owned(),
            config_digest: "config-a".to_owned(),
            source_revision: revision.to_owned(),
            corpus_revision: Some("corpus-a".to_owned()),
            corpus_gaps: Some(0.0),
            path: format!("/repo/{id}.json"),
        }
    };
    let report = MeasurementComparisonReport {
        status: MeasurementComparisonStatus::Compared,
        before: Some(reference("run-001", "2026-08-26T00:00:00Z", "1", "before")),
        after: Some(reference("run-002", "2026-08-26T01:00:00Z", "2", "after")),
        comparisons: vec![MeasurementComparison {
            metric: "quality.example".to_owned(),
            dimensions: std::collections::BTreeMap::new(),
            before: Some(0.5),
            after: Some(0.75),
            delta: Some(0.25),
            status: ComparisonStatus::Comparable,
            reasons: Vec::new(),
        }],
    };

    let rendered = render_measurement_comparison(&report).expect("the comparison renders");
    for literal in [
        "# QA measurement comparison",
        "| quality.example | comparable | 0.5 | 0.75 | 0.25 |",
        "fixture 2; source after; corpus corpus-a; gaps 0; config config-a",
    ] {
        assert!(
            rendered.contains(literal),
            "the comparison must state {literal:?}; it said:\n{rendered}"
        );
    }
    assert_eq!(
        render_measurement_comparison(&report).expect("the comparison renders again"),
        rendered,
        "two renders of one report object must be byte-identical"
    );
}

/// A missing baseline is `not_computed` and still carries the current
/// collection's provenance.
///
/// `tests/measurement.test.ts:298`, which carried no criterion tag. It is the
/// since-revision view FR-044-AC-4 names, refusing rather than inventing a
/// baseline, so it is tagged there.
///
/// Trace: FR-044-AC-4
/// Provenance: quoin#479
#[test]
fn tc_479_011_a_missing_baseline_is_not_computed_with_current_provenance() {
    // `delta` holds exactly one collection, so no revision but its own can be
    // a baseline.
    let delta = repo_in_tree("delta");
    let report = comparison_for(&DiskMeasurement::new(&delta), &delta, "missing")
        .expect("a missing baseline is a report, not an error");

    assert_eq!(
        report.status,
        MeasurementComparisonStatus::NotComputed(
            "no baseline measurement collection for source revision missing".to_owned()
        ),
        "the refusal must name the revision it looked for"
    );
    assert!(
        report.before.is_none(),
        "a baseline that does not exist is absent, not invented"
    );
    assert!(
        report.comparisons.is_empty(),
        "nothing is compared against a baseline that was not found"
    );
    let after = report
        .after
        .as_ref()
        .expect("the current collection's provenance survives the refusal");
    assert_eq!(after.collection_id, "d-2025-11");
    assert_eq!(after.tool_identity, "quoin");
    assert_eq!(after.source_revision, "revision-one");

    let rendered = render_measurement_comparison(&report).expect("the comparison renders");
    assert!(
        rendered.contains("Before: not_computed — no baseline"),
        "the human view must state the absence; it said:\n{rendered}"
    );
    assert!(
        rendered.contains("d-2025-11"),
        "the human view must still carry the current provenance"
    );
}

/// Repeated repository locations produce one report entry.
///
/// FR-045-AC-1: "One command accepts repeated repository locations and shows
/// each readable repository's active `AssuranceProfiles` and `MeasurementPlans`
/// […]".
///
/// # The half this test cannot carry
///
/// AC-1's other half — that `quoin report` exposes repeated `--portfolio`
/// flags and no observation-value flag — is a COMMAND-SURFACE property of
/// `src/commands/report.ts`, which FR-101 does not delete. It has no Rust home
/// because this crate has no command surface. See this file's report.
///
/// Trace: FR-045-AC-1
/// Provenance: quoin#479
#[test]
fn tc_479_012_repeated_repository_locations_produce_one_report_entry() {
    let alpha = repo_in_tree("alpha");
    let bravo = repo_in_tree("bravo");
    let report =
        build_portfolio_report(&[alpha.clone(), bravo.clone(), alpha.clone(), alpha, bravo]);

    let names: Vec<&str> = report
        .repositories
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    assert_eq!(
        names,
        vec!["alpha", "bravo"],
        "five locations naming two repositories are two entries, ordered"
    );

    // Each readable repository shows its active profiles and plans, which is
    // the rest of the criterion's first sentence.
    let alpha_entry = repository(&report, "alpha");
    assert_eq!(alpha_entry.status, RepositoryStatus::Readable);
    assert_eq!(alpha_entry.store, StoreState::Present);
    assert_eq!(
        alpha_entry
            .profiles
            .iter()
            .map(|profile| profile.id.as_str())
            .collect::<Vec<_>>(),
        vec!["ap-core"],
        "the retired profile beside it is not shown; only active ones are"
    );
    assert!(
        alpha_entry.plans.len() >= 5,
        "alpha shows {} active plans, below the tree's own census",
        alpha_entry.plans.len()
    );
    let latest = alpha_entry
        .latest_collection
        .as_ref()
        .expect("a present store has a latest collection");
    assert_eq!(latest.id, "c-2026-03");
    assert_eq!(latest.source_revision, "revision-three");

    // An explicit `not_computed` row, rather than an omitted metric.
    let measurements = alpha_entry
        .measurements
        .as_ref()
        .expect("a readable repository carries its measurement report");
    let unmeasured = measurements
        .current
        .iter()
        .find(|row| row.metric == "unmeasured_metric")
        .expect("an authored plan with no record is still a row");
    assert!(
        unmeasured.observation.is_none() && unmeasured.collection.is_none(),
        "an unmeasured plan names no observation and no collection"
    );
}

/// A corpus-oriented root `assurance` directory is a governed store.
///
/// FR-045-AC-1: "[…] from either `spec/assurance` or the corpus-oriented root
/// `assurance` layout […]".
///
/// # Where the input comes from
///
/// The committed tree holds no repository in the root layout, so one is made
/// here by copying `delta` — its authored plan, its retained collection — and
/// `charlie`'s authored profile, then MOVING the assurance directory to the
/// root. Only the layout is the mutation; every byte read is a committed one,
/// and the expectations below are `delta`'s own captured identities with the
/// path prefix the move implies.
///
/// Trace: FR-045-AC-1
/// Provenance: quoin#479
#[test]
fn tc_479_013_a_corpus_oriented_root_assurance_directory_is_a_governed_store() {
    let temporary = tempfile::tempdir().expect("a temporary directory");
    let root = temporary.path().join("root-assurance");
    let assurance = root.join("assurance");
    std::fs::create_dir_all(&assurance).expect("the root assurance directory is creatable");

    // The authored plan and profile, in the root layout.
    std::fs::copy(
        repo_in_tree("delta")
            .join("spec")
            .join("assurance")
            .join("10-recall.md"),
        assurance.join("10-recall.md"),
    )
    .expect("delta's plan is copied");
    std::fs::copy(
        repo_in_tree("charlie")
            .join("spec")
            .join("assurance")
            .join("00-profile.md"),
        assurance.join("00-profile.md"),
    )
    .expect("charlie's profile is copied");

    // The retained collection stays where the store is: the layout under test
    // is the assurance layout, not the evidence one.
    let store = measurements_root(&root);
    std::fs::create_dir_all(&store).expect("the store directory is creatable");
    std::fs::copy(
        repo_in_tree("delta")
            .join("spec")
            .join("evidence")
            .join("measurements")
            .join("d-2025-11.json"),
        store.join("d-2025-11.json"),
    )
    .expect("delta's collection is copied");

    let report = build_portfolio_report(&[root]);
    let entry = repository(&report, "root-assurance");
    assert_eq!(entry.status, RepositoryStatus::Readable);
    assert_eq!(
        entry.store,
        StoreState::Present,
        "a root-assurance repository's evidence store is still a store"
    );
    assert_eq!(
        entry
            .profiles
            .iter()
            .map(|profile| (profile.id.as_str(), profile.path.as_str()))
            .collect::<Vec<_>>(),
        vec![("cp-core", "assurance/00-profile.md")],
        "the profile is found at the root layout and names that path"
    );
    assert_eq!(
        entry
            .plans
            .iter()
            .map(|plan| (plan.id.as_str(), plan.path.as_str()))
            .collect::<Vec<_>>(),
        vec![("dp-recall", "assurance/10-recall.md")],
        "the plan is found at the root layout and names that path"
    );
    assert_eq!(
        entry
            .latest_collection
            .as_ref()
            .map(|collection| collection.id.as_str()),
        Some("d-2025-11"),
        "the governed store is read, not merely located"
    );
}

/// Missing repositories, unreadable inputs, absent stores and stale evidence
/// are named independently, and none of them is a zero.
///
/// FR-045-AC-2: "Missing repositories, unreadable collections, missing stores,
/// and repositories whose latest collection is more than 30 days behind the
/// newest portfolio collection are named independently. A missing or unreadable
/// value is never zero."
///
/// Trace: FR-045-AC-2
/// Provenance: quoin#479
#[test]
fn tc_479_014_missing_unreadable_storeless_and_stale_repositories_stay_distinct() {
    let report = build_portfolio_report(&every_location());
    assert!(
        report.repositories.len() >= 7,
        "the census read {} repositories, below the committed tree's own seven",
        report.repositories.len()
    );

    // A location that does not exist.
    let zulu = repository(&report, "zulu-does-not-exist");
    assert_eq!(zulu.store, StoreState::Missing);
    assert_eq!(
        zulu.status.as_str(),
        "missing",
        "an absent location is missing, not unreadable"
    );
    assert!(
        zulu.status
            .error()
            .is_some_and(|error| error.contains("repository does not exist")),
        "the absence must be stated, got {:?}",
        zulu.status.error()
    );

    // A location that is there and is not a repository.
    let echo = repository(&report, "echo-not-a-directory");
    assert_eq!(echo.status.as_str(), "unreadable");
    assert!(
        echo.status
            .error()
            .is_some_and(|error| error.contains("is not a directory")),
        "the refusal must say what was wrong with the location"
    );

    // A repository whose collection cannot be read.
    let golf = repository(&report, "golf");
    assert_eq!(golf.status.as_str(), "unreadable");
    assert_eq!(golf.store, StoreState::Unreadable);
    assert!(
        golf.status.error().is_some_and(|error| {
            error.contains("g-undated.json") && error.contains("not a valid date")
        }),
        "an unreadable collection must be named by file and by cause, got {:?}",
        golf.status.error()
    );

    // A repository with no store at all, and one with an empty store: two
    // different facts, not one.
    let charlie = repository(&report, "charlie");
    assert_eq!(charlie.status, RepositoryStatus::Readable);
    assert_eq!(charlie.store, StoreState::Missing);
    let bravo = repository(&report, "bravo");
    assert_eq!(bravo.status, RepositoryStatus::Readable);
    assert_eq!(
        bravo.store,
        StoreState::Empty,
        "an empty store is not a missing one"
    );

    // Staleness is measured against the portfolio's own newest collection.
    let delta = repository(&report, "delta");
    assert_eq!(delta.staleness.as_str(), "stale");
    assert_eq!(delta.staleness.age_days(), Some(120));
    assert_eq!(
        delta.staleness.relative_to(),
        Some("2026-03-01T00:00:00.000Z"),
        "the age is relative to the newest collection in the portfolio, not a wall clock"
    );
    let alpha = repository(&report, "alpha");
    assert_eq!(alpha.staleness.as_str(), "current");
    assert_eq!(alpha.staleness.age_days(), Some(0));
    assert!(
        delta
            .staleness
            .age_days()
            .is_some_and(|age| age > PORTFOLIO_STALE_AFTER_DAYS),
        "delta is stale because it is past the {PORTFOLIO_STALE_AFTER_DAYS}-day threshold, \
         not by coincidence"
    );
    assert!(
        alpha
            .staleness
            .age_days()
            .is_some_and(|age| age <= PORTFOLIO_STALE_AFTER_DAYS),
        "alpha is current because it is inside the threshold"
    );

    // No absence is a zero. `charlie` and `bravo` have no collection and no
    // age; `zulu`, `echo` and `golf` have no measurements at all.
    for name in ["charlie", "bravo"] {
        let entry = repository(&report, name);
        assert_eq!(
            entry.staleness.age_days(),
            None,
            "{name}: a repository with no collection has no age, not an age of zero"
        );
        assert!(
            entry.latest_collection.is_none(),
            "{name}: a repository with no collection quotes none"
        );
    }
    for name in ["zulu-does-not-exist", "echo-not-a-directory", "golf"] {
        let entry = repository(&report, name);
        assert!(
            entry.measurements.is_none(),
            "{name}: an unread repository reports no measurements, not empty ones"
        );
        assert_eq!(
            entry.staleness.age_days(),
            None,
            "{name}: an unread repository has no age"
        );
    }
}

/// Every observation names its plan path and its collection path, and the
/// latest-pair comparison is FR-044's own.
///
/// FR-045-AC-3: "Each observation names its plan path and collection path.
/// Latest-pair comparisons reuse FR-044 compatibility results, including
/// changed definitions, without combining values across repositories or
/// assigning a verdict."
///
/// Trace: FR-045-AC-3
/// Provenance: quoin#479
#[test]
fn tc_479_015_every_observation_names_its_plan_and_collection_path() {
    let report = build_portfolio_report(&every_location());
    let alpha = repository(&report, "alpha");
    let measurements = alpha
        .measurements
        .as_ref()
        .expect("a readable repository carries its measurement report");
    assert!(
        measurements.current.len() >= 5,
        "the census read {} rows, below the tree's own plan count",
        measurements.current.len()
    );

    let mut measured = 0_usize;
    for row in &measurements.current {
        assert!(
            Path::new(&row.plan_path)
                .extension()
                .is_some_and(|extension| extension == "md")
                && row.plan_path.starts_with("spec/assurance/"),
            "{}: an observation must name its plan document, got {:?}",
            row.metric,
            row.plan_path
        );
        match (row.observation.as_ref(), row.collection.as_ref()) {
            (Some(_), Some(collection)) => {
                assert!(
                    collection
                        .path
                        .ends_with(&format!("{}.json", collection.collection_id)),
                    "{}: the collection path must name the collection it quotes, got {:?}",
                    row.metric,
                    collection.path
                );
                measured += 1;
            }
            (None, None) => {}
            (observation, collection) => panic!(
                "{}: an observation and its collection must arrive together, got {:?} and {:?}",
                row.metric,
                observation.is_some(),
                collection.is_some()
            ),
        }
    }
    assert!(
        measured >= 4,
        "only {measured} rows carried an observation; a path census over none measures nothing"
    );

    // The latest-pair comparison rows ARE FR-044's `MeasurementComparison`, and
    // no value crosses a repository boundary.
    let comparison = alpha
        .comparison
        .as_ref()
        .expect("two collections are a comparable pair");
    assert!(
        !comparison.observations.is_empty(),
        "a comparison over no rows compares nothing"
    );
    assert_eq!(comparison.before.id, "b-2026-02");
    assert_eq!(comparison.after.id, "c-2026-03");
    for row in &comparison.observations {
        for reason in &row.reasons {
            assert!(
                ComparisonReasonCode::from_wire(reason.code.as_str()).is_some(),
                "{}: `{}` is not an FR-044 compatibility reason",
                row.metric,
                reason.code.as_str()
            );
        }
    }
    assert!(
        repository(&report, "delta").comparison.is_none(),
        "one collection is not a pair, and no other repository's is borrowed"
    );

    a_definition_move_reaches_the_portfolio();
}

/// The human and canonical JSON views render one loaded report object
/// deterministically, and the human view says it computes no aggregate.
///
/// FR-045-AC-4: "Human and canonical JSON output render the same loaded report
/// object deterministically, and the human view explicitly states that it
/// computes no cross-repository aggregate."
///
/// Trace: FR-045-AC-4
/// Provenance: quoin#479
#[test]
fn tc_479_016_the_two_views_render_one_report_object_deterministically() {
    let report = build_portfolio_report(&every_location());
    let human = render_portfolio_report(&report).expect("the human view renders");
    let canonical = render_portfolio_report_json(&report).expect("the JSON view renders");

    assert_eq!(
        render_portfolio_report(&report).expect("the human view renders again"),
        human,
        "two renders of one report object must be byte-identical"
    );
    assert_eq!(
        render_portfolio_report_json(&report).expect("the JSON view renders again"),
        canonical,
        "two renders of one report object must be byte-identical"
    );

    // The two views are the same object, not two walks of the store: every
    // repository the object holds appears in both, with the same identities.
    let parsed: Value = serde_json::from_str(&canonical).expect("the JSON view is JSON");
    let rows = parsed["repositories"]
        .as_array()
        .expect("the JSON view carries repositories");
    assert_eq!(
        rows.len(),
        report.repositories.len(),
        "the JSON view must hold exactly the object's repositories"
    );
    for (entry, row) in report.repositories.iter().zip(rows) {
        assert_eq!(row["name"], json!(entry.name));
        assert_eq!(row["root"], json!(entry.root));
        assert_eq!(row["status"], json!(entry.status.as_str()));
        assert_eq!(row["store"], json!(entry.store.as_str()));
        assert_eq!(row["staleness"]["status"], json!(entry.staleness.as_str()));
        assert_eq!(
            row["latestCollection"]
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_owned),
            entry
                .latest_collection
                .as_ref()
                .map(|collection| collection.id.clone()),
            "{}: the JSON view's latest collection must be the object's",
            entry.name
        );
        assert!(
            human.contains(&entry.name),
            "{}: the human view must show every repository the object holds",
            entry.name
        );
    }

    // The human view states the absence of an aggregate rather than leaving it
    // to be inferred, and invents none.
    assert!(
        human.contains(
            "No cross-repository metric is summed, averaged, or assigned a quality verdict."
        ),
        "the human view must say it computes no aggregate; it said:\n{human}"
    );
    for forbidden in [
        "overall quality",
        "portfolio quality",
        "aggregate score",
        "aggregateScore",
        "qualityVerdict",
    ] {
        assert!(
            !human.contains(forbidden),
            "the human view invented `{forbidden}`"
        );
        assert!(
            !canonical.contains(forbidden),
            "the JSON view invented `{forbidden}`"
        );
    }
}

/// The store's reader reads each collection independently, so one corrupt
/// record hides no sibling.
///
/// FR-067-AC-8 (as `tests/measurement-store-results.test.ts` carries it):
/// corrupt collections become local gaps that hide nothing beside them. The
/// function under test, `read_measurement_collection_results`, is the store's
/// and not the deleted graph module's — which is why that assertion was split
/// out rather than deleted with `graph-portfolio.ts` (quoin#480), and why it is
/// restated here against the crate that now owns it.
///
/// Trace: FR-067-AC-8
/// Provenance: quoin#479
#[test]
fn tc_479_017_each_collection_is_read_independently_so_corruption_hides_no_sibling() {
    let temporary = tempfile::tempdir().expect("a temporary directory");
    let root = temporary.path();
    let store = measurements_root(root);
    std::fs::create_dir_all(&store).expect("the store directory is creatable");

    // Two readable collections, taken from the committed tree and re-dated so
    // the listing order and the timestamp order disagree — which is what makes
    // "read each one independently" observable rather than incidental.
    let retained = |id: &str, timestamp: &str| -> Vec<u8> {
        let mut value = stored_json("delta", "d-2025-11");
        value["collectionId"] = json!(id);
        value["timestamp"] = json!(timestamp);
        serde_json::to_vec(&value).expect("a collection serialises")
    };
    std::fs::write(
        store.join("a-new.json"),
        retained("a-new", "2026-08-30T23:00:00Z"),
    )
    .expect("the first collection is writable");
    std::fs::write(
        store.join("z-old.json"),
        retained("z-old", "2026-08-31T00:30:00+02:00"),
    )
    .expect("the second collection is writable");
    std::fs::write(store.join("broken.json"), b"{not json")
        .expect("the corrupt record is writable");

    let reads = read_measurement_collection_results(&DiskMeasurement::new(root))
        .expect("the store lists its collections");
    assert_eq!(
        reads.len(),
        3,
        "every record is attempted, including the one that fails"
    );
    assert_eq!(
        reads
            .iter()
            .map(|read| read
                .path
                .rsplit(std::path::MAIN_SEPARATOR)
                .next()
                .expect("a file name"))
            .collect::<Vec<_>>(),
        vec!["a-new.json", "broken.json", "z-old.json"],
        "one result per record, in the store's own order"
    );

    let broken = reads
        .iter()
        .find(|read| read.path.ends_with("broken.json"))
        .expect("the corrupt record has a result of its own");
    let Err(error) = &broken.collection else {
        panic!("the corrupt record's result must be a refusal, not a collection");
    };
    assert!(
        !error.to_string().is_empty(),
        "the refusal must say something a person can act on"
    );

    assert_eq!(
        reads
            .iter()
            .filter_map(|read| read.collection.as_ref().ok())
            .map(|collection| collection.collection_id.as_str())
            .collect::<Vec<_>>(),
        vec!["a-new", "z-old"],
        "the corrupt record hides neither sibling"
    );
    assert!(
        reads
            .iter()
            .all(|read| read.collection.is_ok() != read.collection.is_err()),
        "each result is a collection or a refusal, never both and never neither"
    );
}
