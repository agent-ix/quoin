// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! EA-26: an observation's optional `interval` at intake (FR-044-AC-10,
//! FR-044-AC-11), and the report views that carry it (FR-044-AC-12).
//!
//! Plans are real assurance documents loaded through `load_measurement_plans`,
//! so the `decision_rule.interval_level` parse is exercised along with the
//! check. Quoin computes no interval: every test that expects a refusal for an
//! absent one also asserts that nothing derived one from `population`.
//!
//! Provenance: EA-26

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
use quoin_measurement::report::{
    build_measurement_report, render_measurement_report_json, render_series_json, series_for,
};
use quoin_measurement::source::{DiskMeasurement, MemoryMeasurement};
use quoin_measurement::store::write_measurement_collection;
use quoin_measurement::types::plan::MeasurementPlan;
use quoin_measurement::validate;
use serde_json::{Value, json};

/// An active gate plan for `quality.gate`, its `decision_rule` carrying
/// `rule_extra` (each line already indented as YAML needs).
fn plan_document(rule_extra: &str, estimator: &str) -> String {
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
         protected_apparatus:\n\
         \x20\x20- spec/assurance/MP-900.md\n\
         negative_controls:\n\
         \x20\x20- kind: apparatus-edit\n\
         \x20\x20\x20\x20description: the plan document is digested with every collection\n\
         ground_truth_kind: human-labelled\n\
         statistical_design:\n\
         \x20\x20population: every labelled case\n\
         \x20\x20sampling: exhaustive\n\
         \x20\x20repetitions: 1\n\
         \x20\x20estimator: {estimator}\n\
         \x20\x20error_model: binomial\n\
         \x20\x20uncertainty: stated by the producer\n\
         \x20\x20decision_rule:\n\
         \x20\x20\x20\x20comparator: ge\n\
         \x20\x20\x20\x20threshold: 0.5\n\
         {rule_extra}\
         ---\n\
         \n\
         # Example gate\n"
    )
}

/// A plan whose rule states `interval_level: 0.95`.
fn level_plan() -> String {
    plan_document("\x20\x20\x20\x20interval_level: 0.95\n", "proportion")
}

fn repository_with(document: &str) -> tempfile::TempDir {
    let temporary = tempfile::tempdir().expect("a temporary directory");
    let assurance = temporary.path().join("spec").join("assurance");
    std::fs::create_dir_all(&assurance).expect("the assurance root is creatable");
    std::fs::write(assurance.join("MP-900.md"), document).expect("the plan is writable");
    temporary
}

fn authored_plans(root: &Path) -> Vec<MeasurementPlan> {
    load_measurement_plans(&DiskMeasurement::new(root), PlanLoadOptions::default())
        .expect("the authored plan loads")
}

/// A well-formed interval around `0.9`.
fn interval() -> Value {
    json!({ "lower": 0.7, "upper": 0.99, "level": 0.95, "method": "wilson" })
}

/// One measured observation of `quality.gate`, stating `interval` when given.
fn observation(interval: Option<Value>) -> Value {
    let mut observation = json!({
        "metric": "quality.gate",
        "planId": "MP-900",
        "definitionVersion": "quality.gate-v1",
        "state": "measured",
        "value": 0.9,
        "unit": "fraction",
        "shape": "ratio",
        "population": { "examined": 10, "matched": 9 },
    });
    if let Some(interval) = interval {
        observation["interval"] = interval;
    }
    observation
}

fn collection(observations: &[Value]) -> Value {
    json!({
        "schemaVersion": 2,
        "collectionId": "run-1948",
        "subject": "fixture",
        "scope": { "cases": 1 },
        "toolIdentity": "fixture producer",
        "toolVersion": "fixture 1 (engine a1)",
        "configDigest": format!("sha256:{}", "a".repeat(64)),
        "timestamp": "2026-09-22T00:00:00.000Z",
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
        "observations": observations,
        "rawEvidence": { "payload": [1, 2] },
    })
}

fn intake(
    plans: &[MeasurementPlan],
    candidate: &Value,
) -> Result<String, quoin_measurement::MeasurementError> {
    validate::measurement_collection(
        &from_serde(candidate).expect("the candidate crosses the bridge"),
        plans,
    )
    .map(|admitted| admitted.collection_id.as_str().to_owned())
}

/// Write `candidate` into `root`, declaring the protected plan document at
/// its digest as `verificationStack.artifacts` must (PLAT-975).
fn write_collection(
    root: &Path,
    candidate: &Value,
) -> Result<std::path::PathBuf, quoin_measurement::MeasurementError> {
    let path = "spec/assurance/MP-900.md";
    let mut candidate = candidate.clone();
    candidate["verificationStack"]["artifacts"][path] = json!(
        quoin_store::digest_file_sha256(&root.join(path))
            .expect("the plan is digestible")
            .to_stored()
    );
    write_measurement_collection(
        root,
        &from_serde(&candidate).expect("the candidate crosses"),
    )
}

/// Every way an `interval` can fail FR-044-AC-10, with the fragment of the
/// finding that names why. Each is refused whatever the plan declares.
fn malformed_intervals() -> Vec<(&'static str, Value)> {
    vec![
        ("not an object", json!("wide")),
        ("an array", json!([0.7, 0.99])),
        ("a stated null", Value::Null),
        (
            "a missing member",
            json!({ "lower": 0.7, "upper": 0.99, "level": 0.95 }),
        ),
        (
            "an extra member",
            json!({ "lower": 0.7, "upper": 0.99, "level": 0.95, "method": "wilson", "standardError": 0.05 }),
        ),
        (
            "a non-numeric bound",
            json!({ "lower": "0.7", "upper": 0.99, "level": 0.95, "method": "wilson" }),
        ),
        (
            "a non-numeric level",
            json!({ "lower": 0.7, "upper": 0.99, "level": "0.95", "method": "wilson" }),
        ),
        (
            "lower above value",
            json!({ "lower": 0.95, "upper": 0.99, "level": 0.95, "method": "wilson" }),
        ),
        (
            "value above upper",
            json!({ "lower": 0.7, "upper": 0.8, "level": 0.95, "method": "wilson" }),
        ),
        (
            "a level of zero",
            json!({ "lower": 0.7, "upper": 0.99, "level": 0, "method": "wilson" }),
        ),
        (
            "a level of one",
            json!({ "lower": 0.7, "upper": 0.99, "level": 1, "method": "wilson" }),
        ),
        (
            "a whitespace-only method",
            json!({ "lower": 0.7, "upper": 0.99, "level": 0.95, "method": "  " }),
        ),
    ]
}

/// A malformed `interval` is `QM-INTERVAL-MALFORMED` naming the metric, the
/// member and the stated value, under a plan with `interval_level` and one
/// without; a well-formed one, `lower = value = upper` included, is admitted.
///
/// Trace: FR-044-AC-10, TC-1948
/// Provenance: EA-26
#[test]
fn tc_1948_a_malformed_interval_is_refused_and_a_well_formed_one_admitted() {
    for document in [level_plan(), plan_document("", "proportion")] {
        let repository = repository_with(&document);
        let plans = authored_plans(repository.path());
        for (why, stated) in malformed_intervals() {
            let refusal =
                intake(&plans, &collection(&[observation(Some(stated.clone()))])).expect_err(why);
            assert_eq!(
                refusal.code(),
                MeasurementErrorCode::IntervalMalformed,
                "{why}"
            );
            let [finding] = refusal.findings() else {
                panic!(
                    "{why}: exactly one finding, malformed only: {:?}",
                    refusal.findings()
                );
            };
            assert!(
                finding
                    .starts_with("QM-INTERVAL-MALFORMED: metric `quality.gate` states interval "),
                "{why}: {finding}"
            );
            assert!(
                finding.contains(&format!("states interval {};", canonical(&stated))),
                "{why}: the finding names the stated value: {finding}"
            );
        }
        // The literal for the one quoin states itself.
        let refusal = intake(
            &plans,
            &collection(&[observation(Some(
                json!({ "lower": 0.95, "upper": 0.99, "level": 0.95, "method": "wilson" }),
            ))]),
        )
        .expect_err("value below lower");
        assert_eq!(
            refusal.findings(),
            [
                "QM-INTERVAL-MALFORMED: metric `quality.gate` states interval \
                 {\"level\":0.95,\"lower\":0.95,\"method\":\"wilson\",\"upper\":0.99}; \
                 value 0.9 lies outside [0.95, 0.99]"
            ]
        );

        for admitted in [
            interval(),
            json!({ "lower": 0.9, "upper": 0.9, "level": 0.95, "method": "exact" }),
            json!({ "lower": 0.9, "upper": 0.99, "level": 0.99, "method": "exact" }),
        ] {
            assert_eq!(
                intake(&plans, &collection(&[observation(Some(admitted.clone()))]))
                    .unwrap_or_else(|refusal| panic!("{admitted} was refused: {refusal}")),
                "run-1948"
            );
        }
    }
}

/// An `interval` on a `not_computed` observation is refused as malformed; a
/// collection stating none is admitted and stored exactly as before, and one
/// stating an interval is stored with it, `schemaVersion` unchanged.
///
/// Trace: FR-044-AC-10, TC-1949
/// Provenance: EA-26
#[test]
fn tc_1949_not_computed_forbids_an_interval_and_stored_bytes_are_preserved() {
    let repository = repository_with(&plan_document("", "proportion"));
    let plans = authored_plans(repository.path());

    let mut not_computed = observation(Some(interval()));
    not_computed["state"] = json!("not_computed");
    not_computed["value"] = Value::Null;
    not_computed["reason"] = json!("producer crashed");
    let refusal = intake(&plans, &collection(&[not_computed.clone()]))
        .expect_err("an interval on a not_computed observation is refused");
    assert_eq!(refusal.code(), MeasurementErrorCode::IntervalMalformed);
    assert!(
        refusal.findings()[0].contains("not allowed on a not_computed observation"),
        "{:?}",
        refusal.findings()
    );
    // The same observation with no interval is admitted.
    not_computed.as_object_mut().unwrap().remove("interval");
    assert_eq!(
        intake(&plans, &collection(&[not_computed])).expect("not_computed with none is admitted"),
        "run-1948"
    );

    let root = repository.path();
    let plain = collection(&[observation(None)]);
    let path = write_collection(root, &plain).expect("no interval is admitted");
    let stored: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert!(
        stored["observations"][0].get("interval").is_none(),
        "nothing added an interval: {stored}"
    );
    assert_eq!(stored["schemaVersion"], json!(2));

    let stated = collection(&[observation(Some(interval()))]);
    let mut second = stated.clone();
    second["collectionId"] = json!("run-1948-with-interval");
    let path = write_collection(root, &second).expect("a well-formed interval is admitted");
    let stored: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(stored["observations"][0]["interval"], interval());
    assert_eq!(stored["schemaVersion"], json!(2));
}

/// Under a plan whose rule states `interval_level`, a `measured` observation
/// carrying `matched` and `examined` but no `interval` is refused as
/// `QM-INTERVAL-UNSTATED` naming the metric, the plan and the required level,
/// and no interval is ever computed from the population; a `not_computed`
/// observation and a constant-predictor item row are not held to it; under a
/// plan with no `interval_level` an absent interval is admitted.
///
/// Trace: FR-044-AC-11, TC-1950
/// Provenance: EA-26
#[test]
fn tc_1950_an_absent_interval_is_refused_only_under_an_interval_level() {
    let repository = repository_with(&level_plan());
    let plans = authored_plans(repository.path());

    let refusal = intake(&plans, &collection(&[observation(None)]))
        .expect_err("a proportion with matched and examined but no interval is refused");
    assert_eq!(refusal.code(), MeasurementErrorCode::IntervalUnstated);
    assert_eq!(
        refusal.findings(),
        [
            "QM-INTERVAL-UNSTATED: metric `quality.gate` states no interval; plan MP-900 \
             requires an interval at level 0.95 or above"
        ]
    );
    // `count` is no different: nothing is derived from the population.
    let count = repository_with(&plan_document(
        "\x20\x20\x20\x20interval_level: 0.9\n",
        "count",
    ));
    let refusal = intake(
        &authored_plans(count.path()),
        &collection(&[observation(None)]),
    )
    .expect_err("a count with no interval is refused");
    assert_eq!(refusal.code(), MeasurementErrorCode::IntervalUnstated);

    let mut not_computed = observation(None);
    not_computed["state"] = json!("not_computed");
    not_computed["value"] = Value::Null;
    not_computed["reason"] = json!("producer crashed");
    assert_eq!(
        intake(&plans, &collection(&[not_computed])).expect("not_computed is not held to it"),
        "run-1948"
    );

    // A per-item row answers to its governed metric's plan but is one graded
    // item, not a population.
    let mut item = observation(None);
    item["metric"] = json!("quality.gate.constant-predictor-item");
    item["dimensions"] = json!({ "family": "f", "item_id": "i1", "expected": "x" });
    assert_eq!(
        intake(
            &plans,
            &collection(&[observation(Some(interval())), item.clone()])
        )
        .expect("an item row states no interval"),
        "run-1948"
    );
    // ... but is still refused when the interval it does state is malformed.
    item["interval"] = json!("wide");
    let refusal = intake(&plans, &collection(&[observation(Some(interval())), item]))
        .expect_err("a malformed interval is refused on an item row too");
    assert_eq!(refusal.code(), MeasurementErrorCode::IntervalMalformed);

    let unlevelled = repository_with(&plan_document("", "proportion"));
    assert_eq!(
        intake(
            &authored_plans(unlevelled.path()),
            &collection(&[observation(None)])
        )
        .expect("no interval_level means an absent interval is admitted"),
        "run-1948"
    );
}

/// Under a plan declaring `interval_level` 0.95, an interval at 0.9 is
/// refused as `QM-INTERVAL-LEVEL-SHORT` naming both levels, one at 0.95 or
/// 0.99 is admitted, and a malformed interval raises the malformed code only.
///
/// Trace: FR-044-AC-11, TC-1951
/// Provenance: EA-26
#[test]
fn tc_1951_a_short_interval_level_is_refused() {
    let repository = repository_with(&level_plan());
    let plans = authored_plans(repository.path());
    let at =
        |level: f64| json!({ "lower": 0.7, "upper": 0.99, "level": level, "method": "wilson" });

    let refusal = intake(&plans, &collection(&[observation(Some(at(0.9)))]))
        .expect_err("level 0.9 is below 0.95");
    assert_eq!(refusal.code(), MeasurementErrorCode::IntervalLevelShort);
    assert_eq!(
        refusal.findings(),
        [
            "QM-INTERVAL-LEVEL-SHORT: metric `quality.gate` states an interval at level 0.9 \
             but plan MP-900 requires 0.95"
        ]
    );
    for level in [0.95, 0.99] {
        assert_eq!(
            intake(&plans, &collection(&[observation(Some(at(level)))]))
                .unwrap_or_else(|refusal| panic!("level {level} was refused: {refusal}")),
            "run-1948"
        );
    }
    // Malformed AND short (a level of 0 is not a level): malformed only.
    let refusal =
        intake(&plans, &collection(&[observation(Some(at(0.0)))])).expect_err("a malformed level");
    assert_eq!(refusal.code(), MeasurementErrorCode::IntervalMalformed);
    assert_eq!(refusal.findings().len(), 1);
}

/// Interval findings accumulate with population, unplanned-metric and each
/// other's findings: the refusal carries the interval code when every finding
/// shares it and `QM-COLLECTION-INVALID` otherwise, each interval finding
/// leading with its own code.
///
/// Trace: FR-044-AC-10, FR-044-AC-11, TC-1952
/// Provenance: EA-26
#[test]
fn tc_1952_interval_findings_accumulate_with_every_other_finding() {
    let repository = repository_with(&level_plan());
    let plans = authored_plans(repository.path());
    let variant = |name: &str, interval: Option<Value>| {
        let mut observation = observation(interval);
        observation["dimensions"] = json!({ "variant": name });
        observation
    };

    // Two unstated: one shared code.
    let refusal = intake(
        &plans,
        &collection(&[variant("a", None), variant("b", None)]),
    )
    .expect_err("two unstated intervals");
    assert_eq!(refusal.code(), MeasurementErrorCode::IntervalUnstated);
    assert_eq!(refusal.findings().len(), 2);

    // Mixed interval codes, plus an unplanned metric, refuse as invalid with
    // each interval finding still leading with its own code.
    let mut unplanned = observation(Some(interval()));
    unplanned["metric"] = json!("quality.unplanned");
    let refusal = intake(
        &plans,
        &collection(&[
            variant("a", None),
            variant("b", Some(json!("wide"))),
            variant(
                "c",
                Some(json!({ "lower": 0.7, "upper": 0.99, "level": 0.9, "method": "wilson" })),
            ),
            unplanned,
        ]),
    )
    .expect_err("three interval defects and an unplanned metric");
    assert_eq!(refusal.code(), MeasurementErrorCode::CollectionInvalid);
    let findings = refusal.findings();
    assert_eq!(findings.len(), 4, "{findings:?}");
    assert!(findings[0].starts_with("QM-INTERVAL-UNSTATED: metric `quality.gate [variant=a]`"));
    assert!(findings[1].starts_with("QM-INTERVAL-MALFORMED: metric `quality.gate [variant=b]`"));
    assert!(findings[2].starts_with("QM-INTERVAL-LEVEL-SHORT: metric `quality.gate [variant=c]`"));
    assert_eq!(
        findings[3],
        "metric `quality.unplanned` has no MeasurementPlan under spec/assurance or assurance; \
         record refused"
    );
}

/// The JSON report and the series view carry a stored `interval` exactly as
/// stored, a malformed retained one included, and omit it when absent, so
/// every view over records without `interval` is unchanged (the existing
/// `tc_473_reporting` goldens stay byte-identical).
///
/// Trace: FR-044-AC-12, TC-1953
/// Provenance: EA-26
#[test]
fn tc_1953_report_and_series_carry_the_stored_interval_exactly() {
    let stated = json!({ "lower": 0.7, "upper": 0.99, "level": 0.95, "method": "wilson" });
    let malformed = json!({ "lower": "low", "extra": [1, 2] });
    // A retained collection is read leniently: a malformed interval is still
    // shown as stored. Distinct timestamps fix the series order.
    let retained = |id: &str, timestamp: &str, interval: Option<Value>| {
        let mut candidate = collection(&[observation(interval)]);
        candidate["collectionId"] = json!(id);
        candidate["timestamp"] = json!(timestamp);
        serde_json::to_vec(&candidate).unwrap()
    };
    let source_of = |collections: Vec<(&str, Vec<u8>)>| {
        collections.into_iter().fold(
            MemoryMeasurement::new()
                .with_document("spec/assurance/MP-900.md", plan_document("", "proportion")),
            |source, (name, bytes)| source.with_collection(name, bytes),
        )
    };
    let repo = tempfile::tempdir().unwrap();

    let source = source_of(vec![
        (
            "a.json",
            retained("run-a", "2026-09-01T00:00:00Z", Some(stated.clone())),
        ),
        (
            "b.json",
            retained("run-b", "2026-09-02T00:00:00Z", Some(malformed.clone())),
        ),
        ("c.json", retained("run-c", "2026-09-03T00:00:00Z", None)),
    ]);
    let series: Value = serde_json::from_str(
        &render_series_json(&series_for(&source, "quality.gate").expect("the series reads"))
            .expect("the series renders"),
    )
    .unwrap();
    let observations: Vec<&Value> = series
        .as_array()
        .unwrap()
        .iter()
        .map(|point| &point["observation"])
        .collect();
    assert_eq!(observations.len(), 3);
    assert_eq!(observations[0]["interval"], stated);
    assert_eq!(observations[1]["interval"], malformed);
    assert!(
        observations[2].get("interval").is_none(),
        "an observation that states none carries none: {}",
        observations[2]
    );

    // The report's current row shows the newest observation of each slice.
    for (interval, ordinal) in [(Some(&stated), "a"), (Some(&malformed), "b")] {
        let one = source_of(vec![(
            "only.json",
            retained(
                &format!("run-{ordinal}"),
                "2026-09-01T00:00:00Z",
                interval.cloned(),
            ),
        )]);
        let report: Value = serde_json::from_str(
            &render_measurement_report_json(
                &build_measurement_report(&one, repo.path()).expect("the report builds"),
            )
            .expect("the JSON report renders"),
        )
        .unwrap();
        assert_eq!(
            report["current"][0]["observation"]["interval"],
            *interval.unwrap()
        );
    }
    let plain = source_of(vec![(
        "only.json",
        retained("run-c", "2026-09-01T00:00:00Z", None),
    )]);
    let plain_report = render_measurement_report_json(
        &build_measurement_report(&plain, repo.path()).expect("the report builds"),
    )
    .unwrap();
    assert!(!plain_report.contains("interval"), "{plain_report}");
    let plain_series = render_series_json(&series_for(&plain, "quality.gate").unwrap()).unwrap();
    assert!(!plain_series.contains("interval"), "{plain_series}");
}

/// `value` in the store's canonical spelling (sorted keys),
/// which is how a finding names a stored value; `Value`'s own `Display` keeps
/// insertion order whenever another crate in the build enables `serde_json`'s
/// `preserve_order`.
fn canonical(value: &Value) -> String {
    let bytes = quoin_store::canonical_json_bytes(&from_serde(value).unwrap()).unwrap();
    serde_json::from_slice::<Value>(&bytes).unwrap().to_string()
}
