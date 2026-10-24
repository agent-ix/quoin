// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Acceptance-shaped current-main intake measurement for PLAT-1162.

use ix_trace_rs::trace;
use quoin_measurement::plans::{PlanLoadOptions, load_measurement_plans};
use quoin_measurement::source::MemoryMeasurement;

#[test]
#[trace("TC-1810", "FR-110-AC-1")]
fn evidence_gate_intake_boundary() {
    let mut discrepancies = Vec::new();
    for (name, extra, expected) in [
        ("evidence-only gate", "", true),
        (
            "procedure without apparatus",
            "execution_procedure: harness/run.json\n",
            false,
        ),
        (
            "apparatus-edit without apparatus",
            "negative_controls:\n  - kind: apparatus-edit\n    description: Changed answer key must refuse\n",
            false,
        ),
        ("empty apparatus", "protected_apparatus: []\n", false),
        ("blank apparatus", "protected_apparatus: ['']\n", false),
        (
            "declared apparatus",
            "protected_apparatus: [harness/answers.json]\n",
            true,
        ),
    ] {
        let controls = if name == "apparatus-edit without apparatus" {
            ""
        } else {
            "negative_controls:\n  - kind: suppressed-observation\n    description: Omitted failing observation prevents a pass\n"
        };
        let document = format!(
            "---\nid: MP-900\ntitle: Retained evidence gate\ntype: MeasurementPlan\nstatus: active\nowner: fixture-owner\nstage: gate\nmetric: retention.rate\ndefinition_version: retention-v1\nground_truth_kind: mechanical\nstatistical_design:\n  population: all retained items\n  sampling: exhaustive\n  repetitions: 3\n  estimator: proportion\n  error_model: exact counts\n  uncertainty: complete census\n  decision_rule:\n    comparator: ge\n    threshold: 1\n{controls}{extra}---\n# Retained evidence gate\n"
        );
        let source = MemoryMeasurement::new().with_document("spec/assurance/MP-900.md", document);
        let result = load_measurement_plans(&source, PlanLoadOptions::default());
        println!("{name}: expected_accept={expected}; actual={result:?}");
        if result.is_ok() != expected {
            discrepancies.push(name);
        }
    }
    assert!(
        discrepancies.is_empty(),
        "acceptance discrepancies: {discrepancies:?}"
    );
}
