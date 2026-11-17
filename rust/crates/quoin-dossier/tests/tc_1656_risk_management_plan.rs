// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Typed-reader and output-transaction coverage for FR-200's policy source.

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
    quoin_quire::records::read(&inputs.scope, &selection, &["acceptability_policy"]).map_err(
        |error| SourceFailure {
            source: "acceptability policy".to_owned(),
            detail: format!("failed to read policy records: {error}"),
        },
    )
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

#[trace("FR-200-AC-4")]
#[test]
fn typed_reader_refuses_a_policy_without_a_region_flag_before_output_is_written() {
    let module = module_root();
    assert!(
        module.join("manifest.yaml").is_file(),
        "missing safety module at {}",
        module.display()
    );
    let scope = tempfile::tempdir().unwrap();
    let out = scope.path().join("dossier");
    let records = scope.path().join("spec");
    fs::create_dir_all(&records).unwrap();
    fs::write(
        records.join("policy.md"),
        "---\nid: POL-INVALID\ntitle: Invalid policy\ntype: acceptability_policy\nobject: acceptability_policy\n---\n\
         # Invalid policy\n\n\
         ## Severity levels\n\n| Severity level |\n| --- |\n| serious |\n\n\
         ## Probability levels\n\n| Probability level |\n| --- |\n| likely |\n\n\
         ## Regions\n\n| Region | Outside acceptable |\n| --- | --- |\n| unacceptable | |\n\n\
         ## Cells\n\n| Severity | Probability | Region |\n| --- | --- | --- |\n| serious | likely | unacceptable |\n\n\
         ## Properties\n\n| Field | Type | Multiplicity | Constraints |\n| --- | --- | --- | --- |\n| record_id | UUID | 1..1 | identity |\n",
    )
    .unwrap();

    let error = quoin_dossier::generate(
        &registry(),
        &["risk-management-plan".to_owned()],
        &out,
        &input(scope.path()),
    )
    .unwrap_err();

    assert_eq!(error.outcome().code(), 3);
    assert_eq!(
        error.source_failure().unwrap().source,
        "acceptability policy"
    );
    assert!(!out.join("DOS-RMP.md").exists());
    assert!(!out.exists());
}
