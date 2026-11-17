// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; production lints stand"
)]

//! Source and engine coverage for the risk-management dossier (FR-199).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use ix_trace_rs::trace;
use quoin_dossier::{Inputs, SourceFailure};
use quoin_quire::{ModuleRoot, ModuleSelection};
use tempfile::TempDir;

fn module_root() -> PathBuf {
    std::env::var_os("SPEC_OBJECTS_SAFETY_DIR")
        .map_or_else(
            || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../spec-objects-safety"),
            PathBuf::from,
        )
        .join("spec_objects_safety")
}

fn modules() -> ModuleSelection {
    ModuleSelection::Closed(vec![ModuleRoot::open(module_root()).unwrap()])
}

fn input(scope: &Path) -> Inputs {
    Inputs {
        scope: scope.to_path_buf(),
        modules: vec![module_root()],
        clause_bindings: Vec::new(),
        pack: None,
        soup_binding: None,
        soup_records: None,
        soup_support: None,
        snapshot: None,
        release_verify: None,
        as_of: None,
        version: "1".to_owned(),
    }
}

fn failure(source: &str) -> SourceFailure {
    SourceFailure {
        source: source.to_owned(),
        detail: "unexpected source call".to_owned(),
    }
}

struct NoMatrix;

impl quoin_dossier::MatrixSource for NoMatrix {
    fn matrix(&self, _: &Inputs) -> Result<quoin_assurance::matrix::MatrixOutput, SourceFailure> {
        Err(failure("matrix"))
    }
}

fn policy_source(inputs: &Inputs) -> Result<quoin_quire::records::Corpus, SourceFailure> {
    let modules = inputs
        .modules
        .iter()
        .map(ModuleRoot::open)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| SourceFailure {
            source: "acceptability policy".to_owned(),
            detail: format!("failed to open module: {error}"),
        })?;
    quoin_quire::records::read(
        &inputs.scope,
        &ModuleSelection::Closed(modules),
        &["acceptability_policy"],
    )
    .map_err(|error| SourceFailure {
        source: "acceptability policy".to_owned(),
        detail: format!("failed to read policy records: {error}"),
    })
}

fn registry() -> quoin_dossier::Registry {
    quoin_dossier::builtin_registry(quoin_dossier::Sources {
        matrix: Arc::new(NoMatrix),
        rmm: |_| Err(failure("risk management")),
        soup: |_| Err(failure("SOUP")),
        policy: policy_source,
        threat_model: |_| panic!("unexpected threat-model source"),
    })
    .unwrap()
}

struct Fixture {
    root: TempDir,
}

impl Fixture {
    fn new() -> Self {
        Self {
            root: tempfile::tempdir().unwrap(),
        }
    }

    fn put(&self, id: &str, kind: &str, front: &str, body: &str) {
        let properties = if matches!(
            kind,
            "risk_estimate" | "acceptability_policy" | "residual_risk"
        ) {
            "\n## Properties\n\n| Field | Type | Multiplicity | Constraints |\n|---|---|---|---|\n| record_id | UUID | 1..1 | identity |\n"
        } else {
            ""
        };
        let text = format!(
            "---\nid: {id}\ntitle: \"{id}\"\ntype: {kind}\nobject: {kind}\n{front}---\n# [{id}] {id}\n\n{body}{properties}"
        );
        std::fs::write(self.root.path().join(format!("{id}.md")), text).unwrap();
    }

    fn link(target: &str, relationship: &str) -> String {
        format!("  - target: {target}\n    type: {relationship}\n")
    }

    fn complete_chain(&self) {
        self.put("HAZ-1", "hazard", "status: accepted\n", "");
        self.put(
            "SIT-1",
            "hazardous_situation",
            &format!("relationships:\n{}", Self::link("HAZ-1", "arises_from")),
            "",
        );
        self.put(
            "HARM-1",
            "harm",
            &format!("relationships:\n{}", Self::link("SIT-1", "arises_from")),
            "",
        );
        self.put("CTRL-1", "control", "", "");
        self.put(
            "FR-9",
            "FR",
            &format!("relationships:\n{}", Self::link("CTRL-1", "satisfies")),
            "| ID | Criteria |\n|---|---|\n| FR-9-AC-1 | record evidence |\n",
        );
        self.put(
            "TC-9",
            "TC",
            &format!("relationships:\n{}", Self::link("CTRL-1", "verifies")),
            "result: passed\n",
        );
        let policy_front = "";
        let policy_body = "## Severity levels\n\n| Severity level |\n| --- |\n| Minor |\n| Serious |\n\n## Probability levels\n\n| Probability level |\n| --- |\n| Rare |\n| Likely |\n\n## Regions\n\n| Region | Outside acceptable |\n| --- | --- |\n| Low | false |\n| Mid | false |\n| High | true |\n\n## Cells\n\n| Severity | Probability | Region |\n| --- | --- | --- |\n| Minor | Rare | Low |\n| Minor | Likely | Low |\n| Serious | Rare | Mid |\n| Serious | Likely | High |\n";
        self.put("POL-1", "acceptability_policy", policy_front, policy_body);
        for (id, phase, levels, assumes_control) in [
            ("EST-PRE", "pre_control", ("Serious", "Likely"), false),
            ("EST-POST", "post_control", ("Minor", "Rare"), true),
        ] {
            let mut relationships = Self::link("SIT-1", "estimates");
            if assumes_control {
                relationships.push_str(&Self::link("CTRL-1", "assumes"));
            }
            self.put(
                id,
                "risk_estimate",
                &format!("phase: {phase}\nrelationships:\n{relationships}"),
                &format!(
                    "## Judgement\n\n| Severity | Probability | Rationale |\n| --- | --- | --- |\n| {} | {} | recorded |\n\n## Provenance\n\n| Asserted by | Asserted at |\n| --- | --- |\n| Pat | 2026-01-01 |\n",
                    levels.0, levels.1
                ),
            );
        }
        self.put(
            "RR-1",
            "residual_risk",
            &format!(
                "relationships:\n{}{}",
                Self::link("EST-POST", "estimate"),
                Self::link("POL-1", "references")
            ),
            "## Determination\n\n| Decision | Rationale | Decided by |\n| --- | --- | --- |\n| accepted | recorded | Pat |\n",
        );
    }
}

#[trace("FR-199-AC-5")]
#[test]
fn rendered_policy_regions_match_both_source_reports_for_each_estimate() {
    let fixture = Fixture::new();
    fixture.complete_chain();
    let selection = modules();
    let corpus =
        quoin_quire::records::read(fixture.root.path(), &selection, &["risk_estimate"]).unwrap();
    let estimates = corpus
        .documents
        .iter()
        .filter(|document| document.kind == "risk_estimate")
        .collect::<Vec<_>>();
    assert_eq!(estimates.len(), 2);
    let summary = quoin_risk_advice::summarize(fixture.root.path(), &selection).unwrap();
    let advice = quoin_risk_advice::advise(fixture.root.path(), &selection).unwrap();
    let report = quoin_dossier::generate(
        &registry(),
        &["risk-management".to_owned()],
        &fixture.root.path().join("dossier"),
        &input(fixture.root.path()),
    )
    .unwrap();
    assert_eq!(report.documents[0].status, quoin_dossier::Status::Complete);
    let markdown =
        std::fs::read_to_string(fixture.root.path().join("dossier/DOS-RISK.md")).unwrap();
    for estimate in summary
        .rows
        .iter()
        .flat_map(|row| row.pre_control.iter().chain(&row.post_control))
    {
        let advice_cell = advice
            .policy_cells
            .iter()
            .find(|cell| cell.estimate == estimate.estimate)
            .unwrap();
        assert_eq!(estimate.policy_region, advice_cell.region);
        let rendered = format!(
            "| {} | {} | {} | {} | {} |",
            estimate.estimate,
            advice_cell.phase,
            estimate.severity,
            estimate.probability,
            advice_cell.region.as_deref().unwrap_or("not stated")
        );
        assert!(
            markdown.contains(&rendered),
            "missing source rendering {rendered}"
        );
    }
    assert_eq!(
        summary
            .rows
            .iter()
            .map(|row| row.pre_control.len() + row.post_control.len())
            .sum::<usize>(),
        2
    );
}

#[trace("FR-199-AC-8")]
#[test]
fn unreadable_safety_record_fails_generation_without_writing_selected_document() {
    let scope = tempfile::tempdir().unwrap();
    std::fs::write(scope.path().join("RR-9.md"), [0xff, 0xfe, 0x00, 0x41]).unwrap();
    let out = scope.path().join("dossier");
    let error = quoin_dossier::generate(
        &registry(),
        &["risk-management".to_owned()],
        &out,
        &input(scope.path()),
    )
    .unwrap_err();
    assert_eq!(error.outcome().code(), 3);
    let source = error.source_failure().unwrap();
    assert_eq!(source.source, "risk records");
    assert!(source.detail.contains("RR-9.md"), "{}", source.detail);
    assert!(!out.join("DOS-RISK.md").exists());
    assert!(!out.exists());
}
