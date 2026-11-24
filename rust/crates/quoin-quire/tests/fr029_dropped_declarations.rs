// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! FR-029: a model that lost declarations is refused, never computed over
//! (quoin PLAT-1854, quire-rs PLAT-1804).
//!
//! The scopes are real modules on disk. A flow rule that names an archetype no
//! module declares cannot load, and the engine drops that one declaration; a
//! `traceability:` section that is not a mapping drops the whole section, which
//! leaves the merged model empty. Both reach every compute through the public
//! API, exactly as the shell does.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::path::PathBuf;

use ix_trace_rs::trace;
use quoin_quire::{
    DroppedDeclaration, Error, ErrorCode, ModuleSelection, ScopeRoot, assurance, coverage,
    design_join, design_matrix, evidence, flow_report, ids,
};

const MODULE_NAME: &str = "quoin-quire-fixture";

/// A fixture module's manifest with `extra` appended inside `traceability:`.
fn manifest(extra: &str) -> String {
    let base = std::fs::read_to_string(fixture_dir().join("manifest.yaml"))
        .expect("the fixture manifest is checked in");
    format!("{base}{extra}")
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("repo")
}

/// A flow rule the engine cannot load: it names an archetype nothing declares.
fn bad_flow_rule(name: &str) -> String {
    format!(
        "  flow_rules:\n    - name: {name}\n      subject: [nowhere]\n      \
         match: [{{path: [], field: id, present: true}}]\n      check: {name}-check\n      \
         rationale: Names an archetype nothing declares.\n"
    )
}

/// Tests share a process and run in parallel, so each module gets its own directory.
static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// A scratch module directory that removes itself.
struct Module {
    dir: PathBuf,
}

impl Module {
    fn new(label: &str, manifest: &str) -> Self {
        let dir = std::env::temp_dir().join("quoin-quire-fr029").join(format!(
            "{}-{}-{label}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("module dir");
        std::fs::write(dir.join("manifest.yaml"), manifest).expect("manifest");
        Self { dir }
    }

    fn request(&self) -> coverage::Request {
        coverage::Request {
            scope: ScopeRoot::open(fixture_dir()).expect("the fixture repository is checked in"),
            modules: ModuleSelection::Closed(vec![
                ids::ModuleRoot::open(&self.dir).expect("the module is on disk"),
            ]),
        }
    }
}

impl Drop for Module {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn export_request(request: &coverage::Request) -> assurance::Request {
    assurance::Request {
        scope: request.scope.clone(),
        modules: request.modules.clone(),
    }
}

fn dropped(module: &str, kind: &str, item: &str) -> DroppedDeclaration {
    DroppedDeclaration {
        module: module.to_owned(),
        kind: kind.to_owned(),
        item: item.to_owned(),
    }
}

/// One declaration of the fixture module is dropped.
fn one_drop() -> Module {
    Module::new("one-drop", &manifest(&bad_flow_rule("bad")))
}

/// The whole `traceability:` section is unreadable, so the merged model is empty.
fn every_declaration_dropped() -> Module {
    let base = manifest("");
    let head = base
        .split("traceability:")
        .next()
        .expect("the fixture manifest declares artifact types first");
    Module::new(
        "all-dropped",
        &format!("{head}traceability:\n  required_relations: 5\n"),
    )
}

fn refusal(error: &Error) -> &[DroppedDeclaration] {
    assert_eq!(
        error.code(),
        ErrorCode::TraceabilityDeclarationsDropped,
        "{error}"
    );
    let Error::TraceabilityDeclarationsDropped { dropped } = error else {
        panic!("expected the dropped-declaration refusal, got {error:?}");
    };
    dropped
}

/// FR-029-AC-15: the coverage run returns the typed drop, never QQ-1020, with
/// one record per dropped declaration.
#[trace("FR-029-AC-15")]
#[test]
fn a_coverage_run_over_a_dropped_declaration_is_qq_1022_not_qq_1020() {
    let module = one_drop();
    let error = coverage::compute(&module.request()).expect_err("a dropped declaration refuses");
    assert_eq!(error.code().as_str(), "QQ-1022");
    assert_eq!(refusal(&error), [dropped(MODULE_NAME, "flow_rule", "bad")]);
}

/// FR-029-AC-15: the records keep the engine's order across several drops.
#[trace("FR-029-AC-15")]
#[test]
fn the_records_keep_the_engine_order() {
    let module = Module::new(
        "two-drops",
        &manifest(&format!(
            "{}{}",
            bad_flow_rule("zeta"),
            "    - name: alpha\n      subject: [nowhere]\n      match: [{path: [], field: id, \
             present: true}]\n      check: alpha-check\n      rationale: Names an archetype \
             nothing declares.\n"
        )),
    );
    let error = coverage::compute(&module.request()).expect_err("drops refuse");
    assert_eq!(
        refusal(&error),
        [
            dropped(MODULE_NAME, "flow_rule", "alpha"),
            dropped(MODULE_NAME, "flow_rule", "zeta"),
        ]
    );
}

/// FR-029-AC-16: the message renders every drop on its own line.
#[trace("FR-029-AC-16")]
#[test]
fn the_message_renders_every_dropped_declaration_on_its_own_line() {
    let error = Error::TraceabilityDeclarationsDropped {
        dropped: vec![
            dropped("alpha-module", "flow_rule", "first"),
            dropped("beta-module", "trace_target", "second"),
        ],
    };
    let message = error.to_string();
    let lines: Vec<&str> = message.lines().collect();
    assert!(
        lines.contains(&"module=alpha-module kind=flow_rule item=first"),
        "{message}"
    );
    assert!(
        lines.contains(&"module=beta-module kind=trace_target item=second"),
        "{message}"
    );
    assert!(message.contains("QQ-1022"), "{message}");
}

/// FR-029-AC-17: a whole section that cannot be read is one record of kind
/// `traceability_section`.
#[trace("FR-029-AC-17")]
#[test]
fn an_unreadable_section_is_the_traceability_section_record() {
    let module = every_declaration_dropped();
    let error = coverage::compute(&module.request()).expect_err("the section dropped");
    assert_eq!(
        refusal(&error),
        [dropped(MODULE_NAME, "traceability_section", "traceability")]
    );
}

/// FR-029-AC-19: through `quire.design_join`, the design matrix and the
/// flow-report compute, a dropped declaration is QQ-1022, never QQ-1110. The
/// engine-error mapping itself is unit-tested beside the mappers.
#[trace("FR-029-AC-19")]
#[test]
fn the_design_computes_refuse_a_dropped_declaration_with_qq_1022() {
    let module = one_drop();
    let expected = [dropped(MODULE_NAME, "flow_rule", "bad")];
    for error in [
        design_join::compute(&module.request())
            .map(|_| ())
            .expect_err("design_join refuses"),
        flow_report::compute(&module.request())
            .map(|_| ())
            .expect_err("flow report refuses"),
        design_matrix::compute(&module.request())
            .map(|_| ())
            .expect_err("matrix refuses"),
    ] {
        assert_eq!(refusal(&error), expected);
        assert_ne!(error.code().as_str(), "QQ-1110");
    }
}

/// FR-029-AC-20: with every declaration dropped the model reads as absent, and
/// each compute still refuses with the drop, not QQ-1020.
#[trace("FR-029-AC-20")]
#[test]
fn an_empty_model_from_dropped_declarations_is_qq_1022_in_every_compute() {
    let module = every_declaration_dropped();
    let expected = [dropped(MODULE_NAME, "traceability_section", "traceability")];
    let request = module.request();
    let evidence_request = evidence::Request {
        scope: module.request(),
        requested: Vec::new(),
        runs: Vec::new(),
        bindings: Vec::new(),
        injections: None,
        mock_inspection_suites: None,
        host: quoin_evidence::qualify::HostObservations::default(),
    };
    let errors = [
        coverage::compute(&request)
            .map(|_| ())
            .expect_err("coverage"),
        evidence::compute(&evidence_request)
            .map(|_| ())
            .expect_err("evidence"),
        flow_report::compute(&request)
            .map(|_| ())
            .expect_err("flow report"),
        assurance::build(&export_request(&request))
            .map(|_| ())
            .expect_err("assurance export"),
    ];
    for error in &errors {
        assert_eq!(refusal(error), expected);
    }
}

/// FR-029-AC-22: the trace-tag walk, the evidence compute and the assurance
/// export each refuse on a scope with a dropped declaration.
#[trace("FR-029-AC-22")]
#[test]
fn the_direct_readers_of_the_model_refuse_on_a_dropped_declaration() {
    let module = one_drop();
    let expected = [dropped(MODULE_NAME, "flow_rule", "bad")];
    let request = module.request();

    let walk = coverage::test_traces(&request).expect_err("the trace-tag walk refuses");
    assert_eq!(refusal(&walk), expected);

    let evidence_request = evidence::Request {
        scope: module.request(),
        requested: Vec::new(),
        runs: Vec::new(),
        bindings: Vec::new(),
        injections: None,
        mock_inspection_suites: None,
        host: quoin_evidence::qualify::HostObservations::default(),
    };
    let computed = evidence::compute(&evidence_request)
        .map(|_| ())
        .expect_err("evidence refuses");
    assert_eq!(refusal(&computed), expected);

    let export = assurance::build(&export_request(&request))
        .map(|_| ())
        .expect_err("the export refuses");
    assert_eq!(refusal(&export), expected);
}

/// FR-029-AC-23 (library half): without a dropped declaration the same
/// computes run and answer.
#[trace("FR-029-AC-23")]
#[test]
fn a_scope_without_a_dropped_declaration_still_computes() {
    let module = Module::new("clean", &manifest(""));
    let request = module.request();
    assert!(coverage::compute(&request).is_ok());
    assert!(coverage::test_traces(&request).is_ok());
    assert!(assurance::build(&export_request(&request)).is_ok());
    assert!(flow_report::compute(&request).is_ok());
}
