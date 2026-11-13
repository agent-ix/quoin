// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Tests for the operation router and its self-census.

use super::*;

/// Every `ops` source file, as `(domain, path, source)`.
///
/// A domain is one file (`ops/core.rs`) or a directory whose `mod.rs`
/// re-exports its parts (`ops/modules/`), so the domain is named beside the
/// path rather than derived from it: `ops/modules/wire.rs` declares bounds
/// that a caller reaches as `ops::modules::MAX_*`.
///
/// `include_str!` takes a literal path, so this list is hand-written — but
/// it is checked against `ops/mod.rs` and against each directory's own
/// `mod.rs` by [`the_bound_census_can_see_every_ops_module`], which are
/// different files, so the two cannot go stale together.
const OPS_SOURCES: &[(&str, &str, &str)] = &[
    (
        "assurance",
        "assurance.rs",
        include_str!("../ops/assurance.rs"),
    ),
    (
        "auditor",
        "auditor/mod.rs",
        include_str!("../ops/auditor/mod.rs"),
    ),
    (
        "auditor",
        "auditor/taxonomy.rs",
        include_str!("../ops/auditor/taxonomy.rs"),
    ),
    (
        "auditor",
        "auditor/tests.rs",
        include_str!("../ops/auditor/tests.rs"),
    ),
    (
        "auditor",
        "auditor/wire.rs",
        include_str!("../ops/auditor/wire.rs"),
    ),
    ("catalog", "catalog.rs", include_str!("../ops/catalog.rs")),
    (
        "change_assurance",
        "change_assurance/mod.rs",
        include_str!("../ops/change_assurance/mod.rs"),
    ),
    (
        "change_assurance",
        "change_assurance/support.rs",
        include_str!("../ops/change_assurance/support.rs"),
    ),
    (
        "change_assurance",
        "change_assurance/taxonomy.rs",
        include_str!("../ops/change_assurance/taxonomy.rs"),
    ),
    (
        "change_assurance",
        "change_assurance/tests.rs",
        include_str!("../ops/change_assurance/tests.rs"),
    ),
    (
        "change_assurance",
        "change_assurance/wire.rs",
        include_str!("../ops/change_assurance/wire.rs"),
    ),
    (
        "completeness",
        "completeness.rs",
        include_str!("../ops/completeness.rs"),
    ),
    (
        "config",
        "config/mod.rs",
        include_str!("../ops/config/mod.rs"),
    ),
    (
        "config",
        "config/taxonomy.rs",
        include_str!("../ops/config/taxonomy.rs"),
    ),
    ("core", "core.rs", include_str!("../ops/core.rs")),
    (
        "evidence",
        "evidence/discharge.rs",
        include_str!("../ops/evidence/discharge.rs"),
    ),
    (
        "evidence",
        "evidence/documents.rs",
        include_str!("../ops/evidence/documents.rs"),
    ),
    (
        "evidence",
        "evidence/mod.rs",
        include_str!("../ops/evidence/mod.rs"),
    ),
    (
        "evidence",
        "evidence/records.rs",
        include_str!("../ops/evidence/records.rs"),
    ),
    (
        "evidence",
        "evidence/store.rs",
        include_str!("../ops/evidence/store.rs"),
    ),
    (
        "evidence",
        "evidence/taxonomy.rs",
        include_str!("../ops/evidence/taxonomy.rs"),
    ),
    (
        "evidence",
        "evidence/tests.rs",
        include_str!("../ops/evidence/tests.rs"),
    ),
    (
        "evidence",
        "evidence/wire.rs",
        include_str!("../ops/evidence/wire.rs"),
    ),
    ("graph", "graph/mod.rs", include_str!("../ops/graph/mod.rs")),
    (
        "graph",
        "graph/taxonomy.rs",
        include_str!("../ops/graph/taxonomy.rs"),
    ),
    (
        "graph",
        "graph/tests.rs",
        include_str!("../ops/graph/tests.rs"),
    ),
    (
        "graph",
        "graph/wire.rs",
        include_str!("../ops/graph/wire.rs"),
    ),
    ("matrix", "matrix.rs", include_str!("../ops/matrix.rs")),
    (
        "measurement",
        "measurement/mod.rs",
        include_str!("../ops/measurement/mod.rs"),
    ),
    (
        "measurement",
        "measurement/attach.rs",
        include_str!("../ops/measurement/attach.rs"),
    ),
    (
        "measurement",
        "measurement/portfolio.rs",
        include_str!("../ops/measurement/portfolio.rs"),
    ),
    (
        "measurement",
        "measurement/produce.rs",
        include_str!("../ops/measurement/produce.rs"),
    ),
    (
        "measurement",
        "measurement/record.rs",
        include_str!("../ops/measurement/record.rs"),
    ),
    (
        "measurement",
        "measurement/report.rs",
        include_str!("../ops/measurement/report.rs"),
    ),
    (
        "measurement",
        "measurement/taxonomy.rs",
        include_str!("../ops/measurement/taxonomy.rs"),
    ),
    (
        "measurement",
        "measurement/tests.rs",
        include_str!("../ops/measurement/tests.rs"),
    ),
    (
        "measurement",
        "measurement/verify.rs",
        include_str!("../ops/measurement/verify.rs"),
    ),
    (
        "measurement",
        "measurement/wire.rs",
        include_str!("../ops/measurement/wire.rs"),
    ),
    (
        "modules",
        "modules/mod.rs",
        include_str!("../ops/modules/mod.rs"),
    ),
    (
        "modules",
        "modules/support.rs",
        include_str!("../ops/modules/support.rs"),
    ),
    (
        "modules",
        "modules/taxonomy.rs",
        include_str!("../ops/modules/taxonomy.rs"),
    ),
    (
        "modules",
        "modules/tests.rs",
        include_str!("../ops/modules/tests.rs"),
    ),
    (
        "modules",
        "modules/wire.rs",
        include_str!("../ops/modules/wire.rs"),
    ),
    ("quire", "quire/mod.rs", include_str!("../ops/quire/mod.rs")),
    (
        "quire",
        "quire/taxonomy.rs",
        include_str!("../ops/quire/taxonomy.rs"),
    ),
    (
        "quire",
        "quire/tests.rs",
        include_str!("../ops/quire/tests.rs"),
    ),
    (
        "quire",
        "quire/wire.rs",
        include_str!("../ops/quire/wire.rs"),
    ),
    (
        "semantic",
        "semantic/mod.rs",
        include_str!("../ops/semantic/mod.rs"),
    ),
    (
        "semantic",
        "semantic/taxonomy.rs",
        include_str!("../ops/semantic/taxonomy.rs"),
    ),
    (
        "semantic",
        "semantic/tests.rs",
        include_str!("../ops/semantic/tests.rs"),
    ),
    (
        "semantic",
        "semantic/wire.rs",
        include_str!("../ops/semantic/wire.rs"),
    ),
    (
        "validators",
        "validators.rs",
        include_str!("../ops/validators.rs"),
    ),
];

/// The `mod` names a source declares, whatever the visibility or `cfg`.
fn declared_modules(source: &str) -> Vec<String> {
    source
        .lines()
        .map(str::trim)
        .filter_map(|line| {
            line.strip_prefix("pub mod ")
                .or_else(|| line.strip_prefix("mod "))
        })
        .filter_map(|rest| rest.split_once(';'))
        .map(|(name, _)| name.to_owned())
        .collect()
}

/// The domains `ops/mod.rs` declares, in its own spelling.
fn declared_ops_modules() -> Vec<String> {
    declared_modules(include_str!("../ops/mod.rs"))
}

/// Every `pub const MAX_*_BYTES` an `ops` source declares, qualified by the
/// domain that re-exports it.
fn declared_domain_bounds() -> Vec<String> {
    OPS_SOURCES
        .iter()
        .flat_map(|(domain, _, source)| {
            source
                .lines()
                .filter_map(|line| line.trim().strip_prefix("pub const "))
                .filter_map(|rest| rest.split_once(':'))
                .map(|(name, _)| name.trim())
                .filter(|name| name.starts_with("MAX_") && name.ends_with("_BYTES"))
                .map(move |name| format!("ops::{domain}::{name}"))
        })
        .collect()
}

/// Without this, a rename of `pub const` or of the `mod` spelling turns the
/// census check into a comparison of two empty lists, which passes.
///
/// Two levels are checked, because a domain has two shapes: `ops/mod.rs`
/// against the domains listed, and each directory domain's `mod.rs` against
/// the files listed under it. Checking only the first would let a bound
/// added to a new `ops/modules/<new>.rs` go uncensused.
#[test]
fn the_bound_census_can_see_every_ops_module() {
    let mut declared = declared_ops_modules();
    declared.sort();
    let mut included: Vec<String> = OPS_SOURCES
        .iter()
        .map(|(domain, _, _)| (*domain).to_owned())
        .collect();
    included.sort();
    included.dedup();
    assert_eq!(
        declared, included,
        "`ops/mod.rs` and `OPS_SOURCES` disagree about which domains exist; \
         a domain missing from `OPS_SOURCES` has its size bounds uncensused"
    );

    for (domain, path, source) in OPS_SOURCES {
        if !path.ends_with("/mod.rs") {
            continue;
        }
        let mut expected: Vec<String> = declared_modules(source)
            .into_iter()
            .map(|name| format!("{domain}/{name}.rs"))
            .chain(std::iter::once((*path).to_owned()))
            .collect();
        expected.sort();
        let mut listed: Vec<String> = OPS_SOURCES
            .iter()
            .filter(|(other, _, _)| other == domain)
            .map(|(_, other, _)| (*other).to_owned())
            .collect();
        listed.sort();
        assert_eq!(
            expected, listed,
            "`{path}` and `OPS_SOURCES` disagree about which files make up \
             `ops::{domain}`; a file missing from `OPS_SOURCES` has its size \
             bounds uncensused"
        );
    }
}

/// Every `"…" =>` arm of `dispatch`'s match, read out of this file's own
/// source.
///
/// `OPERATIONS` is a hand-written list standing beside an exhaustive
/// `match`, and the compiler checks neither against the other. It drifted
/// on four consecutive additions (#426, #431, #435, #441) and stayed green
/// throughout, because the test that was supposed to catch it asserted a
/// literal — `known == "core.ping"` — which is exactly the stale value. A
/// test that reads the same hand-written string it is guarding cannot
/// notice it going stale (#443).
///
/// Reading the source is the only instrument available: Rust cannot
/// enumerate a match's arms at compile time, and enumerating them at run
/// time would mean the run-time registry this module's own doc comment
/// refuses.
fn routed_operations() -> Vec<String> {
    let source = include_str!("../dispatch.rs");
    let body = source
        .split_once("pub fn dispatch(")
        .expect("dispatch is defined in this file")
        .1
        .split_once("\n}")
        .expect("dispatch's body is brace-terminated")
        .0;
    let mut routed: Vec<String> = body
        .lines()
        .filter_map(|line| line.trim().strip_prefix('"'))
        .filter_map(|rest| rest.split_once("\" =>"))
        .map(|(name, _)| name.to_owned())
        .collect();
    routed.sort();
    routed
}

#[test]
fn the_operations_const_is_every_operation_dispatch_routes() {
    let mut declared: Vec<String> = OPERATIONS.iter().map(|op| (*op).to_owned()).collect();
    declared.sort();
    assert_eq!(
        declared,
        routed_operations(),
        "`OPERATIONS` and `dispatch`'s match disagree. Whichever was edited, \
         edit the other: the const is what a caller and the native fixture suite \
         enumerate, and the unknown-op error reports it to the user."
    );
}

#[test]
fn an_unknown_operation_names_the_ones_that_exist() {
    // A domain this build will never implement. The example used to be
    // `evidence.record`, which quoin#458 then made real — a test whose
    // subject can be promoted into the routing table is a test that
    // silently changes what it asserts, so the guard below is stated.
    const ABSENT: &str = "phlogiston.measure";
    assert!(
        !OPERATIONS.contains(&ABSENT),
        "`{ABSENT}` is routed now; this test needs an operation that is not"
    );
    let error = dispatch(ABSENT, &serde_json::json!({}), &Capabilities::none()).unwrap_err();
    assert_eq!(error.code, CoreErrorCode::UnknownOp);
    assert_eq!(error.outcome().code(), 3);
    assert_eq!(error.context["known"], OPERATIONS.join(","));
    // Not a tautology against the line above: this is the fact a user
    // reads off a mistyped operation, and `dispatch` builds the string
    // itself. The value is pinned by the drift guard, not by this test.
    assert!(
        error.context["known"].contains("assurance.build_case"),
        "known: {}",
        error.context["known"]
    );
}

/// The ceiling is on the READ. A stream past it is refused without the
/// document ever being parsed — which is why the payload below is
/// deliberately UNPARSEABLE: a `CORE_BAD_JSON` here would mean the parser
/// ran first, and the bound was a remark about an allocation that had
/// already happened (quoin#448 FND-003).
#[test]
fn a_stream_past_the_ceiling_is_refused_before_it_is_parsed() {
    let unparseable = format!(r#"{{"echo":"{}"#, "x".repeat(MAX_REQUEST_BYTES));
    assert!(unparseable.len() > MAX_REQUEST_BYTES);
    let error = read_request(unparseable.as_bytes()).unwrap_err();
    assert_eq!(error.code, CoreErrorCode::Refused);
    assert_eq!(error.outcome().code(), 2);
    assert_eq!(error.context["limit_bytes"], MAX_REQUEST_BYTES.to_string());
}

/// And the read really stops: the stream is not drained to find out how big
/// it was.
///
/// A counting reader rather than a comment. Without the cap this consumes
/// every byte offered — four times the ceiling here, unbounded in
/// production, where the offer is whatever a caller pipes in.
#[test]
fn the_stream_is_abandoned_at_the_ceiling_not_drained() {
    struct Counting<R> {
        inner: R,
        read: std::rc::Rc<std::cell::Cell<usize>>,
    }
    impl<R: Read> Read for Counting<R> {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            let n = self.inner.read(buffer)?;
            self.read.set(self.read.get() + n);
            Ok(n)
        }
    }

    let read = std::rc::Rc::new(std::cell::Cell::new(0));
    let offered = MAX_REQUEST_BYTES * 4;
    let error = read_request(Counting {
        inner: std::io::repeat(b'x').take(u64::try_from(offered).unwrap_or(u64::MAX)),
        read: std::rc::Rc::clone(&read),
    })
    .unwrap_err();

    assert_eq!(error.code, CoreErrorCode::Refused);
    assert_eq!(
        error.context["read_bytes"],
        (MAX_REQUEST_BYTES + 1).to_string()
    );
    assert!(
        read.get() <= MAX_REQUEST_BYTES + 1,
        "the reader consumed {} bytes of the {offered} offered; the ceiling \
         is on the read, not on what the read produced",
        read.get()
    );
}

/// Exactly at the ceiling is accepted. A bound that refuses its own limit
/// is a different bound from the one documented, and the boundary between
/// "accepted" and "refused" is the only part of a ceiling anyone meets.
#[test]
fn a_request_of_exactly_the_ceiling_is_accepted() {
    let padding = MAX_REQUEST_BYTES - r#"{"echo":""}"#.len();
    let at_the_limit = format!(r#"{{"echo":"{}"}}"#, "x".repeat(padding));
    assert_eq!(at_the_limit.len(), MAX_REQUEST_BYTES);
    let request = read_request(at_the_limit.as_bytes()).unwrap();
    assert_eq!(request["echo"].as_str().map(str::len), Some(padding));
}

/// One byte more is refused, and the refusal names what it read.
#[test]
fn one_byte_past_the_ceiling_is_refused() {
    let padding = MAX_REQUEST_BYTES - r#"{"echo":""}"#.len() + 1;
    let over = format!(r#"{{"echo":"{}"}}"#, "x".repeat(padding));
    assert_eq!(over.len(), MAX_REQUEST_BYTES + 1);
    let error = read_request(over.as_bytes()).unwrap_err();
    assert_eq!(error.code, CoreErrorCode::Refused);
    assert_eq!(
        error.context["read_bytes"],
        (MAX_REQUEST_BYTES + 1).to_string()
    );
}

/// Every domain bound outside `ops::evidence`, named one by one: a scan
/// would find whatever it found, and find nothing once the constants are
/// renamed.
const CENSUS_OF_THE_OLDER_DOMAINS: &[(&str, usize)] = &[
    (
        "ops::core::MAX_ECHO_BYTES",
        crate::ops::core::MAX_ECHO_BYTES,
    ),
    (
        "ops::assurance::MAX_BUILD_CASE_BYTES",
        crate::ops::assurance::MAX_BUILD_CASE_BYTES,
    ),
    (
        "ops::validators::MAX_RUN_REQUEST_BYTES",
        crate::ops::validators::MAX_RUN_REQUEST_BYTES,
    ),
    (
        "ops::assurance::MAX_OBLIGATION_ID_BYTES",
        crate::ops::assurance::MAX_OBLIGATION_ID_BYTES,
    ),
    (
        "ops::auditor::MAX_AUDITOR_REQUEST_BYTES",
        crate::ops::auditor::MAX_AUDITOR_REQUEST_BYTES,
    ),
    (
        "ops::catalog::MAX_LOAD_BYTES",
        crate::ops::catalog::MAX_LOAD_BYTES,
    ),
    (
        "ops::catalog::MAX_ROOT_BYTES",
        crate::ops::catalog::MAX_ROOT_BYTES,
    ),
    (
        "ops::graph::MAX_GRAPH_REQUEST_BYTES",
        crate::ops::graph::MAX_GRAPH_REQUEST_BYTES,
    ),
    (
        "ops::matrix::MAX_MATRIX_BUILD_BYTES",
        crate::ops::matrix::MAX_MATRIX_BUILD_BYTES,
    ),
    (
        "ops::graph::MAX_SCALAR_BYTES",
        crate::ops::graph::MAX_SCALAR_BYTES,
    ),
    (
        "ops::change_assurance::MAX_INTAKE_BYTES",
        crate::ops::change_assurance::MAX_INTAKE_BYTES,
    ),
    (
        "ops::change_assurance::MAX_RECEIPT_BYTES",
        crate::ops::change_assurance::MAX_RECEIPT_BYTES,
    ),
    (
        "ops::change_assurance::MAX_RECORD_DIGEST_BYTES",
        crate::ops::change_assurance::MAX_RECORD_DIGEST_BYTES,
    ),
    (
        "ops::change_assurance::MAX_RECOVER_BYTES",
        crate::ops::change_assurance::MAX_RECOVER_BYTES,
    ),
    (
        "ops::change_assurance::MAX_SCALAR_BYTES",
        crate::ops::change_assurance::MAX_SCALAR_BYTES,
    ),
    (
        "ops::change_assurance::MAX_SCHEMA_BYTES",
        crate::ops::change_assurance::MAX_SCHEMA_BYTES,
    ),
    (
        "ops::change_assurance::MAX_PREPARE_ATTESTATION_BYTES",
        crate::ops::change_assurance::MAX_PREPARE_ATTESTATION_BYTES,
    ),
    (
        "ops::change_assurance::MAX_WRITE_RECORD_BYTES",
        crate::ops::change_assurance::MAX_WRITE_RECORD_BYTES,
    ),
    (
        "ops::change_assurance::MAX_VERIFY_RECEIPT_BYTES",
        crate::ops::change_assurance::MAX_VERIFY_RECEIPT_BYTES,
    ),
    (
        "ops::completeness::MAX_MANIFEST_BYTES",
        crate::ops::completeness::MAX_MANIFEST_BYTES,
    ),
    (
        "ops::completeness::MAX_ASSESS_BUNDLE_BYTES",
        crate::ops::completeness::MAX_ASSESS_BUNDLE_BYTES,
    ),
    (
        "ops::config::MAX_GIT_CONFIG_BYTES",
        crate::ops::config::MAX_GIT_CONFIG_BYTES,
    ),
    (
        "ops::config::MAX_CONFIG_LAYER_BYTES",
        crate::ops::config::MAX_CONFIG_LAYER_BYTES,
    ),
    (
        "ops::config::MAX_SCALAR_BYTES",
        crate::ops::config::MAX_SCALAR_BYTES,
    ),
    (
        "ops::modules::MAX_MANIFEST_BYTES",
        crate::ops::modules::MAX_MANIFEST_BYTES,
    ),
    (
        "ops::modules::MAX_SCALAR_BYTES",
        crate::ops::modules::MAX_SCALAR_BYTES,
    ),
    (
        "ops::modules::MAX_SOURCE_ARG_BYTES",
        crate::ops::modules::MAX_SOURCE_ARG_BYTES,
    ),
    (
        "ops::quire::MAX_REQUEST_BYTES",
        crate::ops::quire::MAX_REQUEST_BYTES,
    ),
    (
        "ops::quire::MAX_SCALAR_BYTES",
        crate::ops::quire::MAX_SCALAR_BYTES,
    ),
    (
        "ops::semantic::MAX_READ_BLOCKS_BYTES",
        crate::ops::semantic::MAX_READ_BLOCKS_BYTES,
    ),
    (
        "ops::semantic::MAX_SCALAR_BYTES",
        crate::ops::semantic::MAX_SCALAR_BYTES,
    ),
    (
        "ops::semantic::MAX_SWEEP_CORPUS_BYTES",
        crate::ops::semantic::MAX_SWEEP_CORPUS_BYTES,
    ),
];

/// Every `ops::evidence` bound, named one by one (quoin#458). Its own
/// function only because the two together exceed the length lint; the two
/// are concatenated below and compared against `declared_domain_bounds()`
/// as one population, so a bound listed in neither still fails.
fn census_of_the_evidence_domain() -> Vec<(&'static str, usize)> {
    vec![
        (
            "ops::evidence::MAX_ASSURANCE_RECORD_BYTES",
            crate::ops::evidence::MAX_ASSURANCE_RECORD_BYTES,
        ),
        (
            "ops::evidence::MAX_SCENARIO_RECORD_BYTES",
            crate::ops::evidence::MAX_SCENARIO_RECORD_BYTES,
        ),
        (
            "ops::evidence::MAX_AUDIT_INPUTS_BYTES",
            crate::ops::evidence::MAX_AUDIT_INPUTS_BYTES,
        ),
        (
            "ops::evidence::MAX_DISCHARGE_BYTES",
            crate::ops::evidence::MAX_DISCHARGE_BYTES,
        ),
        (
            "ops::evidence::MAX_GC_BYTES",
            crate::ops::evidence::MAX_GC_BYTES,
        ),
        (
            "ops::evidence::MAX_INSPECT_MOCKS_BYTES",
            crate::ops::evidence::MAX_INSPECT_MOCKS_BYTES,
        ),
        (
            "ops::evidence::MAX_PARSE_LINEAGE_BYTES",
            crate::ops::evidence::MAX_PARSE_LINEAGE_BYTES,
        ),
        (
            "ops::evidence::MAX_PARSE_POLICY_BYTES",
            crate::ops::evidence::MAX_PARSE_POLICY_BYTES,
        ),
        (
            "ops::evidence::MAX_PARSE_RESULTS_BYTES",
            crate::ops::evidence::MAX_PARSE_RESULTS_BYTES,
        ),
        (
            "ops::evidence::MAX_RECORD_BYTES",
            crate::ops::evidence::MAX_RECORD_BYTES,
        ),
        (
            "ops::evidence::MAX_SCALAR_BYTES",
            crate::ops::evidence::MAX_SCALAR_BYTES,
        ),
        (
            "ops::evidence::MAX_STORE_FACTS_BYTES",
            crate::ops::evidence::MAX_STORE_FACTS_BYTES,
        ),
        (
            "ops::evidence::MAX_TRUST_ASSESSMENTS_BYTES",
            crate::ops::evidence::MAX_TRUST_ASSESSMENTS_BYTES,
        ),
        (
            "ops::evidence::MAX_TRUST_DECISION_BYTES",
            crate::ops::evidence::MAX_TRUST_DECISION_BYTES,
        ),
    ]
}

/// Every `ops::measurement` bound, named one by one (quoin#478). Its own
/// function for the same reason the evidence census is: the three together
/// exceed the length lint, and all three are concatenated below and
/// compared against `declared_domain_bounds()` as ONE population, so a
/// bound listed in none of them still fails.
///
/// A file-size bound in `quoin-store` is deliberately absent: it is a
/// FILE bound, not an `ops::*` request bound, and
/// `tc_412` compares this census against a scan of the `ops` sources only.
/// Listing it here would make the two disagree by construction.
fn census_of_the_measurement_domain() -> Vec<(&'static str, usize)> {
    vec![
        (
            "ops::measurement::MAX_ASSURANCE_DOCUMENT_BYTES",
            crate::ops::measurement::MAX_ASSURANCE_DOCUMENT_BYTES,
        ),
        (
            "ops::measurement::MAX_COLLECTION_BYTES",
            crate::ops::measurement::MAX_COLLECTION_BYTES,
        ),
        (
            "ops::measurement::MAX_GRAPH_MAPPING_BYTES",
            crate::ops::measurement::MAX_GRAPH_MAPPING_BYTES,
        ),
        (
            "ops::measurement::MAX_INTERVENTION_RECORD_BYTES",
            crate::ops::measurement::MAX_INTERVENTION_RECORD_BYTES,
        ),
        (
            "ops::measurement::MAX_METRIC_NAME_BYTES",
            crate::ops::measurement::MAX_METRIC_NAME_BYTES,
        ),
        (
            "ops::measurement::MAX_OPERATIONAL_RECORD_BYTES",
            crate::ops::measurement::MAX_OPERATIONAL_RECORD_BYTES,
        ),
        (
            "ops::measurement::MAX_PORTFOLIO_ROOTS_BYTES",
            crate::ops::measurement::MAX_PORTFOLIO_ROOTS_BYTES,
        ),
        (
            "ops::measurement::MAX_PRODUCER_DEFINITION_BYTES",
            crate::ops::measurement::MAX_PRODUCER_DEFINITION_BYTES,
        ),
        (
            "ops::measurement::MAX_RECORD_ID_BYTES",
            crate::ops::measurement::MAX_RECORD_ID_BYTES,
        ),
        (
            "ops::measurement::MAX_RETAINED_EXPORT_BYTES",
            crate::ops::measurement::MAX_RETAINED_EXPORT_BYTES,
        ),
        (
            "ops::measurement::MAX_VERIFY_REQUEST_BYTES",
            crate::ops::measurement::MAX_VERIFY_REQUEST_BYTES,
        ),
        (
            "ops::measurement::MAX_WORKFLOW_YAML_BYTES",
            crate::ops::measurement::MAX_WORKFLOW_YAML_BYTES,
        ),
    ]
}

/// The transport ceiling stays strictly above every domain bound.
///
/// Each `ops::*` bound refuses with the `op` that refused and the quantity
/// it measured. The transport refusal carries neither, because it fires
/// before the bytes are a request at all. If this ceiling ever met or fell
/// below a domain bound, the transport verdict would shadow that domain's
/// and the domain's own refusal would become unreachable through the
/// binary — compiled, unit-tested, and dead. The failure mode is silent,
/// so it is asserted rather than commented (quoin#448 FND-003).
///
/// `ops::validators::MAX_RUN_REQUEST_BYTES` is the one this is really for:
/// it is the largest request the boundary accepts, and the transport
/// ceiling above it is what stops the read before that bound can be
/// measured on something unbounded.
#[test]
fn tc_412_the_transport_ceiling_stays_above_every_domain_bound() {
    let domain_bounds: Vec<(&str, usize)> = CENSUS_OF_THE_OLDER_DOMAINS
        .iter()
        .copied()
        .chain(census_of_the_evidence_domain())
        .chain(census_of_the_measurement_domain())
        .collect();
    // The census is hand-written, so it is checked against the `ops`
    // sources rather than against itself. A literal count (`len() == 6`)
    // is the quoin#443 shape: the two functions above are the census, so
    // the assertion would re-read the thing it guards, and a bound added
    // to a module but never listed there would leave the inequality below
    // unevaluated while the test stayed green.
    let mut listed: Vec<String> = domain_bounds
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect();
    listed.sort();
    let mut declared = declared_domain_bounds();
    declared.sort();
    assert_eq!(
        listed, declared,
        "a domain bound was added or removed without this census moving"
    );
    for (name, bound) in domain_bounds {
        assert!(
            bound < MAX_REQUEST_BYTES,
            "{name} is {bound} and the transport ceiling is {MAX_REQUEST_BYTES}: \
             a request that hits {name} would be refused by the transport first, \
             and {name}'s refusal could no longer be reached"
        );
    }
}

/// A stream that is not UTF-8 is a malformed request, not a failing stream:
/// the boundary speaks JSON text and a caller cannot act differently on
/// "bad bytes" than on "bad syntax".
#[test]
fn a_non_utf8_stream_is_bad_json_not_io() {
    let error = read_request(&[0xff, 0xfe, 0x00][..]).unwrap_err();
    assert_eq!(error.code, CoreErrorCode::BadJson);
    assert_eq!(error.outcome().code(), 3);
}

#[test]
fn empty_stdin_is_the_empty_request() {
    assert_eq!(parse_request("   \n ").unwrap(), serde_json::json!({}));
}

#[test]
fn malformed_json_is_invalid_not_internal() {
    let error = parse_request("{oops").unwrap_err();
    assert_eq!(error.code, CoreErrorCode::BadJson);
    assert_eq!(error.outcome().code(), 3);
}

#[test]
fn a_non_object_request_is_refused_by_type() {
    let error = parse_request("[1,2]").unwrap_err();
    assert_eq!(error.context["observed_type"], "array");
}

#[test]
fn usage_needs_exactly_one_dotted_argument() {
    assert_eq!(
        parse_operation(&["core.ping".to_owned()]).unwrap(),
        "core.ping"
    );
    for bad in [
        vec![],
        vec!["core".to_owned()],
        vec!["core.ping".to_owned(), "x".to_owned()],
    ] {
        assert_eq!(
            parse_operation(&bad).unwrap_err().code,
            CoreErrorCode::BadUsage
        );
    }
    assert_eq!(
        parse_operation(&["core.".to_owned()]).unwrap_err().code,
        CoreErrorCode::BadUsage
    );
}
