// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `evidence` operations that WRITE a record.
//!
//! Transcribing a suite run or a finding-shaped scan, a trust decision, a mock
//! inspection and the two content-addressed assurance families. Each one
//! acquires the store through the granted [`EvidenceHost`] and hands back the
//! absolute path of what it wrote.

use std::path::Path;

use quoin_evidence::adapters::{AdapterSelection, select_adapter, select_finding_adapter};
use quoin_evidence::assurance_records::{
    write_experiment_record, write_operational_evidence_record,
};
use quoin_evidence::mock_inspection::inspect_mock_injections;
use quoin_evidence::record::{RecordRequest as StoreRecordRequest, record_run};
use quoin_evidence::store::{
    bind, read_bindings, write_bindings, write_mock_inspection, write_scan, write_trust_decision,
};
use quoin_evidence::trust::assess_trust;
use quoin_evidence::types::{
    Binding, FindingRecord, MockInspectionRecord, STORE_SCHEMA_VERSION, TrustDecision,
};
use quoin_evidence::{Commit, ObligationId, StatementHash, SuiteId};

use crate::capabilities::{Capabilities, EvidenceHost};
use crate::error::{CoreError, CoreErrorCode};
use crate::protocol::Response;

use super::documents::{tool_name, unrepresented_views};
use super::taxonomy::map_error;
use super::wire::{
    AssuranceRecordPayload, AssuranceRecordRequest, InspectMocksPayload, InspectMocksRequest,
    MAX_ASSURANCE_RECORD_BYTES, MAX_INSPECT_MOCKS_BYTES, MAX_RECORD_BYTES,
    MAX_TRUST_DECISION_BYTES, RecordPayload, RecordRequest, TrustDecisionPayload,
    TrustDecisionRequest,
};
use super::{absolute, bound, check_scalar, host, parse, value};

/// Answer an `evidence.record`.
///
/// The branch between a run record and a finding-shaped scan record is taken
/// BEFORE anything is parsed, from the adapter registry alone. Letting a scan
/// fall through to the run path would write it into `runs/` and lose the
/// clean-versus-unrun distinction at the point of intake — silently, and
/// permanently for that commit (FR-034).
///
/// # Errors
///
/// - [`CoreErrorCode::Refused`] when the request exceeds [`MAX_RECORD_BYTES`],
///   when the result document names a tool that is not `--tool`, or when the
///   store cannot be written.
/// - [`CoreErrorCode::BadRequest`] when `adapter` names nothing, or the
///   producer document does not read.
/// - [`CoreErrorCode::Io`] when no evidence host was granted.
pub fn record(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    let op = "evidence.record";
    bound(request, op, MAX_RECORD_BYTES)?;
    let request: RecordRequest = parse(request, op)?;
    for (field, text) in [
        ("repo", request.repo.as_str()),
        ("suite", request.suite.as_str()),
        ("commit", request.commit.as_str()),
        ("tool", request.tool.as_str()),
        ("timestamp", request.timestamp.as_str()),
        ("adapter", request.adapter.as_deref().unwrap_or("")),
        ("kind", request.kind.as_deref().unwrap_or("")),
    ] {
        check_scalar(op, field, text)?;
    }

    let selection = AdapterSelection {
        adapter: request.adapter.as_deref(),
        tool: Some(request.tool.as_str()),
    };
    let host = host(capabilities, op)?;
    // Finding-shaped first, and that ORDER is the rule: `select_adapter` has a
    // default, so a scan that fell through here would be parsed as entries and
    // written into `runs/`, losing the clean-versus-unrun distinction at intake.
    let payload = if let Some(adapter) = select_finding_adapter(selection) {
        record_scan(&request, adapter, host, op)?
    } else {
        record_entries(
            &request,
            select_adapter(selection).map_err(|e| map_error(&e, op))?,
            host,
            op,
        )?
    };
    Ok(Response::ok(payload))
}

/// The finding-shaped half of [`record`]: write the scan, then bind what it
/// discharges unless the scan was vacuous.
fn record_scan(
    request: &RecordRequest,
    adapter: quoin_evidence::adapters::FindingAdapter,
    host: &dyn EvidenceHost,
    op: &'static str,
) -> Result<serde_json::Value, CoreError> {
    let result = (adapter.parse)(&request.results).map_err(|e| map_error(&e, op))?;
    // The scanner's own claim about its identity, checked against the suite's
    // declared tool. A scan recorded under the wrong tool is a transcript that
    // names a producer that did not produce it.
    if let Some(reported) = result.tool.as_deref()
        && reported != request.tool
        && Some(reported) != tool_name(&request.tool)
    {
        return Err(CoreError::new(
            CoreErrorCode::Refused,
            format!(
                "result tool identity {reported:?} does not match --tool {:?}",
                request.tool
            ),
        )
        .with_context("op", op));
    }
    let vacuous = result.rules_evaluated == Some(0);
    let mut ids: Vec<String> = request
        .discharges
        .iter()
        .map(|id| id.trim().to_owned())
        .filter(|id| !id.is_empty())
        .collect();
    ids.sort();
    ids.dedup();

    let root = host.store_root(Path::new(&request.repo));
    host.with_store(Path::new(&request.repo), &mut |source| {
        let scan_path = write_scan(
            source,
            &FindingRecord {
                schema_version: STORE_SCHEMA_VERSION,
                suite: SuiteId::new(&request.suite),
                commit: Commit::new(&request.commit),
                tool: request.tool.clone(),
                evidence_kind: request.kind.clone(),
                timestamp: request.timestamp.clone(),
                ruleset: result.ruleset.clone(),
                rules_evaluated: result.rules_evaluated,
                findings: result.findings.clone(),
            },
        )
        .map_err(|e| map_error(&e, op))?;

        let mut bound_ids = Vec::new();
        let mut unknown = Vec::new();
        // A rule-less scan reports zero findings and proves nothing, so binding
        // on it would put the store's strongest claim behind its weakest
        // evidence (FR-034 vacuity).
        if !ids.is_empty() && !vacuous {
            let mut bindings = read_bindings(source)
                .map_err(|e| map_error(&e, op))?
                .bindings;
            for id in &ids {
                let Some(obligation) = request.obligations.iter().find(|o| &o.id == id) else {
                    unknown.push(id.clone());
                    continue;
                };
                let outcome = bind(
                    &bindings,
                    &Binding {
                        obligation: ObligationId::new(id),
                        statement_hash_at_binding: StatementHash::new(&obligation.statement_hash),
                        suite: SuiteId::new(&request.suite),
                        commit: Commit::new(&request.commit),
                        // A scan binds no symbol: it has none. The record is
                        // the evidence.
                        symbols: Vec::new(),
                        lineage: request.lineage.clone(),
                        affirmations: None,
                    },
                );
                bindings = outcome.bindings;
                if outcome.created {
                    bound_ids.push(id.clone());
                }
            }
            write_bindings(source, &bindings).map_err(|e| map_error(&e, op))?;
        }
        unknown.sort();
        value(&RecordPayload::Scan {
            scan_path: absolute(&root, &scan_path),
            findings: result.findings.clone(),
            bound: bound_ids,
            unknown,
            vacuous,
            rules_evaluated: result.rules_evaluated,
        })
    })
}

/// The run-shaped half of [`record`]: transcribe the entries and let
/// `quoin_evidence::record::record_run` update the binding graph.
fn record_entries(
    request: &RecordRequest,
    adapter: quoin_evidence::adapters::RunAdapter,
    host: &dyn EvidenceHost,
    op: &'static str,
) -> Result<serde_json::Value, CoreError> {
    let result = (adapter.parse)(&request.results).map_err(|e| map_error(&e, op))?;
    let unrepresented = result.unrepresented.as_deref().map(unrepresented_views);
    let root = host.store_root(Path::new(&request.repo));
    host.with_store(Path::new(&request.repo), &mut |source| {
        let outcome = record_run(
            source,
            &StoreRecordRequest {
                suite: SuiteId::new(&request.suite),
                commit: Commit::new(&request.commit),
                tool: request.tool.clone(),
                evidence_kind: request.kind.clone(),
                lineage: request.lineage.clone(),
                timestamp: request.timestamp.clone(),
                entries: result.entries.clone(),
            },
            &request.obligations,
        )
        .map_err(|e| map_error(&e, op))?;
        value(&RecordPayload::Run {
            run_path: absolute(&root, &outcome.run_path),
            bound: outcome.bound,
            suspect: outcome.suspect,
            unmatched: outcome.unmatched,
            unrepresented: unrepresented.clone(),
        })
    })
}

/// Answer an `evidence.trust_decision`.
///
/// # Errors
///
/// - [`CoreErrorCode::Refused`] when the request exceeds
///   [`MAX_TRUST_DECISION_BYTES`], or when the store cannot be written.
/// - [`CoreErrorCode::BadRequest`] when the document is not a trust decision or
///   fails the FR-093 boundary.
/// - [`CoreErrorCode::Io`] when no evidence host was granted.
pub fn trust_decision(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    let op = "evidence.trust_decision";
    bound(request, op, MAX_TRUST_DECISION_BYTES)?;
    let request: TrustDecisionRequest = parse(request, op)?;
    check_scalar(op, "repo", &request.repo)?;
    let decision: TrustDecision =
        serde_json::from_value(request.decision.clone()).map_err(|e| {
            CoreError::new(CoreErrorCode::BadRequest, format!("trust decision: {e}"))
                .with_context("op", op)
        })?;
    // Assessed BEFORE it is written, because `assess_trust` validates: a
    // decision the boundary refuses must never reach the store, and a store
    // that held one could render it as `accepted`.
    let assessment = assess_trust(&decision).map_err(|e| map_error(&e, op))?;

    let host = host(capabilities, op)?;
    let root = host.store_root(Path::new(&request.repo));
    let payload = host.with_store(Path::new(&request.repo), &mut |source| {
        let path = write_trust_decision(source, &decision).map_err(|e| map_error(&e, op))?;
        value(&TrustDecisionPayload {
            path: absolute(&root, &path),
            assessment: assessment.clone(),
        })
    })?;
    Ok(Response::ok(payload))
}

/// Answer an `evidence.inspect_mocks`.
///
/// An empty completed inspection is recorded too. That is how audit tells
/// "looked and found no relevant injections" from "nobody looked"
/// (agent-ix/quoin#204).
///
/// # Errors
///
/// - [`CoreErrorCode::Refused`] when the request exceeds
///   [`MAX_INSPECT_MOCKS_BYTES`], or when the source walk fails.
/// - [`CoreErrorCode::Io`] when no evidence host was granted.
pub fn inspect_mocks(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    let op = "evidence.inspect_mocks";
    bound(request, op, MAX_INSPECT_MOCKS_BYTES)?;
    let request: InspectMocksRequest = parse(request, op)?;
    for (field, text) in [
        ("repo", request.repo.as_str()),
        ("suite", request.suite.as_str()),
        ("commit", request.commit.as_str()),
        ("tool", request.tool.as_str()),
        ("timestamp", request.timestamp.as_str()),
    ] {
        check_scalar(op, field, text)?;
    }

    let host = host(capabilities, op)?;
    let root = host.store_root(Path::new(&request.repo));
    let payload = host.with_store(Path::new(&request.repo), &mut |source| {
        let suite = SuiteId::new(&request.suite);
        let injections = inspect_mock_injections(source, &suite).map_err(|e| map_error(&e, op))?;
        let path = if request.dry_run {
            None
        } else {
            Some(
                write_mock_inspection(
                    source,
                    &MockInspectionRecord {
                        schema_version: STORE_SCHEMA_VERSION,
                        suite: suite.clone(),
                        commit: Commit::new(&request.commit),
                        tool: request.tool.clone(),
                        timestamp: request.timestamp.clone(),
                        injections: injections.clone(),
                    },
                )
                .map_err(|e| map_error(&e, op))
                .map(|written| absolute(&root, written))?,
            )
        };
        value(&InspectMocksPayload { path, injections })
    })?;
    Ok(Response::ok(payload))
}

/// Answer an `evidence.record_experiment`.
///
/// # Errors
///
/// As [`record_operational`].
pub fn record_experiment(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    assurance_record(request, capabilities, AssuranceFamily::Experiment)
}

/// Answer an `evidence.record_operational`.
///
/// # Errors
///
/// - [`CoreErrorCode::Refused`] when the request exceeds
///   [`MAX_ASSURANCE_RECORD_BYTES`], when the store cannot be written, or when
///   the path already holds different bytes under the same identity.
/// - [`CoreErrorCode::BadRequest`] when the document does not validate.
/// - [`CoreErrorCode::Io`] when no evidence host was granted.
pub fn record_operational(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
) -> Result<Response, CoreError> {
    assurance_record(request, capabilities, AssuranceFamily::Operational)
}

/// Which content-addressed assurance family a record belongs to.
///
/// The two operations share one request shape, one ceiling and one payload
/// shape, and differ only in which writer they call and which directory the
/// record lands in. That difference was an `if op == "evidence.record_experiment"`
/// over the operation NAME, which is a string comparison standing in for a
/// choice with exactly two answers: a misspelling silently selected the other
/// family and the `else` branch would have swallowed a third family added
/// later. As an enum the match is exhaustive, so a new family is a compile
/// error rather than a skipped branch — the quoin#447 trap, one level down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AssuranceFamily {
    /// A design-of-experiments record, under `experiments/`.
    Experiment,
    /// An operational evidence record, under `operational/`.
    Operational,
}

impl AssuranceFamily {
    /// The wire spelling of the operation that writes this family.
    pub(super) const fn op(self) -> &'static str {
        match self {
            Self::Experiment => "evidence.record_experiment",
            Self::Operational => "evidence.record_operational",
        }
    }

    /// The family one wire spelling selects, or `None` for any other.
    ///
    /// Only the round-trip test calls this: production goes the other way,
    /// from a dispatch arm that already knows which family it routed. It
    /// exists so the round trip is a property something can check, rather than
    /// two literal lists nobody compares.
    #[cfg(test)]
    pub(super) fn from_op(op: &str) -> Option<Self> {
        match op {
            "evidence.record_experiment" => Some(Self::Experiment),
            "evidence.record_operational" => Some(Self::Operational),
            _ => None,
        }
    }
}

/// Publish one content-addressed assurance record, experiment or operational.
fn assurance_record(
    request: &serde_json::Value,
    capabilities: &Capabilities<'_>,
    family: AssuranceFamily,
) -> Result<Response, CoreError> {
    let op = family.op();
    bound(request, op, MAX_ASSURANCE_RECORD_BYTES)?;
    let request: AssuranceRecordRequest = parse(request, op)?;
    check_scalar(op, "repo", &request.repo)?;

    let host = host(capabilities, op)?;
    let root = host.store_root(Path::new(&request.repo));
    let payload = host.with_store(Path::new(&request.repo), &mut |source| {
        let (path, created, record) = match family {
            AssuranceFamily::Experiment => {
                let stored = write_experiment_record(source, &request.document)
                    .map_err(|e| map_error(&e, op))?;
                (
                    stored.path,
                    stored.created,
                    serde_json::to_value(&stored.record),
                )
            }
            AssuranceFamily::Operational => {
                let stored = write_operational_evidence_record(source, &request.document)
                    .map_err(|e| map_error(&e, op))?;
                (
                    stored.path,
                    stored.created,
                    serde_json::to_value(&stored.record),
                )
            }
        };
        let record = record.map_err(|e| CoreError::new(CoreErrorCode::Io, e.to_string()))?;
        value(&AssuranceRecordPayload {
            path: absolute(&root, &path),
            created,
            record,
        })
    })?;
    Ok(Response::ok(payload))
}
