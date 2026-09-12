// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Every capability `src/quire/` exposes, exercised end to end against a real
//! module and a real corpus (quoin#379).
//!
//! The fixture repository is `tests/fixtures/repo`: one module, two documents
//! that classify, one that does not, and a source tree carrying exactly one
//! trace marker — so the rollup has one backed row, one unbacked row and one
//! status that claims complete for a row nothing backs.
//!
//! These tests reach only the public API and never shell out.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::{Path, PathBuf};

use quoin_quire::{
    ErrorCode, ModuleSelection, PayloadLimit, ScopeRoot, assurance, clauses, coverage, engine, ids,
    payload, properties, validate,
};

fn fixture_scope() -> ScopeRoot {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("repo");
    ScopeRoot::open(&root).expect("the fixture repository is checked in")
}

/// The fixture module is the scope root itself, named explicitly so the run is
/// attributable to these bytes and not to whatever is installed ambiently.
fn fixture_modules() -> ModuleSelection {
    ModuleSelection::Closed(vec![
        ids::ModuleRoot::open(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures")
                .join("repo"),
        )
        .expect("the fixture module is checked in"),
    ])
}

/// Trace: FR-099
///
/// Coverage: the capability `runQuire(["coverage", …])` + `parseCoverage`
/// carried for six quoin commands.
#[test]
fn tc_379_110_coverage_reconciles_the_fixture_corpus() {
    let outcome = coverage::compute(&coverage::Request {
        scope: fixture_scope(),
        modules: fixture_modules(),
    })
    .expect("the fixture declares a traceability model");

    let report = &outcome.report;
    // Measured against the fixture, not assumed: two criteria and two matrix
    // rows mint four trace targets, and exactly one of them — TC-001 — has a
    // source symbol behind it.
    assert_eq!(report.totals.total, 4);
    assert_eq!(report.totals.backed, 1);
    assert_eq!(report.totals.criteria, Some(2));

    let minted: Vec<(&str, bool)> = report
        .minted_targets
        .iter()
        .map(|row| (row.id.as_str(), row.backed))
        .collect();
    assert_eq!(
        minted,
        [
            ("FR-001-AC-1", false),
            ("FR-001-AC-2", false),
            ("TC-001", true),
            ("TC-002", false),
        ],
        "row identity behind the totals, so a moved number names which row moved",
    );

    // The unbacked row is TC-002's, and it names the id it is answerable for.
    let unbacked: Vec<&str> = report
        .unbacked_rows
        .iter()
        .filter_map(|row| row.row_id.as_deref())
        .collect();
    assert_eq!(
        unbacked,
        ["TC-002", "FR-001-AC-2"],
        "the matrix row and the criterion row that nothing backs",
    );
    assert!(
        report.unbacked_rows.iter().all(|row| row.line.is_some()),
        "every finding carries the document line it is at",
    );

    // ...and the matrix says ✅ for it, which is the status lie.
    let lies: Vec<&str> = report
        .status_lies
        .iter()
        .filter_map(|lie| lie.row_id.as_deref())
        .collect();
    assert_eq!(lies, ["TC-002"], "the ✅ on an unbacked row is the finding");

    // The binder's premise is reported even when it holds — the one list on
    // this report that is never omitted (quire-rs FR-050-AC-27).
    assert!(
        !report.binding_census.is_empty(),
        "the census is the premise the whole report rests on",
    );
}

/// Trace: FR-099
///
/// The agent-ix/quoin#106 diagnostic: a scope whose module set declares no
/// traceability model. The subprocess printed this to a discarded stderr; here
/// it is a named variant.
#[test]
fn tc_379_111_a_scope_without_a_traceability_model_names_that_exactly() {
    // A named module that declares no model, rather than the ambient set:
    // whether the machine running this suite happens to have a model
    // installed is not something a test may depend on.
    let no_model = ModuleSelection::Closed(vec![
        ids::ModuleRoot::open(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures")
                .join("module-no-traceability"),
        )
        .expect("the fixture module is checked in"),
    ]);
    let error = coverage::compute(&coverage::Request {
        scope: fixture_scope(),
        modules: no_model,
    })
    .expect_err("without a model there is nothing to reconcile");
    assert_eq!(error.code(), ErrorCode::TraceabilityModelUndeclared);
    assert!(
        error.to_string().contains("traceability"),
        "the refusal must name the missing declaration: {error}",
    );
}

/// Trace: FR-099
///
/// Properties: classification, and the agent-ix/quoin#103 partial result as
/// data rather than as an exit code plus a populated stdout.
#[test]
fn tc_379_112_classification_reports_partial_results_as_data() {
    let scope = fixture_scope();
    let mut documents = properties::documents_under_spec(&scope).expect("spec/ exists");
    assert_eq!(documents.len(), 2, "spec/ holds the two typed documents");
    // The two that do not resolve live outside spec/, so the corpus walk never
    // sees them; they are passed explicitly, the way `advise` passes a glob.
    documents.push(scope.as_path().join("notes").join("UNTYPED.md"));
    documents.push(scope.as_path().join("notes").join("UNREGISTERED.md"));

    let outcome = properties::classify(&properties::Request {
        scope: fixture_scope(),
        modules: fixture_modules(),
        documents,
        archetype: None,
    })
    .expect("classification does not fail over an untyped document");

    // The two typed documents classified...
    let classified: Vec<&str> = outcome
        .report
        .documents
        .iter()
        .map(|document| document.document.as_str())
        .collect();
    assert!(classified.iter().any(|d| d.ends_with("FR-001-example.md")));

    // ...and the two that do not are two records, not a thrown-away run —
    // each naming which premise failed, because "states no type" and "states
    // a type nothing registers" are different corpus defects.
    assert_eq!(outcome.unresolved.len(), 2);
    let reasons: Vec<(&str, &properties::UnresolvedReason)> = outcome
        .unresolved
        .iter()
        .map(|record| (record.document.as_str(), &record.reason))
        .collect();
    assert_eq!(
        reasons,
        [
            (
                "notes/UNTYPED.md",
                &properties::UnresolvedReason::ArchetypeUndeclared
            ),
            (
                "notes/UNREGISTERED.md",
                &properties::UnresolvedReason::ArchetypeUnknown {
                    archetype: "Memo".to_string()
                }
            ),
        ],
    );

    // The FR's criteria carry the fields a generator reads — the ones
    // `coverage --json` never had.
    let fr = outcome
        .report
        .documents
        .iter()
        .find(|document| document.document.ends_with("FR-001-example.md"))
        .expect("the FR classified");
    assert_eq!(fr.archetype, "FR");
    assert_eq!(fr.criteria.len(), 2);
    let ids: Vec<&str> = fr
        .criteria
        .iter()
        .filter_map(|criterion| criterion.row_id.as_deref())
        .collect();
    assert_eq!(ids, ["FR-001-AC-1", "FR-001-AC-2"]);
    assert!(
        fr.criteria
            .iter()
            .all(|criterion| !criterion.statement.is_empty()),
        "every record carries the untruncated statement its spans index",
    );
}

/// Trace: FR-097
///
/// The hand-written properties payload is a `Serialize` struct, so it has a
/// reviewable field list — and it survives a round trip, which a
/// `serde_json::json!` envelope has no type to be checked against.
#[test]
fn tc_379_113_the_properties_payload_round_trips() {
    let scope = fixture_scope();
    let outcome = properties::classify(&properties::Request {
        scope: fixture_scope(),
        modules: fixture_modules(),
        documents: properties::documents_under_spec(&scope).expect("spec/ exists"),
        archetype: None,
    })
    .expect("classifies");

    let encoded =
        payload::encode("properties", &outcome.report, PayloadLimit::DEFAULT).expect("encodes");
    let decoded: properties::Report = payload::from_slice(
        "properties",
        "properties::Report",
        encoded.as_bytes(),
        PayloadLimit::DEFAULT,
    )
    .expect("reads back");
    assert_eq!(outcome.report, decoded);

    // The keys `src/quire/types.ts` declares on a criterion, so the retained
    // TypeScript can still read a payload this crate writes during the staged
    // coexistence.
    let value: serde_json::Value = serde_json::from_str(&encoded).expect("valid JSON");
    let criterion = &value["documents"][0]["criteria"][0];
    for key in [
        "row_id",
        "statement",
        "line",
        "shape",
        "property",
        "extractable",
        "extraction",
        "domain",
        "precondition",
        "oracle",
        "signals",
        "obligation",
    ] {
        assert!(
            criterion.get(key).is_some(),
            "the payload lost `{key}`: {criterion}",
        );
    }
    assert!(value["engine"]["capabilities"].is_array());
}

/// Trace: FR-099
///
/// Validation: the capability `runQuireBatch(["validate", …])` carried, with
/// the three-way outcome taxonomy preserved and the stderr-scraping gone.
#[test]
fn tc_379_114_validation_reports_an_outcome_per_document() {
    let scope = fixture_scope();
    let mut documents = properties::documents_under_spec(&scope).expect("spec/ exists");
    documents.push(scope.as_path().join("notes").join("UNTYPED.md"));
    documents.push(scope.as_path().join("spec").join("does-not-exist.md"));

    let outcome = validate::run(&validate::Request {
        scope: fixture_scope(),
        modules: fixture_modules(),
        documents,
    })
    .expect("a rejected document is not a failed run");

    assert_eq!(outcome.evaluations.len(), 4);

    // Every diagnostic names its own document as a field. The TypeScript had
    // to recover this with a regular expression over the message.
    for evaluation in &outcome.evaluations {
        for diagnostic in &evaluation.diagnostics {
            assert_eq!(diagnostic.document, evaluation.document);
        }
    }

    let untyped = outcome
        .evaluations
        .iter()
        .find(|evaluation| evaluation.document.ends_with("UNTYPED.md"))
        .expect("the untyped document has an evaluation");
    assert_eq!(untyped.outcome, validate::DocumentOutcome::CouldNotRun);
    assert_eq!(
        untyped.not_run,
        Some(validate::NotRunReason::ArchetypeUndeclared),
        "never reached is not the same fact as rejected",
    );

    let missing = outcome
        .evaluations
        .iter()
        .find(|evaluation| evaluation.document.ends_with("does-not-exist.md"))
        .expect("the missing document has an evaluation");
    assert_eq!(missing.outcome, validate::DocumentOutcome::CouldNotRun);
    assert!(matches!(
        missing.not_run,
        Some(validate::NotRunReason::Unreadable { .. })
    ));

    // The two typed documents were actually reached.
    assert_eq!(
        outcome
            .evaluations
            .iter()
            .filter(|evaluation| evaluation.outcome != validate::DocumentOutcome::CouldNotRun)
            .count(),
        2,
    );
}

/// Trace: FR-100
///
/// Assurance: the producing half quoin never had, and the engine's own
/// fail-closed reader in place of the vendored `assurance-v1` schema.
#[test]
fn tc_379_115_an_assurance_export_builds_and_reads_back_under_its_own_premises() {
    let outcome = assurance::build(&assurance::Request {
        scope: fixture_scope(),
        modules: fixture_modules(),
        repository: ids::RepositoryId::new("agent-ix/quoin").expect("non-empty"),
        revision: ids::RevisionId::new("0".repeat(40)).expect("40 hex"),
    })
    .expect("the fixture corpus exports");

    let export = &outcome.export;
    assert_eq!(export.format, "quire-assurance");
    assert_eq!(export.format_version, 1);
    assert_eq!(export.source.repository, "agent-ix/quoin");
    assert!(
        export.artifacts.len() >= 2,
        "the FR and the matrix are artifacts: {:?}",
        export.artifacts.len(),
    );

    // Read it back through the engine's reader, under the premises the export
    // itself states. There is no vendored schema anywhere in this path.
    let bytes = export.to_json_bytes().expect("serializes");
    let accepted = quoin_quire::model::AcceptedAssurancePremises::from_export(export);
    let round_tripped = assurance::read(
        "the fixture export",
        &bytes,
        &accepted,
        PayloadLimit::DEFAULT,
    )
    .expect("the engine's reader accepts what the engine's builder wrote");
    assert_eq!(&round_tripped, export);

    // A premise the export does not satisfy is refused **by the engine**, not
    // by anything quoin restates. The accepted set is a claim about which
    // module version produced the export, so moving that version is the
    // premise failure: a caller that accepted 9.9.9 must not silently be
    // handed an export built by 0.1.0.
    //
    // Adding an *extra* accepted module would not refuse, and that is correct
    // — the accepted set says what the caller allows, so a superset allows
    // more. Asserted the other way round on purpose: an earlier draft of this
    // test used the superset and passed for the wrong reason.
    let mut wrong = accepted.clone();
    wrong
        .modules
        .first_mut()
        .expect("the export names its module")
        .version = "9.9.9".to_string();
    let error = assurance::read("the fixture export", &bytes, &wrong, PayloadLimit::DEFAULT)
        .expect_err("an unmet premise must refuse");
    assert_eq!(error.code(), ErrorCode::Assurance);
}

/// Trace: FR-096
///
/// Clause sets: the module declares none, and "declares none" must be a named
/// refusal that lists what *is* loaded — not an empty report.
#[test]
fn tc_379_116_an_absent_clause_set_is_named_with_what_is_loaded() {
    let module = ids::ModuleRoot::open(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("repo"),
    )
    .expect("the fixture module is checked in");
    let wanted = quoin_quire::ClauseSetRef {
        authority: quoin_quire::ClauseSetAuthority::new("iso").expect("non-empty"),
        id: quoin_quire::ClauseSetId::new("29148").expect("non-empty"),
        version: quoin_quire::ClauseSetVersion::new("2018").expect("non-empty"),
    };
    let mut notices = Vec::new();
    let error = clauses::evaluate(&module, &wanted, &clauses::Context::new(), &mut notices)
        .expect_err("the fixture declares no clause sets");
    assert_eq!(error.code(), ErrorCode::ClauseSetNotLoaded);
    let message = error.to_string();
    assert!(message.contains("iso/29148/2018"), "{message}");
    assert!(
        message.contains("none"),
        "the refusal must say what is loaded, so 'absent' and 'present at \
         another version' are tellable apart: {message}",
    );
}

/// Trace: FR-096
///
/// A document outside the scope is refused rather than reported under a path
/// that looks scope-relative (rust-review §10).
#[test]
fn tc_379_117_a_document_outside_the_scope_is_refused() {
    let outside = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let error = validate::run(&validate::Request {
        scope: fixture_scope(),
        modules: fixture_modules(),
        documents: vec![outside],
    })
    .expect_err("a path outside the scope must be refused");
    assert_eq!(error.code(), ErrorCode::PathEscapesRoot);
}

/// Trace: FR-097
///
/// The capability list is not decorative: every token this build publishes
/// names an engine surface, and the witnesses in `engine.rs` are what make
/// that true at compile time. Asserted here so the list is also non-empty and
/// reaches a consumer.
#[test]
fn tc_379_118_the_published_capabilities_reach_a_payload() {
    let provenance = engine::Provenance::current();
    assert!(!provenance.capabilities.is_empty());
    for token in ["binding_census", "assurance_export.v1", "clause_sets"] {
        assert!(
            provenance.capabilities.iter().any(|c| c == token),
            "{token} is not published",
        );
    }
    assert_eq!(provenance.engine, engine::ENGINE_VERSION);
    assert_eq!(engine::ENGINE_REVISION.len(), 40);
    assert!(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("Cargo.toml")
            .is_file()
    );
}
