// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `semantic` domain's unit tests, against an in-memory host.
//!
//! Every test here runs with **no disk**: the host is a recorder that answers
//! from a canned result and remembers what it was asked. That is the property
//! [`crate::capabilities`] exists for — the refusal, mapping, ordering and
//! payload paths are all decided by this module, so they can all be exercised
//! without a vendored contract tree, a module or a corpus.
//!
//! What the real host does with a real tree is `quoin-semantic`'s own golden
//! parity suite, and what the wire spelling is end to end is
//! `tests/tc_452_semantic_boundary.rs` against the real binary.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use quoin_semantic::{
    CompatibilityPosture, CorpusRoot, DiagnosticCode, LegacyForms, SemanticBlock,
    SemanticDiagnostic, SemanticError, SemanticModule, SemanticReadResult, SweepCounts,
    SweepIdentity, SweepReport,
};

use super::{migration_example, read_blocks, sweep_corpus};
use crate::capabilities::{Capabilities, SemanticHost};
use crate::error::CoreErrorCode;

/// A block with every key filled, so a payload assertion is about the shape and
/// not about which fields happened to be set.
fn block(package: &str) -> SemanticBlock {
    SemanticBlock {
        contract_version: "1.0.0".into(),
        semantic_core: "0.1.0".into(),
        package: package.into(),
        exports: vec!["Entity".into()],
        imports: BTreeMap::new(),
        targets: vec!["filament".to_owned()],
        mappings: vec![],
        compatibility_posture: CompatibilityPosture::Strict,
        legacy_forms: LegacyForms::Warning,
        sweep_report: None,
    }
}

/// The in-memory [`SemanticHost`]: answers from a canned result, records what
/// it was asked.
struct Recorder {
    read: RefCell<Vec<PathBuf>>,
    swept: RefCell<Vec<(Vec<CorpusRoot>, SweepIdentity, String)>>,
    answer: Result<SemanticReadResult, SemanticErrorKind>,
}

/// A failure the recorder can be told to raise.
///
/// A kind rather than a `SemanticError`, because `SemanticError` is not
/// `Clone` and a recorder answers more than once.
#[derive(Debug, Clone, Copy)]
enum SemanticErrorKind {
    ContractRootUnset,
}

impl SemanticErrorKind {
    fn build(self) -> SemanticError {
        match self {
            Self::ContractRootUnset => SemanticError::ContractRootUnset,
        }
    }
}

impl Recorder {
    fn answering(module: Option<SemanticModule>, diagnostics: Vec<SemanticDiagnostic>) -> Self {
        Self {
            read: RefCell::new(Vec::new()),
            swept: RefCell::new(Vec::new()),
            answer: Ok(SemanticReadResult {
                module,
                diagnostics,
            }),
        }
    }

    fn failing(kind: SemanticErrorKind) -> Self {
        Self {
            read: RefCell::new(Vec::new()),
            swept: RefCell::new(Vec::new()),
            answer: Err(kind),
        }
    }
}

impl SemanticHost for Recorder {
    fn read_module(&self, module_root: &Path) -> Result<SemanticReadResult, SemanticError> {
        self.read.borrow_mut().push(module_root.to_path_buf());
        match &self.answer {
            Ok(result) => Ok(SemanticReadResult {
                module: result.module.clone(),
                diagnostics: result.diagnostics.clone(),
            }),
            Err(kind) => Err(kind.build()),
        }
    }

    fn sweep(
        &self,
        roots: &[CorpusRoot],
        identity: &SweepIdentity,
        generated_at: &str,
    ) -> Result<SweepReport, SemanticError> {
        self.swept
            .borrow_mut()
            .push((roots.to_vec(), identity.clone(), generated_at.to_owned()));
        if let Err(kind) = &self.answer {
            return Err(kind.build());
        }
        Ok(SweepReport {
            package: identity.package.as_str().to_owned(),
            version: identity.version.clone(),
            generated_at: generated_at.to_owned(),
            corpus: vec![],
            counts: SweepCounts {
                artifacts: 0,
                forms: BTreeMap::new(),
                legacy: BTreeMap::new(),
            },
            findings: vec![],
        })
    }
}

/// One module's read result, with the block filled in.
fn module_at(root: &str, package: &str) -> SemanticModule {
    SemanticModule {
        name: "spec-objects".into(),
        version: "0.3.0".into(),
        root: PathBuf::from(root),
        block: block(package),
        data_schemas: BTreeMap::new(),
    }
}

#[test]
fn read_blocks_answers_one_view_per_root_in_the_order_asked() {
    let host = Recorder::answering(Some(module_at("/m", "agent-ix/spec-objects")), vec![]);
    let response = read_blocks(
        &serde_json::json!({ "roots": ["/a", "/b", "/c"] }),
        &Capabilities::with_semantic(&host),
    )
    .expect("a well-formed request with a host is answered");

    let modules = response.payload["modules"]
        .as_array()
        .expect("modules is an array");
    assert_eq!(modules.len(), 3);
    // The echoed root is what pairs an answer with the caller's own record, so
    // it is asserted rather than the length alone.
    assert_eq!(
        modules
            .iter()
            .map(|m| m["root"].as_str().unwrap_or_default())
            .collect::<Vec<_>>(),
        ["/a", "/b", "/c"]
    );
    assert_eq!(
        modules[0]["block"]["package"], "agent-ix/spec-objects",
        "the block the host read is what the payload carries"
    );
    assert_eq!(
        host.read.borrow().len(),
        3,
        "one host read per root, and no more"
    );
}

#[test]
fn a_module_with_no_semantic_block_carries_no_block_and_no_diagnostics() {
    let host = Recorder::answering(None, vec![]);
    let response = read_blocks(
        &serde_json::json!({ "roots": ["/plain"] }),
        &Capabilities::with_semantic(&host),
    )
    .expect("a module without a semantic block is not an error");
    let view = &response.payload["modules"][0];
    assert!(
        view.get("block").is_none(),
        "an absent block is absent, not null: {view}"
    );
    assert_eq!(view["diagnostics"].as_array().map(Vec::len), Some(0));
}

#[test]
fn a_refused_block_is_reported_as_diagnostics_and_not_as_a_failure() {
    // The whole point of the diagnostic type: a module that violates the
    // contract is READ successfully and reported, exactly as `readSemanticBlock`
    // did. A non-zero exit here would turn one bad module into a failed catalog.
    let host = Recorder::answering(
        None,
        vec![SemanticDiagnostic::error(
            DiagnosticCode::UnknownKey,
            "semantic.nope",
            "unknown key inside semantic: nope",
        )],
    );
    let response = read_blocks(
        &serde_json::json!({ "roots": ["/bad"] }),
        &Capabilities::with_semantic(&host),
    )
    .expect("a contract violation is a diagnostic, not an error");
    assert_eq!(response.outcome.code(), 0);
    let diagnostics = &response.payload["modules"][0]["diagnostics"];
    assert_eq!(diagnostics[0]["code"], "semantic.unknown-key");
    assert_eq!(diagnostics[0]["severity"], "error");
}

#[test]
fn an_oversized_root_is_refused_before_the_host_is_consulted() {
    let host = Recorder::answering(None, vec![]);
    let error = read_blocks(
        &serde_json::json!({ "roots": ["x".repeat(super::MAX_SCALAR_BYTES + 1)] }),
        &Capabilities::with_semantic(&host),
    )
    .expect_err("a path past the ceiling is refused");
    assert_eq!(error.code, CoreErrorCode::Refused);
    assert_eq!(error.outcome().code(), 2);
    assert_eq!(error.context["field"], "roots");
    assert_eq!(
        error.context["limit_bytes"],
        super::MAX_SCALAR_BYTES.to_string()
    );
    assert!(
        host.read.borrow().is_empty(),
        "the bound is applied BEFORE the host: it was consulted anyway"
    );
}

#[test]
fn an_oversized_request_is_refused_before_the_host_is_consulted() {
    let host = Recorder::answering(None, vec![]);
    // Many short roots rather than one long one: this is the bound the
    // per-path ceiling cannot state.
    let roots: Vec<String> = (0..150_000).map(|i| format!("/m/{i}")).collect();
    let error = read_blocks(
        &serde_json::json!({ "roots": roots }),
        &Capabilities::with_semantic(&host),
    )
    .expect_err("a request past the ceiling is refused");
    assert_eq!(error.code, CoreErrorCode::Refused);
    assert_eq!(error.context["op"], "semantic.read_blocks");
    assert_eq!(
        error.context["limit_bytes"],
        super::MAX_READ_BLOCKS_BYTES.to_string()
    );
    assert!(host.read.borrow().is_empty());
}

#[test]
fn a_host_failure_carries_its_semantic_code_onto_the_taxonomy() {
    let host = Recorder::failing(SemanticErrorKind::ContractRootUnset);
    let error = read_blocks(
        &serde_json::json!({ "roots": ["/m"] }),
        &Capabilities::with_semantic(&host),
    )
    .expect_err("no vendored contract is a refusal");
    assert_eq!(error.code, CoreErrorCode::Refused);
    assert_eq!(error.context["semantic_code"], "QSEM-009");
}

#[test]
fn an_operation_dispatched_without_a_host_is_an_internal_fault_not_a_refusal() {
    // Internal (4), not Refused (2): the caller did nothing wrong. This is a
    // build that routed an operation without granting what it needs.
    let error = read_blocks(
        &serde_json::json!({ "roots": ["/m"] }),
        &Capabilities::none(),
    )
    .expect_err("a semantic op needs a semantic host");
    assert_eq!(error.code, CoreErrorCode::Io);
    assert_eq!(error.outcome().code(), 4);
}

#[test]
fn a_request_of_the_wrong_shape_is_a_caller_mistake() {
    let host = Recorder::answering(None, vec![]);
    for bad in [
        serde_json::json!({}),
        serde_json::json!({ "roots": "/m" }),
        serde_json::json!({ "roots": [], "extra": 1 }),
    ] {
        let error = read_blocks(&bad, &Capabilities::with_semantic(&host))
            .expect_err("a malformed request is refused");
        assert_eq!(error.code, CoreErrorCode::BadRequest, "{bad}");
        assert_eq!(error.outcome().code(), 3);
    }
}

#[test]
fn sweep_corpus_hands_the_host_the_roots_identity_and_clock_it_was_given() {
    let host = Recorder::answering(None, vec![]);
    let response = sweep_corpus(
        &serde_json::json!({
            "roots": [{ "root": "/corpus", "repository": "quire-rs", "revision": "abc" }],
            "package": "agent-ix/spec-objects-business",
            "version": "0.3.0",
            "generated_at": "2026-09-12T00:00:00.000Z",
        }),
        &Capabilities::with_semantic(&host),
    )
    .expect("a well-formed sweep is answered");

    let asked = host.swept.borrow();
    let (roots, identity, generated_at) = asked.first().expect("the host was asked exactly once");
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].root, PathBuf::from("/corpus"));
    assert_eq!(roots[0].repository, "quire-rs");
    assert_eq!(roots[0].revision, "abc");
    assert_eq!(identity.package.as_str(), "agent-ix/spec-objects-business");
    assert_eq!(identity.version, "0.3.0");
    // The clock is the CALLER's. A timestamp minted inside the boundary could
    // not be asserted by anyone.
    assert_eq!(generated_at, "2026-09-12T00:00:00.000Z");
    assert_eq!(
        response.payload["report"]["generatedAt"], "2026-09-12T00:00:00.000Z",
        "the report's own camelCase spelling is the schema's, not the request's"
    );
}

#[test]
fn an_oversized_sweep_scalar_is_refused_before_the_host_is_consulted() {
    let host = Recorder::answering(None, vec![]);
    let error = sweep_corpus(
        &serde_json::json!({
            "roots": [],
            "package": "x".repeat(super::MAX_SCALAR_BYTES + 1),
            "version": "0.3.0",
            "generated_at": "2026-09-12T00:00:00.000Z",
        }),
        &Capabilities::with_semantic(&host),
    )
    .expect_err("an identity past the ceiling is refused");
    assert_eq!(error.code, CoreErrorCode::Refused);
    assert_eq!(error.context["field"], "package");
    assert!(host.swept.borrow().is_empty());
}

#[test]
fn migration_example_serves_the_one_copy_of_the_guidance_and_takes_no_arguments() {
    let response =
        migration_example(&serde_json::json!({})).expect("the guidance needs no capability");
    let served = response.payload["example"]
        .as_str()
        .expect("example is a string");
    assert_eq!(served, quoin_semantic::LEGACY_MIGRATION_EXAMPLE);
    // Not a tautology against the line above: this is what a user reads off
    // `quoin write`, and the constant is pinned to the diagnostic text by
    // `quoin-semantic`'s own suite.
    assert!(served.contains("Properties migration (FR-074)"), "{served}");
    assert!(served.contains("| Field | Type | Multiplicity | Constraints |"));

    let error = migration_example(&serde_json::json!({ "x": 1 }))
        .expect_err("an operation with no arguments says so");
    assert_eq!(error.code, CoreErrorCode::BadRequest);
}
