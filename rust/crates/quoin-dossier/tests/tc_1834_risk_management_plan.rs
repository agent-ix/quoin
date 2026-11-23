// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Typed-reader coverage for FR-200's plan record: the safety module's own
//! `risk_management_plan` type, read through the same reader as the policy.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ix_trace_rs::trace;
use quoin_dossier::{Inputs, SourceFailure};

fn policy_source(inputs: &Inputs) -> Result<quoin_quire::records::Corpus, SourceFailure> {
    let modules = inputs
        .modules
        .iter()
        .map(quoin_quire::ids::ModuleRoot::open)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| SourceFailure {
            source: "acceptability policy".to_owned(),
            detail: format!("failed to open module: {error}"),
        })?;
    let selection = quoin_quire::ModuleSelection::Closed(modules);
    quoin_quire::records::read(
        &inputs.scope,
        &selection,
        &["acceptability_policy", "risk_management_plan"],
    )
    .map_err(|error| SourceFailure {
        source: "acceptability policy".to_owned(),
        detail: format!("failed to read policy and plan records: {error}"),
    })
}

fn module_root() -> PathBuf {
    std::env::var_os("SPEC_OBJECTS_SAFETY_DIR")
        .map_or_else(
            || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../spec-objects-safety"),
            PathBuf::from,
        )
        .join("spec_objects_safety")
}

fn registry() -> quoin_dossier::Registry {
    quoin_dossier::builtin_registry(quoin_dossier::Sources {
        matrix: Arc::new(NoMatrix),
        rmm: |_| Err(unexpected("risk management")),
        soup: |_| Err(unexpected("SOUP")),
        policy: policy_source,
        threat_model: |_| panic!("unexpected threat-model source"),
        records_of_processing: |_| panic!("unexpected records-of-processing source"),
    })
    .unwrap()
}

fn unexpected(source: &str) -> SourceFailure {
    SourceFailure {
        source: source.to_owned(),
        detail: "unexpected source call".to_owned(),
    }
}

struct NoMatrix;

impl quoin_dossier::MatrixSource for NoMatrix {
    fn matrix(&self, _: &Inputs) -> Result<quoin_assurance::matrix::MatrixOutput, SourceFailure> {
        Err(unexpected("matrix"))
    }
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

const POLICY: &str = "---\nid: POL-1\ntitle: Policy\ntype: acceptability_policy\nobject: acceptability_policy\n---\n\
    # Policy\n\n\
    ## Severity levels\n\n| Severity level |\n| --- |\n| serious |\n\n\
    ## Probability levels\n\n| Probability level |\n| --- |\n| likely |\n\n\
    ## Regions\n\n| Region | Outside acceptable |\n| --- | --- |\n| unacceptable | true |\n\n\
    ## Cells\n\n| Severity | Probability | Region |\n| --- | --- | --- |\n| serious | likely | unacceptable |\n\n\
    ## Properties\n\n| Field | Type | Multiplicity | Constraints |\n| --- | --- | --- | --- |\n| record_id | UUID | 1..1 | identity |\n";

/// The module's own plan skeleton with every placeholder filled by a person,
/// referencing `target`.
fn plan_document(target: &str) -> String {
    let skeleton =
        fs::read_to_string(module_root().join("skeletons/risk_management_plan.md")).unwrap();
    let filled = [
        (
            "<what the plan covers>",
            "Brake controller, whole lifecycle.",
        ),
        ("<responsibility>", "Keep the risk file current."),
        ("<person or role>", "Safety lead"),
        (
            "<planned verification method>",
            "Test against the control requirements.",
        ),
        ("<the controls it verifies>", "Every risk control."),
        (
            "<production or post-production activity>",
            "Review field reports.",
        ),
        (
            "<how it is carried out>",
            "Quarterly review by the safety lead.",
        ),
    ]
    .iter()
    .fold(skeleton, |text, (placeholder, value)| {
        text.replace(placeholder, value)
    });
    filled.replacen(
        "\n---\n<!--",
        &format!("\nrelationships:\n  - target: {target}\n    type: references\n---\n<!--"),
        1,
    )
}

fn generate_with(plan: &str) -> (quoin_dossier::Report, String) {
    let scope = tempfile::tempdir().unwrap();
    let out = scope.path().join("dossier");
    let records = scope.path().join("spec");
    fs::create_dir_all(&records).unwrap();
    fs::write(records.join("policy.md"), POLICY).unwrap();
    fs::write(records.join("plan.md"), plan).unwrap();
    let report = quoin_dossier::generate(
        &registry(),
        &["risk-management-plan".to_owned()],
        &out,
        &input(scope.path()),
    )
    .unwrap();
    let markdown = fs::read_to_string(out.join("DOS-RMP.md")).unwrap();
    (report, markdown)
}

#[trace("FR-200-AC-7", "FR-200-AC-8")]
#[test]
fn a_plan_authored_from_the_module_skeleton_renders_and_references_its_policy() {
    let (report, markdown) = generate_with(&plan_document("POL-1"));
    assert!(
        report.documents[0].gaps.is_empty(),
        "{:?}",
        report.documents[0].gaps
    );
    assert!(markdown.contains("### RMP-001"));
    assert!(markdown.contains("| Keep the risk file current. | Safety lead |"));
    assert!(markdown.contains("Plan RMP-001 references the acceptability criteria POL-1."));
    assert!(!markdown.contains("No authored plan record exists."));
}

#[trace("FR-200-AC-9")]
#[test]
fn a_plan_that_names_no_policy_in_scope_is_reported_with_the_plan_as_subject() {
    let (report, _) = generate_with(&plan_document("POL-404"));
    let gaps = &report.documents[0].gaps;
    assert_eq!(gaps.len(), 1);
    assert_eq!(gaps[0].subject, "RMP-001");
}
