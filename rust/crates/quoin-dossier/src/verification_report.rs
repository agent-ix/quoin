// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Verification report generator (FR-196, PLAT-1617).
//!
//! A rendering of the evidence-backed matrix (`quoin_assurance::matrix`,
//! FR-115). Evidence Status, method and bindings are copied from the matrix
//! and never recomputed. A failed run is recognised only by the typed
//! `failed-run` cause on a `stale-evidence` finding (PLAT-1760), never by
//! reading a summary.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::sync::Arc;

use quoin_assurance::matrix::{
    EvidenceStatus, MatrixCriterion, MatrixOutput, MatrixRequirement, RunIdentity, StaleCause,
};
use quoin_quire::records::Document;
use serde_json::Value;

use crate::error::SourceFailure;
use crate::gap::{Gap, GapKind};
use crate::markdown::{cell, line, unusable_input};
use crate::order::dossier_cmp;
use crate::registry::{DocumentSpec, Generated, Generator, Inputs};
use crate::sources::MatrixSource;
use crate::srs::{REQUIREMENT_TYPES, criteria_of, criticality_of, title_of};

/// The finding kind the matrix reports for stale evidence.
const STALE_EVIDENCE: &str = "stale-evidence";

/// The analysis item status of a qualified retained record.
const QUALIFIED_RECORD: &str = "qualified-record";

/// What the report reads where the source states no value.
const NOT_RECORDED: &str = "not recorded";

/// Every status the summary counts, in the order FR-115 lists them.
const STATUSES: [EvidenceStatus; 5] = [
    EvidenceStatus::Bound,
    EvidenceStatus::Stale,
    EvidenceStatus::Suspect,
    EvidenceStatus::Undischarged,
    EvidenceStatus::NoRunEvidence,
];

/// The verification report generator.
pub(crate) struct VerificationReportGenerator {
    matrix: Arc<dyn MatrixSource>,
}

impl VerificationReportGenerator {
    /// A generator reading its matrix from `matrix`.
    pub(crate) fn new(matrix: Arc<dyn MatrixSource>) -> Self {
        Self { matrix }
    }
}

impl Generator for VerificationReportGenerator {
    fn spec(&self) -> DocumentSpec {
        DocumentSpec {
            kind: "verification-report",
            id: "DOS-VR",
            title: "Verification report",
            needs_as_of: false,
        }
    }

    fn generate(&self, inputs: &Inputs) -> Result<Generated, SourceFailure> {
        let matrix = self.matrix.matrix(inputs)?;

        let modules = inputs
            .modules
            .iter()
            .map(quoin_quire::ids::ModuleRoot::open)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| SourceFailure {
                source: "modules".to_owned(),
                detail: format!("failed to open module: {error}"),
            })?;
        let selection = if modules.is_empty() {
            quoin_quire::ModuleSelection::ScopeOrAmbient
        } else {
            quoin_quire::ModuleSelection::Closed(modules)
        };
        let corpus = quoin_quire::records::read(&inputs.scope, &selection, &REQUIREMENT_TYPES)
            .map_err(|error| SourceFailure {
                source: "requirements".to_owned(),
                detail: format!("failed to read requirement documents: {error}"),
            })?;
        if let Some(failure) = unusable_input("requirements", &corpus) {
            return Err(failure);
        }

        Ok(render(&matrix, &corpus.documents))
    }
}

/// The report over `matrix` and the requirement `documents` that supply titles
/// and criticality.
fn render(matrix: &MatrixOutput, documents: &[Document]) -> Generated {
    let mut gaps = Vec::new();
    let mut body = String::from("# Verification report\n\n## Summary\n\n");

    let mut counts: BTreeMap<EvidenceStatus, usize> = BTreeMap::new();
    for criterion in matrix.requirements.iter().flat_map(|req| &req.criteria) {
        *counts.entry(criterion.evidence_status).or_default() += 1;
    }
    body.push_str("| Evidence status | Criteria |\n|-----------------|----------|\n");
    for status in STATUSES {
        let _ = writeln!(
            body,
            "| {} | {} |",
            status.as_str(),
            counts.get(&status).copied().unwrap_or_default()
        );
    }
    body.push('\n');

    if let Some(reason) = &matrix.reason {
        let _ = write!(body, "{}\n\n", line(reason));
        gaps.push(Gap::new(
            GapKind::NoRequirements,
            "this document",
            "the matrix holds no criterion",
        ));
    }

    let requirement_documents: BTreeMap<&str, &Document> = documents
        .iter()
        .filter(|doc| REQUIREMENT_TYPES.contains(&doc.kind.as_str()))
        .map(|doc| (doc.id.as_str(), doc))
        .collect();
    let matrix_requirements: BTreeMap<&str, &MatrixRequirement> = matrix
        .requirements
        .iter()
        .map(|req| (req.id.as_str(), req))
        .collect();
    let mut ids: Vec<&str> = requirement_documents
        .keys()
        .chain(matrix_requirements.keys())
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    ids.sort_by(|a, b| dossier_cmp(a, b));

    for id in ids {
        let document = requirement_documents.get(id).copied();
        let criteria = matrix_requirements
            .get(id)
            .map_or(&[][..], |req| req.criteria.as_slice());
        render_requirement(&mut body, &mut gaps, id, document, criteria);
    }

    Generated {
        body,
        document_version: None,
        gaps,
    }
}

/// One requirement's section, its rows and its gaps.
fn render_requirement(
    body: &mut String,
    gaps: &mut Vec<Gap>,
    id: &str,
    document: Option<&Document>,
    criteria: &[MatrixCriterion],
) {
    let title = document.map(title_of).filter(|title| !title.is_empty());
    let _ = write!(
        body,
        "## {}: {}\n\n",
        line(id),
        line(title.as_deref().unwrap_or("not stated"))
    );
    let criticality = document.and_then(criticality_of);
    let _ = write!(
        body,
        "Criticality: {}\n\n",
        line(criticality.as_deref().unwrap_or("unassigned"))
    );
    if criticality.is_none() {
        gaps.push(
            Gap::new(GapKind::CriticalityUnassigned, id, "has no criticality").for_requirement(id),
        );
    }

    let stated = document.map(criteria_of).unwrap_or_default();
    if criteria.is_empty() {
        if stated.is_empty() {
            gaps.push(
                Gap::new(
                    GapKind::NoAcceptanceCriteria,
                    id,
                    "has no acceptance criterion",
                )
                .for_requirement(id),
            );
        }
        // The matrix is silent on criteria the document states, which is not
        // the same as the document stating none.
        for (criterion_id, _) in &stated {
            gaps.push(
                Gap::new(
                    GapKind::NoEvidence,
                    criterion_id,
                    "the matrix reports no row for this criterion",
                )
                .for_requirement(id)
                .for_criterion(criterion_id),
            );
        }
        return;
    }

    body.push_str(
        "| Criterion | Method | Evidence | Source findings | Run identity | Evidence status |\n\
         |-----------|--------|----------|-----------------|--------------|-----------------|\n",
    );
    for criterion in criteria {
        let _ = writeln!(
            body,
            "| {} | {} | {} | {} | {} | {} |",
            cell(&criterion.id),
            cell(criterion.method.as_deref().unwrap_or("not stated")),
            cell(&evidence_cell(criterion)),
            cell(&findings_cell(criterion)),
            cell(&run_identity_cell(criterion)),
            criterion.evidence_status.as_str()
        );
        gaps.extend(criterion_gaps(id, criterion));
    }
    body.push('\n');
}

/// The traced tests, suites and retained records the matrix supplies.
fn evidence_cell(criterion: &MatrixCriterion) -> String {
    let mut items = Vec::new();
    for binder in &criterion.binders {
        let ignored = if binder.ignored { ", ignored" } else { "" };
        items.push(format!(
            "{} ({}{ignored}, {}:{})",
            binder.qualified_name, binder.kind, binder.path, binder.line
        ));
    }
    for binding in &criterion.evidence_detail.bindings {
        items.push(format!("suite {}", binding.suite));
    }
    for binder in retained_records(criterion) {
        let locator = |key: &str| {
            binder
                .get("locator")
                .and_then(|locator| locator.get(key))
                .and_then(Value::as_str)
        };
        let record = locator("record").unwrap_or("unknown");
        let mut item = format!("record {record}");
        let detail: Vec<&str> = ["run", "entry"].into_iter().filter_map(locator).collect();
        if !detail.is_empty() {
            let _ = write!(item, " ({})", detail.join(", "));
        }
        items.push(item);
    }
    joined(&items)
}

/// The retained record binders the matrix attaches to a criterion.
fn retained_records(criterion: &MatrixCriterion) -> impl Iterator<Item = &Value> {
    criterion
        .analysis_evidence
        .iter()
        .flatten()
        .filter(|item| item.get("status").and_then(Value::as_str) == Some(QUALIFIED_RECORD))
        .filter_map(|item| item.get("record_binder"))
}

/// The auditor's findings, its unevaluated checks and the non-qualified
/// analysis results, by kind and typed cause. A summary is prose and is not
/// copied.
fn findings_cell(criterion: &MatrixCriterion) -> String {
    let mut items = Vec::new();
    for finding in &criterion.evidence_detail.findings {
        items.push(match finding.cause.and_then(StaleCause::as_str) {
            Some(cause) => format!("{} (cause {cause})", finding.kind),
            None => finding.kind.clone(),
        });
    }
    for check in &criterion.evidence_detail.unevaluated {
        items.push(format!("unevaluated {check}"));
    }
    for item in criterion.analysis_evidence.iter().flatten() {
        let status = item.get("status").and_then(Value::as_str);
        if let Some(status) = status.filter(|status| *status != QUALIFIED_RECORD) {
            items.push(format!("analysis {status}"));
        }
    }
    joined(&items)
}

/// The run identity the matrix selects for each suite bound to a criterion,
/// every value as recorded and a value the source does not state as
/// `not recorded`. The tally counts recorded outcome values; no outcome is
/// computed from it.
fn run_identity_cell(criterion: &MatrixCriterion) -> String {
    let bindings = &criterion.evidence_detail.bindings;
    if bindings.is_empty() {
        return NOT_RECORDED.to_owned();
    }
    let items: Vec<String> = bindings
        .iter()
        .map(|binding| match &binding.run {
            Some(run) => format!("suite {}: {}", binding.suite, run_values(run)),
            None => format!("suite {}: {NOT_RECORDED}", binding.suite),
        })
        .collect();
    items.join("; ")
}

fn run_values(run: &RunIdentity) -> String {
    let stated = |value: &str| {
        if value.is_empty() {
            NOT_RECORDED
        } else {
            value
        }
        .to_owned()
    };
    let tally = &run.outcomes;
    format!(
        "timestamp {}, tool {}, file {}, outcomes pass {} fail {} skip {} error {}, source revision {}",
        stated(&run.timestamp),
        stated(&run.tool),
        stated(&run.file),
        tally.pass,
        tally.fail,
        tally.skip,
        tally.error,
        run.source_revision
            .as_deref()
            .map_or(NOT_RECORDED, |revision| revision),
    )
}

fn joined(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_owned()
    } else {
        items.join("; ")
    }
}

/// The gaps one criterion's matrix status requires; none when it is `bound`
/// and backed.
fn criterion_gaps(requirement: &str, criterion: &MatrixCriterion) -> Vec<Gap> {
    let id = criterion.id.as_str();
    let gap = |kind, detail: &str| {
        Gap::new(kind, id, detail)
            .for_requirement(requirement)
            .for_criterion(id)
    };
    let mut gaps = Vec::new();

    let failed = criterion.evidence_detail.findings.iter().any(|finding| {
        finding.kind == STALE_EVIDENCE && finding.cause == Some(StaleCause::FailedRun)
    });
    if failed {
        gaps.push(gap(
            GapKind::EvidenceFailed,
            "a source finding carries the typed cause failed-run",
        ));
    }
    match criterion.evidence_status {
        EvidenceStatus::Stale if !failed => gaps.push(gap(
            GapKind::EvidenceStale,
            "the evidence is stale and no failed run is recorded",
        )),
        EvidenceStatus::Suspect => gaps.push(gap(
            GapKind::EvidenceSuspect,
            "the auditor distrusts the binding",
        )),
        EvidenceStatus::Undischarged | EvidenceStatus::NoRunEvidence => gaps.push(gap(
            GapKind::NoEvidence,
            "no trusted run evidence discharges this criterion",
        )),
        EvidenceStatus::Bound | EvidenceStatus::Stale => {}
    }

    let backed = !criterion.binders.is_empty() || retained_records(criterion).next().is_some();
    if !backed {
        gaps.push(gap(
            GapKind::UnbackedCriterion,
            "no traced test or retained record binder backs this criterion",
        ));
    }
    gaps
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use serde_json::json;

    use super::*;
    use ix_trace_rs::trace;

    fn binder(name: &str) -> Value {
        json!({
            "path": "tests/t.rs", "line": 3, "column": 1,
            "qualified_name": name, "kind": "function",
        })
    }

    fn criterion(id: &str, status: &str, binders: &[Value], findings: &[Value]) -> Value {
        json!({
            "id": id,
            "method": "Test",
            "static_status": "tagged",
            "binders": binders,
            "evidence_status": status,
            "evidence_detail": {
                "findings": findings,
                "bindings": [],
                "unevaluated": [],
            },
        })
    }

    fn finding(kind: &str, summary: &str, cause: Option<&str>) -> Value {
        let mut value = json!({ "kind": kind, "summary": summary });
        if let Some(cause) = cause {
            value["cause"] = json!(cause);
        }
        value
    }

    fn matrix(requirements: &[(&str, Vec<Value>)]) -> MatrixOutput {
        let requirements: Vec<Value> = requirements
            .iter()
            .map(|(id, criteria)| json!({ "id": id, "criteria": criteria }))
            .collect();
        serde_json::from_value(json!({ "requirements": requirements })).unwrap()
    }

    /// The six states of FR-196-AC-1, in source order.
    fn six() -> MatrixOutput {
        matrix(&[(
            "FR-1",
            vec![
                criterion("FR-1-AC-1", "bound", &[binder("t1")], &[]),
                criterion(
                    "FR-1-AC-2",
                    "stale",
                    &[binder("t2")],
                    &[finding(
                        STALE_EVIDENCE,
                        "the newest run passed once",
                        Some("missing-run"),
                    )],
                ),
                criterion(
                    "FR-1-AC-3",
                    "suspect",
                    &[binder("t3")],
                    &[finding("vacuous-evidence", "verified nothing", None)],
                ),
                criterion("FR-1-AC-4", "undischarged", &[binder("t4")], &[]),
                criterion("FR-1-AC-5", "no run evidence", &[binder("t5")], &[]),
                criterion(
                    "FR-1-AC-6",
                    "stale",
                    &[binder("t6")],
                    &[finding(STALE_EVIDENCE, "ok", Some("failed-run"))],
                ),
            ],
        )])
    }

    fn gap_pairs(generated: &Generated) -> Vec<(&'static str, String)> {
        let mut gaps = generated.gaps.clone();
        crate::gap::sort_gaps(&mut gaps);
        gaps.iter()
            .map(|gap| (gap.gap_kind.as_str(), gap.subject.clone()))
            .collect()
    }

    fn row<'a>(generated: &'a Generated, id: &str) -> &'a str {
        generated
            .body
            .lines()
            .find(|row| row.starts_with(&format!("| {id} |")))
            .unwrap()
    }

    #[trace("FR-196-AC-1")]
    #[test]
    fn six_states_give_six_rows_with_the_matrix_status_and_method() {
        let source = six();
        let generated = render(&source, &[]);
        let rows: Vec<&str> = generated
            .body
            .lines()
            .filter(|row| row.starts_with("| FR-1-AC-"))
            .collect();
        assert_eq!(rows.len(), 6);
        let criteria = &source.requirements[0].criteria;
        for (rendered, expected) in rows.iter().zip(criteria) {
            assert!(rendered.starts_with(&format!("| {} | Test |", expected.id)));
            assert!(rendered.ends_with(&format!("| {} |", expected.evidence_status.as_str())));
        }
    }

    #[trace("FR-196-AC-2")]
    #[test]
    fn summary_counts_the_rendered_rows_and_states_no_percentage() {
        let generated = render(&six(), &[]);
        for expected in [
            "| bound | 1 |",
            "| stale | 2 |",
            "| suspect | 1 |",
            "| undischarged | 1 |",
            "| no run evidence | 1 |",
        ] {
            assert!(generated.body.contains(expected), "{expected}");
        }
        assert!(!generated.body.contains('%'));
    }

    #[trace("FR-196-AC-3")]
    #[test]
    fn a_typed_failed_run_is_evidence_failed_whatever_the_summary_says() {
        let generated = render(&six(), &[]);
        assert!(gap_pairs(&generated).contains(&("evidence_failed", "FR-1-AC-6".to_owned())));
        let failed = row(&generated, "FR-1-AC-6");
        assert!(failed.contains("cause failed-run"));

        // The summary is not read: the same finding with a different summary
        // classifies identically, and a failure-sounding summary on a
        // missing-run cause is not a failed run.
        let reworded = matrix(&[(
            "FR-1",
            vec![
                criterion(
                    "FR-1-AC-1",
                    "stale",
                    &[binder("t")],
                    &[finding(
                        STALE_EVIDENCE,
                        "the test failed",
                        Some("missing-run"),
                    )],
                ),
                criterion(
                    "FR-1-AC-2",
                    "stale",
                    &[binder("t")],
                    &[finding(
                        STALE_EVIDENCE,
                        "completely different",
                        Some("failed-run"),
                    )],
                ),
            ],
        )]);
        let pairs = gap_pairs(&render(&reworded, &[]));
        assert_eq!(
            pairs,
            [
                ("criticality_unassigned", "FR-1".to_owned()),
                ("evidence_stale", "FR-1-AC-1".to_owned()),
                ("evidence_failed", "FR-1-AC-2".to_owned()),
            ]
        );
    }

    #[trace("FR-196-AC-3")]
    #[test]
    fn no_failing_stale_or_suspect_row_claims_a_pass() {
        let generated = render(&six(), &[]);
        for id in ["FR-1-AC-2", "FR-1-AC-3", "FR-1-AC-6"] {
            let row = row(&generated, id).to_lowercase();
            assert!(!row.contains("pass"), "{row}");
            assert!(!row.contains("verified"), "{row}");
        }
    }

    #[trace("FR-196-AC-4")]
    #[test]
    fn stale_without_a_failed_run_and_suspect_have_their_own_gaps() {
        let pairs = gap_pairs(&render(&six(), &[]));
        assert!(pairs.contains(&("evidence_stale", "FR-1-AC-2".to_owned())));
        assert!(pairs.contains(&("evidence_suspect", "FR-1-AC-3".to_owned())));
        assert!(!pairs.contains(&("evidence_failed", "FR-1-AC-2".to_owned())));
        assert!(!pairs.contains(&("evidence_failed", "FR-1-AC-3".to_owned())));
        assert!(!pairs.contains(&("evidence_stale", "FR-1-AC-6".to_owned())));
    }

    #[trace("FR-196-AC-5")]
    #[test]
    fn undischarged_and_no_run_evidence_are_no_evidence_and_bound_has_no_gap() {
        let pairs = gap_pairs(&render(&six(), &[]));
        assert!(pairs.contains(&("no_evidence", "FR-1-AC-4".to_owned())));
        assert!(pairs.contains(&("no_evidence", "FR-1-AC-5".to_owned())));
        assert!(!pairs.iter().any(|(_, subject)| subject == "FR-1-AC-1"));
        assert!(!pairs.iter().any(|(kind, _)| *kind == "unbacked_criterion"));
    }

    #[trace("FR-196-AC-5")]
    #[test]
    fn a_criterion_without_a_binder_is_unbacked_as_well() {
        let source = matrix(&[(
            "FR-1",
            vec![criterion("FR-1-AC-1", "undischarged", &[], &[])],
        )]);
        assert_eq!(
            gap_pairs(&render(&source, &[])),
            [
                ("criticality_unassigned", "FR-1".to_owned()),
                ("unbacked_criterion", "FR-1-AC-1".to_owned()),
                ("no_evidence", "FR-1-AC-1".to_owned()),
            ]
        );
    }

    #[trace("FR-196-AC-6")]
    #[test]
    fn a_bound_criterion_backed_only_by_a_retained_record_shows_it_and_has_no_gap() {
        let mut inspected = criterion("FR-1-AC-1", "bound", &[], &[]);
        inspected["analysis_evidence"] = json!([{
            "status": "qualified-record",
            "record_binder": {
                "locator": {"record": "runs/INS/a.json", "run": "INS@a", "entry": "review::x"},
            },
        }]);
        let generated = render(&matrix(&[("FR-1", vec![inspected])]), &[]);
        assert!(row(&generated, "FR-1-AC-1").contains("record runs/INS/a.json (INS@a, review::x)"));
        assert_eq!(
            gap_pairs(&generated),
            [("criticality_unassigned", "FR-1".to_owned())]
        );
    }

    #[trace("FR-196-AC-6")]
    #[test]
    fn a_retained_record_that_is_not_bound_still_gets_its_gap() {
        let mut inspected = criterion("FR-1-AC-1", "undischarged", &[], &[]);
        inspected["analysis_evidence"] = json!([{
            "status": "qualified-record",
            "record_binder": {"locator": {"record": "runs/INS/a.json"}},
        }]);
        let generated = render(&matrix(&[("FR-1", vec![inspected])]), &[]);
        assert_eq!(
            gap_pairs(&generated),
            [
                ("criticality_unassigned", "FR-1".to_owned()),
                ("no_evidence", "FR-1-AC-1".to_owned()),
            ]
        );
    }

    #[trace("FR-196-AC-7")]
    #[test]
    fn a_requirement_without_criteria_stays_listed_with_no_row() {
        let source = matrix(&[("FR-2", vec![])]);
        let generated = render(&source, &[]);
        assert!(generated.body.contains("## FR-2: not stated"));
        assert!(generated.body.contains("Criticality: unassigned"));
        assert!(!generated.body.contains("| Criterion |"));
        assert_eq!(
            gap_pairs(&generated),
            [
                ("no_acceptance_criteria", "FR-2".to_owned()),
                ("criticality_unassigned", "FR-2".to_owned()),
            ]
        );
    }

    #[trace("FR-196-AC-3")]
    #[test]
    fn an_invalid_cause_has_no_invented_spelling() {
        let source = matrix(&[(
            "FR-1",
            vec![criterion(
                "FR-1-AC-1",
                "stale",
                &[binder("t")],
                &[finding(STALE_EVIDENCE, "x", Some("unknown"))],
            )],
        )]);
        assert_eq!(
            findings_cell(&source.requirements[0].criteria[0]),
            "stale-evidence"
        );
    }

    #[trace("FR-196-AC-8")]
    #[test]
    fn an_empty_matrix_shows_its_reason_and_is_deterministic() {
        let empty = matrix(&[]);
        let empty = MatrixOutput {
            reason: Some(quoin_assurance::matrix::EMPTY_REASON.to_owned()),
            ..empty
        };
        let first = render(&empty, &[]);
        assert!(first.body.contains(quoin_assurance::matrix::EMPTY_REASON));
        assert_eq!(
            gap_pairs(&first),
            [("no_requirements", "this document".to_owned())]
        );
        assert_eq!(first, render(&empty, &[]));
        assert_eq!(render(&six(), &[]), render(&six(), &[]));
    }
}
