// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The observation a gate verdict (MP-229, MP-230) is recorded as.
//!
//! A gate plan combines bars that are plans of their own. The producer is the
//! run that computed each bar: it states every bar it evaluated as an examined
//! id and every bar that held as a matched id, so the observation is the
//! verdict and its population at once. The ids come from the bars the run
//! computed, not from the plan's declaration; the plan's `population_set`
//! then decides whether the run evaluated exactly its bars.

use std::path::Path;

use quoin_measurement::json_bridge::from_serde;
use quoin_measurement::plans::{PlanLoadOptions, load_measurement_plans};
use quoin_measurement::source::MemoryMeasurement;
use quoin_measurement::types::plan::MeasurementPlan;
use quoin_measurement::validate;
use serde_json::{Value, json};

/// One bar of a gate: the plan that defines it and whether the run met it.
pub(crate) struct Bar {
    pub(crate) plan_id: &'static str,
    pub(crate) met: bool,
}

/// The authored gate plan `id`, loaded from `spec/assurance`.
pub(crate) fn plan(id: &str) -> MeasurementPlan {
    let assurance = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../spec/assurance");
    let file = std::fs::read_dir(&assurance)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .find(|name| name.starts_with(&format!("{id}-")))
        .unwrap_or_else(|| panic!("{id} is not a plan in spec/assurance"));
    let document = std::fs::read_to_string(assurance.join(&file)).unwrap();
    let source = MemoryMeasurement::new().with_document(format!("spec/assurance/{file}"), document);
    load_measurement_plans(&source, PlanLoadOptions::default())
        .unwrap()
        .remove(0)
}

/// The candidate collection of one gate run: a single observation whose value
/// is the share of bars met and whose population names the bars.
pub(crate) fn collection(plan: &MeasurementPlan, variant: &str, bars: &[Bar]) -> Value {
    let examined: Vec<&str> = bars.iter().map(|bar| bar.plan_id).collect();
    let matched: Vec<&str> = bars
        .iter()
        .filter(|bar| bar.met)
        .map(|bar| bar.plan_id)
        .collect();
    let value = matched.len() as f64 / examined.len() as f64;
    json!({
        "schemaVersion": 2,
        "collectionId": format!("{}-{variant}", plan.id.as_str()),
        "subject": "quoin",
        "scope": { "ids": examined },
        "toolIdentity": "quoin-jev gate run",
        "toolVersion": env!("CARGO_PKG_VERSION"),
        "timestamp": "2026-11-11T00:00:00.000Z",
        "environment": { "variant": variant },
        "verificationStack": {
            "schemaVersion": "verification-stack-attestation-v1",
            "buildProfile": "debug",
            "capabilities": ["jev.gate"],
            "artifacts": [plan.path],
        },
        "observations": [{
            "metric": plan.metric.as_str(),
            "planId": plan.id.as_str(),
            "definitionVersion": plan.definition_version.as_str(),
            "state": "measured",
            "value": value,
            "unit": "fraction",
            "shape": "ratio",
            "population": {
                "examined": examined.len(),
                "matched": matched.len(),
                "complete": true,
                "repetitions": 1,
                "examined_ids": examined,
                "matched_ids": matched,
            },
        }],
        "rawEvidence": { "bars": bars.iter().map(|bar| json!({"plan": bar.plan_id, "met": bar.met})).collect::<Vec<_>>() },
    })
}

/// Admit `collection` against `plan` as intake does.
pub(crate) fn admit(
    plan: &MeasurementPlan,
    collection: &Value,
) -> Result<(), quoin_measurement::error::MeasurementError> {
    validate::measurement_collection(&from_serde(collection).unwrap(), std::slice::from_ref(plan))
        .map(|_| ())
}

/// Record a live gate run: build the collection, require intake to admit it,
/// and keep it under `QUOIN_GATE_COLLECTION_OUT` when that names a directory.
/// Called before the run's assertions so a NO-GO is recorded too.
///
/// # Panics
///
/// When intake refuses the observation: the run did not evaluate exactly the
/// plan's bars, and a live run must not go on as though it had.
pub(crate) fn record(plan_id: &str, variant: &str, bars: &[Bar]) {
    let out = std::env::var_os("QUOIN_GATE_COLLECTION_OUT");
    if let Err(error) = record_into(plan_id, variant, bars, out.as_deref().map(Path::new)) {
        panic!("{plan_id} {variant}: intake refused the gate observation: {error}");
    }
}

/// [`record`] with the output directory given and the refusal returned. An
/// observation intake refuses is never written.
pub(crate) fn record_into(
    plan_id: &str,
    variant: &str,
    bars: &[Bar],
    out: Option<&Path>,
) -> Result<(), quoin_measurement::error::MeasurementError> {
    let plan = plan(plan_id);
    let collection = collection(&plan, variant, bars);
    admit(&plan, &collection)?;
    if let Some(directory) = out {
        std::fs::create_dir_all(directory).unwrap();
        std::fs::write(
            directory.join(format!("{plan_id}-{variant}.json")),
            serde_json::to_string_pretty(&collection).unwrap(),
        )
        .unwrap();
    }
    Ok(())
}
