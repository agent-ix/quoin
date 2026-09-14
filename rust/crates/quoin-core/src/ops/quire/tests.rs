// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `quire` domain, decided against an engine in memory.
//!
//! Every test here substitutes a [`FakeEngine`] for the granted host, so the
//! whole domain — the request shapes, the ceilings, the exit mapping and the
//! two projections — is exercised with no repository on disk. That is the
//! property `tests/tc_library_containment.rs` makes a rule: `ops/quire/` is
//! handed what it may walk and acquires nothing.
//!
//! What a coverage rollup or a classification COMPUTES is `quoin-quire`'s and
//! is pinned there against the engine. What is pinned here is the boundary:
//! that a request reaches those functions unchanged, that a refusal reaches the
//! caller as the status the retained `runQuire` exited with, and that the
//! payload carries the fields — and only the fields — `src/quire/` handed its
//! six callers.
//!
//! Provenance: quoin#502

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use quoin_quire::properties::{
    AcShape, Criterion, Document, Extraction, PropertyShape, Report, Unresolved, UnresolvedReason,
};
use serde_json::{Value, json};

use crate::capabilities::{Capabilities, QuireHost};
use crate::error::CoreErrorCode;
use crate::protocol::Response;

use super::{MAX_SCALAR_BYTES, coverage, properties};

const HASH: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

/// What the fake engine was asked, and what it will answer.
///
/// The recorded arguments are half the point: the operation's own contract is
/// that a scope and a module list reach the engine unchanged, and only the host
/// can testify to that.
#[derive(Default)]
struct FakeEngine {
    obligations: Vec<quoin_quire::model::Obligation>,
    diagnostics: Vec<quoin_quire::model::CoverageDiagnostic>,
    documents: Vec<Document>,
    unresolved: Vec<Unresolved>,
    failure: Option<quoin_quire::ErrorCode>,
    seen: std::cell::RefCell<Vec<Call>>,
}

/// What one call handed the host.
struct Call {
    scope: PathBuf,
    modules: Vec<PathBuf>,
}

impl FakeEngine {
    fn declined(&self) -> Option<quoin_quire::Error> {
        match self.failure? {
            quoin_quire::ErrorCode::TraceabilityModelUndeclared => {
                Some(quoin_quire::Error::TraceabilityModelUndeclared)
            }
            other => panic!("this double only declines with a code it can construct: {other:?}"),
        }
    }

    fn record(&self, scope: &Path, modules: &[PathBuf]) {
        self.seen.borrow_mut().push(Call {
            scope: scope.to_path_buf(),
            modules: modules.to_vec(),
        });
    }
}

impl QuireHost for FakeEngine {
    fn coverage(
        &self,
        scope: &Path,
        modules: &[PathBuf],
    ) -> Result<quoin_quire::coverage::Outcome, quoin_quire::Error> {
        self.record(scope, modules);
        if let Some(error) = self.declined() {
            return Err(error);
        }
        let mut report = quire_report();
        report.obligations.clone_from(&self.obligations);
        report.diagnostics.clone_from(&self.diagnostics);
        Ok(quoin_quire::coverage::Outcome {
            report,
            // Notices are produced and deliberately not carried; see the
            // module header and quoin#513. A double that emitted none could not
            // distinguish "dropped" from "never there".
            notices: vec![quoin_quire::Notice {
                kind: quoin_quire::NoticeKind::Corpus,
                message: "an advisory nobody has ever been shown".to_owned(),
                path: None,
            }],
        })
    }

    fn properties(
        &self,
        scope: &Path,
        modules: &[PathBuf],
        _documents: &[String],
    ) -> Result<quoin_quire::properties::Outcome, quoin_quire::Error> {
        self.record(scope, modules);
        if let Some(error) = self.declined() {
            return Err(error);
        }
        Ok(quoin_quire::properties::Outcome {
            report: Report {
                documents: self.documents.clone(),
                engine: None,
            },
            unresolved: self.unresolved.clone(),
            notices: Vec::new(),
        })
    }
}

/// An empty engine report. `CoverageReport` is `Default`, and every field this
/// boundary does not carry stays at it.
fn quire_report() -> quoin_quire::model::CoverageReport {
    quoin_quire::model::CoverageReport::default()
}

/// One engine obligation, with every optional field populated.
fn obligation() -> quoin_quire::model::Obligation {
    quoin_quire::model::Obligation {
        source: "acceptance-criterion".to_owned(),
        id: "FR-001-AC-1".to_owned(),
        document: "spec/FR-001.md".to_owned(),
        statement: "The command reports coverage.".to_owned(),
        statement_hash: HASH.to_owned(),
        method: Some("Test".to_owned()),
        parameters: BTreeMap::from([("threshold".to_owned(), "0.9".to_owned())]),
        criticality: Some("high".to_owned()),
        target_ids: vec!["TC-1249".to_owned()],
    }
}

/// The same obligation with both collections empty.
fn bare_obligation() -> quoin_quire::model::Obligation {
    quoin_quire::model::Obligation {
        method: None,
        parameters: BTreeMap::new(),
        criticality: None,
        target_ids: Vec::new(),
        id: "FR-001-AC-2".to_owned(),
        ..obligation()
    }
}

fn diagnostic() -> quoin_quire::model::CoverageDiagnostic {
    quoin_quire::model::CoverageDiagnostic {
        declaration: "acceptance-criterion".to_owned(),
        reason: "uncatalogued-verification-method".to_owned(),
        message: "the method is not in the catalog".to_owned(),
        path: Some("spec/FR-001.md".to_owned()),
        line: Some(20),
        value: Some("Inspection".to_owned()),
        guidance: None,
    }
}

/// One classified criterion. `row_id` is the only field the projection reads
/// besides `property`, so the rest are filled once and never varied.
fn criterion(row_id: Option<&str>, property: PropertyShape) -> Criterion {
    Criterion {
        row_id: row_id.map(str::to_owned),
        statement: "The command reports coverage.".to_owned(),
        line: Some(20),
        shape: AcShape::Assertion,
        property,
        extractable: true,
        extraction: Extraction::Extractable,
        domain: None,
        precondition: None,
        oracle: None,
        signals: Vec::new(),
        obligation: None,
    }
}

fn document(criteria: Vec<Criterion>) -> Document {
    Document {
        document: "spec/FR-001.md".to_owned(),
        archetype: "FR".to_owned(),
        criteria,
    }
}

fn payload(response: &Response) -> &Value {
    &response.payload
}

/// `quire.coverage` carries the engine's obligations, and an empty collection
/// stays the absence it was on the wire.
///
/// Trace: FR-099
/// Provenance: quoin#502
#[test]
fn coverage_projects_the_obligations_the_engine_derived() {
    let engine = FakeEngine {
        obligations: vec![obligation(), bare_obligation()],
        diagnostics: vec![diagnostic()],
        ..FakeEngine::default()
    };
    let request = json!({ "scope": "/repo", "modules": ["/modules/quoin"] });
    let response = coverage(&request, &Capabilities::with_quire(&engine)).unwrap();

    assert_eq!(response.outcome.code(), 0);
    let obligations = payload(&response)["obligations"].as_array().unwrap();
    assert_eq!(obligations.len(), 2);
    assert_eq!(obligations[0]["id"], "FR-001-AC-1");
    assert_eq!(obligations[0]["statement_hash"], HASH);
    assert_eq!(obligations[0]["target_ids"], json!(["TC-1249"]));
    assert_eq!(obligations[0]["parameters"]["threshold"], "0.9");
    assert!(
        obligations[1].get("target_ids").is_none(),
        "an empty target list must serialise as an absent key, as quire emits it"
    );
    assert!(
        obligations[1].get("parameters").is_none(),
        "an empty parameter map must serialise as an absent key, not as `{{}}`"
    );

    let diagnostics = payload(&response)["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0]["reason"], "uncatalogued-verification-method");
    assert_eq!(diagnostics[0]["value"], "Inspection");
}

/// The scope and module roots reach the engine as the caller spelled them.
///
/// Trace: FR-099
/// Provenance: quoin#502
#[test]
fn the_request_reaches_the_engine_unchanged() {
    let engine = FakeEngine::default();
    let request = json!({ "scope": "/repo", "modules": ["/m/one", "/m/two"] });
    coverage(&request, &Capabilities::with_quire(&engine)).unwrap();

    let seen = engine.seen.borrow();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].scope, PathBuf::from("/repo"));
    assert_eq!(
        seen[0].modules,
        vec![PathBuf::from("/m/one"), PathBuf::from("/m/two")]
    );
}

/// An absent `modules` is ambient discovery, and reaches the host as the empty
/// list that says so rather than as a refusal.
///
/// Trace: FR-099
/// Provenance: quoin#502
#[test]
fn an_absent_module_list_is_ambient_discovery() {
    let engine = FakeEngine::default();
    let response = coverage(
        &json!({ "scope": "/repo" }),
        &Capabilities::with_quire(&engine),
    );
    assert_eq!(response.unwrap().outcome.code(), 0);
    assert!(engine.seen.borrow()[0].modules.is_empty());
}

/// `quire.properties` answers with the shape MAP `quoin advise` built by hand,
/// keyed on the obligation id, and skips a criterion that has no id to key on.
///
/// Trace: FR-052-CON-3
/// Provenance: quoin#502
#[test]
fn properties_answers_with_the_shape_map_keyed_on_the_obligation_id() {
    let engine = FakeEngine {
        documents: vec![document(vec![
            criterion(Some("FR-001-AC-1"), PropertyShape::RoundTrip),
            criterion(None, PropertyShape::Invariant),
            criterion(Some("FR-001-AC-2"), PropertyShape::ErrorCase),
        ])],
        unresolved: vec![Unresolved {
            document: "spec/assets/logo.md".to_owned(),
            reason: UnresolvedReason::ArchetypeUndeclared,
        }],
        ..FakeEngine::default()
    };
    let request = json!({ "scope": "/repo", "documents": ["spec/FR-001.md"] });
    let response = properties(&request, &Capabilities::with_quire(&engine)).unwrap();

    let shapes = &payload(&response)["shapes"];
    assert_eq!(shapes["FR-001-AC-1"]["property"], "round-trip");
    assert_eq!(shapes["FR-001-AC-1"]["archetype"], "FR");
    assert_eq!(shapes["FR-001-AC-2"]["property"], "error-case");
    assert_eq!(
        shapes.as_object().unwrap().len(),
        2,
        "a criterion with no row_id has no obligation to offer a shape for"
    );
    assert_eq!(
        payload(&response)["unresolved"],
        json!(["spec/assets/logo.md"]),
        "a document that resolved to no archetype is reported, not raised"
    );
}

/// An unresolved document is a partial result and not a failure — the exit
/// status `runQuireAllowFailure` existed to swallow (agent-ix/quoin#103).
///
/// Trace: FR-052
/// Provenance: quoin#502
#[test]
fn an_unresolved_document_is_a_clean_partial_answer() {
    let engine = FakeEngine {
        documents: vec![document(vec![criterion(
            Some("FR-001-AC-1"),
            PropertyShape::Example,
        )])],
        unresolved: vec![Unresolved {
            document: "spec/assets/plan.md".to_owned(),
            reason: UnresolvedReason::ArchetypeUnknown {
                archetype: "Sketch".to_owned(),
            },
        }],
        ..FakeEngine::default()
    };
    let response = properties(
        &json!({ "scope": "/repo" }),
        &Capabilities::with_quire(&engine),
    )
    .expect("a partial classification is an answer");
    assert_eq!(response.outcome.code(), 0);
    assert_eq!(payload(&response)["shapes"].as_object().unwrap().len(), 1);
}

/// An engine refusal reaches the caller as exit 2, naming the rule that
/// declined — which is the whole of what "quire exited 1" could not say
/// (quoin#106).
///
/// Trace: FR-099
/// Provenance: quoin#502
#[test]
fn an_engine_refusal_names_the_rule_that_declined() {
    let engine = FakeEngine {
        failure: Some(quoin_quire::ErrorCode::TraceabilityModelUndeclared),
        ..FakeEngine::default()
    };
    let error = coverage(
        &json!({ "scope": "/repo" }),
        &Capabilities::with_quire(&engine),
    )
    .unwrap_err();
    assert_eq!(error.code, CoreErrorCode::Refused);
    assert_eq!(error.outcome().code(), 2);
    assert_eq!(error.context["op"], "quire.coverage");
    assert_eq!(error.context["quire_code"], "QQ-1020");
}

/// A misspelled member is refused rather than silently dropped (rust-style §11).
///
/// Provenance: quoin#502
#[test]
fn an_unknown_member_is_refused() {
    let engine = FakeEngine::default();
    let error = coverage(
        &json!({ "scope": "/repo", "module": ["/m/one"] }),
        &Capabilities::with_quire(&engine),
    )
    .unwrap_err();
    assert_eq!(error.code, CoreErrorCode::BadRequest);
    assert_eq!(error.outcome().code(), 3);
    assert!(
        engine.seen.borrow().is_empty(),
        "a request that does not parse must not reach the engine"
    );
}

/// Every path is bounded before any walk begins, on both operations and on
/// every repeated field.
///
/// Provenance: quoin#502
#[test]
fn an_oversized_path_is_refused_before_any_walk() {
    let engine = FakeEngine::default();
    let grant = Capabilities::with_quire(&engine);
    let oversized = "/".repeat(MAX_SCALAR_BYTES + 1);

    let error = coverage(&json!({ "scope": "/repo", "modules": [oversized] }), &grant).unwrap_err();
    assert_eq!(error.code, CoreErrorCode::Refused);
    assert_eq!(error.context["field"], "modules");
    assert_eq!(error.context["limit_bytes"], MAX_SCALAR_BYTES.to_string());

    let error = properties(
        &json!({ "scope": "/repo", "documents": [oversized] }),
        &grant,
    )
    .unwrap_err();
    assert_eq!(error.code, CoreErrorCode::Refused);
    assert_eq!(error.context["field"], "documents");

    assert!(
        engine.seen.borrow().is_empty(),
        "a refused request must not reach the engine"
    );
}

/// A build that dispatched a quire operation without granting a host is a build
/// fault (4), never a refusal aimed at the caller (2).
///
/// Provenance: quoin#502
#[test]
fn a_missing_grant_is_an_internal_fault() {
    for (op, result) in [
        (
            "quire.coverage",
            coverage(&json!({ "scope": "/repo" }), &Capabilities::none()),
        ),
        (
            "quire.properties",
            properties(&json!({ "scope": "/repo" }), &Capabilities::none()),
        ),
    ] {
        let error = result.unwrap_err();
        assert_eq!(error.code, CoreErrorCode::Io);
        assert_eq!(error.outcome().code(), 4);
        assert_eq!(error.context["op"], op);
    }
}
