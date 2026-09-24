// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Origin replay negative controls.

#![allow(
    clippy::expect_used,
    reason = "fixtures must fail on malformed EA records"
)]

use engineering_assurance::campaign::{
    CampaignAttempt, CampaignMember, MeasurementProcedure, canonical_digest,
};
use engineering_assurance::producer_execution::{ContentDigest, InputBinding};
use serde_json::json;

use crate::campaign::input_origin::{
    INPUT_ORIGINS_SCHEMA, InputByteIdentity, InputOriginBinding, InputOriginClaim,
    InputOriginInventory, OriginDeclaration, OriginSource,
};
use crate::campaign::store::{retain_bytes, retain_value};

use super::{BTreeMap, EvidenceError, check_input_origins};

fn procedure() -> MeasurementProcedure {
    serde_json::from_value(json!({
        "schemaVersion":"engineering-assurance.measurement-procedure/v1",
        "producerName":"fictional-tool", "producerVersion":"1",
        "sourceRepository":"fictional/source",
        "responseProtocol":"quoin.process-evidence/v1",
        "responseAdapter":"quoin.process-evidence-adapter",
        "responseAdapterVersion":"1", "repetitions":1, "timeoutMillis":1000
    }))
    .expect("typed procedure")
}

fn declared_procedure() -> MeasurementProcedure {
    serde_json::from_value(json!({
        "schemaVersion":"engineering-assurance.measurement-procedure/v1",
        "producerName":"fictional-tool", "producerVersion":"1",
        "sourceRepository":"fictional/source",
        "responseProtocol":"quoin.process-evidence/v1",
        "responseAdapter":"quoin.process-evidence-adapter",
        "responseAdapterVersion":"1", "repetitions":1, "timeoutMillis":1000,
        "inputs":[{"role":"spec","required":true}],
        "inputOrigins":[{"role":"spec","kind":"source_file",
            "sourceRepository":"fictional/source","sourcePath":"corpus/spec.tl"}]
    }))
    .expect("typed declared procedure")
}

fn member(name: &str, dependencies: &[&str]) -> CampaignMember {
    serde_json::from_value(json!({
        "name":name, "planId":"MP-FIXTURE", "definitionVersion":"v1",
        "dependsOn":dependencies, "required":true
    }))
    .expect("typed member")
}

fn attempt(name: &str, index: i64) -> CampaignAttempt {
    serde_json::from_value(json!({"member":name,"index":index,"status":"completed"}))
        .expect("typed attempt")
}

fn inventory(digest: &str, source: OriginSource) -> InputOriginInventory {
    InputOriginInventory {
        schema: INPUT_ORIGINS_SCHEMA.to_owned(),
        declaration: OriginDeclaration::Unclaimed,
        inputs: vec![InputOriginClaim {
            binding: InputOriginBinding {
                role: "spec".to_owned(),
                path: "spec.tl".to_owned(),
                bytes: InputByteIdentity {
                    digest: digest.to_owned(),
                    executable: false,
                },
            },
            source,
        }],
    }
}

/// Trace: FR-114-AC-3; PLAT-1061
#[test]
fn source_file_substitution_still_rejects_after_origin_rehash() {
    let repo = tempfile::tempdir().expect("repository");
    let bytes = b"exact tracked spec";
    let digest = ContentDigest::of_bytes(bytes).as_str().to_owned();
    let source_inputs = BTreeMap::from([(
        "fictional/source".to_owned(),
        vec![InputBinding {
            role: "source/corpus/spec.tl".to_owned(),
            path: "corpus/spec.tl".to_owned(),
            digest: ContentDigest::of_bytes(bytes),
            executable: false,
        }],
    )]);
    let implicit = serde_json::to_value(
        source_inputs
            .get("fictional/source")
            .and_then(|inputs| inputs.first())
            .expect("one source input"),
    )
    .expect("serialized source input");
    let request = json!({"inputs":[implicit,{"role":"spec","path":"spec.tl","digest":digest}]});
    let current = attempt("compile", 1);
    let attempts = [current.clone()];
    let valid = inventory(
        &digest,
        OriginSource::SourceFile {
            repository: "fictional/source".to_owned(),
            path: "corpus/spec.tl".to_owned(),
        },
    );
    assert!(
        check_input_origins(
            repo.path(),
            &valid,
            &request,
            &member("compile", &[]),
            &current,
            &attempts,
            &source_inputs,
            &procedure()
        )
        .is_ok()
    );
    let mut substituted = valid.clone();
    if let OriginSource::SourceFile { path, .. } = &mut substituted
        .inputs
        .first_mut()
        .expect("one selected input")
        .source
    {
        *path = "corpus/other.tl".to_owned();
    }
    assert_ne!(
        canonical_digest(&valid).expect("valid identity"),
        canonical_digest(&substituted).expect("rehashed identity")
    );
    assert!(matches!(
        check_input_origins(
            repo.path(),
            &substituted,
            &request,
            &member("compile", &[]),
            &current,
            &attempts,
            &source_inputs,
            &procedure()
        ),
        Err(EvidenceError::Contradiction)
    ));
}

/// Trace: FR-114-AC-3; PLAT-1061
#[test]
fn dependency_artifact_substitution_still_rejects_after_origin_rehash() {
    let repo = tempfile::tempdir().expect("repository");
    let bytes = b"sealed generated input";
    let (digest, _) = retain_bytes(repo.path(), "raw", "bin", bytes).expect("retained raw");
    let (result_digest, _) = retain_value(
        repo.path(),
        "results",
        &json!({
            "requestIdentity":{"digest":"a".repeat(64)},
            "state":{"kind":"completed"},
            "artifacts":[{"role":"generated/spec.tl","digest":digest,"byteLength":bytes.len()}]
        }),
    )
    .expect("retained EA result");
    let dependency: CampaignAttempt = serde_json::from_value(json!({
        "member":"prepare", "index":1, "status":"completed",
        "requestDigest":"a".repeat(64), "resultDigest":result_digest,
        "rawArtifacts":[{"role":"generated/spec.tl","digest":digest}]
    }))
    .expect("dependency attempt");
    let current = attempt("compile", 1);
    let attempts = [dependency, current.clone()];
    let request = json!({"inputs":[{"role":"spec","path":"spec.tl","digest":digest}]});
    let valid = inventory(
        &digest,
        OriginSource::Dependency {
            member: "prepare".to_owned(),
            index: 1,
            artifact_role: "generated/spec.tl".to_owned(),
        },
    );
    assert!(
        check_input_origins(
            repo.path(),
            &valid,
            &request,
            &member("compile", &["prepare"]),
            &current,
            &attempts,
            &BTreeMap::from([("fictional/source".to_owned(), Vec::new())]),
            &procedure()
        )
        .is_ok()
    );
    let mut substituted = valid.clone();
    if let OriginSource::Dependency { artifact_role, .. } = &mut substituted
        .inputs
        .first_mut()
        .expect("one selected input")
        .source
    {
        *artifact_role = "generated/other.tl".to_owned();
    }
    assert_ne!(
        canonical_digest(&valid).expect("valid identity"),
        canonical_digest(&substituted).expect("rehashed identity")
    );
    assert!(matches!(
        check_input_origins(
            repo.path(),
            &substituted,
            &request,
            &member("compile", &["prepare"]),
            &current,
            &attempts,
            &BTreeMap::from([("fictional/source".to_owned(), Vec::new())]),
            &procedure()
        ),
        Err(EvidenceError::Contradiction)
    ));
}

/// Trace: FR-114-AC-3. Provenance: PLAT-1061.
#[test]
fn authored_source_origin_cannot_be_downgraded_to_selected_bytes() {
    let repo = tempfile::tempdir().expect("repository");
    let digest = ContentDigest::of_bytes(b"tracked").as_str().to_owned();
    let source = InputBinding {
        role: "source/corpus/spec.tl".to_owned(),
        path: "corpus/spec.tl".to_owned(),
        digest: ContentDigest::of_bytes(b"tracked"),
        executable: false,
    };
    let request = json!({"inputs":[
        serde_json::to_value(&source).expect("source binding"),
        {"role":"spec","path":"spec.tl","digest":digest}
    ]});
    let source_inputs = BTreeMap::from([("fictional/source".to_owned(), vec![source])]);
    let current = attempt("compile", 1);
    let mut valid = inventory(
        &digest,
        OriginSource::SourceFile {
            repository: "fictional/source".to_owned(),
            path: "corpus/spec.tl".to_owned(),
        },
    );
    valid.declaration = OriginDeclaration::Enforced;
    let procedure = declared_procedure();
    assert!(
        check_input_origins(
            repo.path(),
            &valid,
            &request,
            &member("compile", &[]),
            &current,
            std::slice::from_ref(&current),
            &source_inputs,
            &procedure
        )
        .is_ok()
    );
    valid.inputs.first_mut().expect("one input").source = OriginSource::SelectedBytes;
    assert!(matches!(
        check_input_origins(
            repo.path(),
            &valid,
            &request,
            &member("compile", &[]),
            &current,
            std::slice::from_ref(&current),
            &source_inputs,
            &procedure
        ),
        Err(EvidenceError::Contradiction)
    ));
    valid.declaration = OriginDeclaration::Unclaimed;
    assert!(matches!(
        check_input_origins(
            repo.path(),
            &valid,
            &request,
            &member("compile", &[]),
            &current,
            std::slice::from_ref(&current),
            &source_inputs,
            &procedure
        ),
        Err(EvidenceError::Contradiction)
    ));
}
