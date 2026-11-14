// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The strategy and solution-leaf parts of the case: how a decomposition is
//! named and what a bound leaf rests on (FR-040-AC-16..AC-19).

use std::collections::{BTreeMap, BTreeSet};

use quoin_finding_types::{Finding, UnevaluatedCheck};
use quoin_quire_types::{Obligation, TargetLocus};
use quoin_store::instant::{Instant, InstantGrammar};
use serde::{Deserialize, Serialize};

use super::{CaseInput, CaseNode, NodeKind, NodeStatus};
use crate::matrix::{EvidenceStatus, bucket, status_of};

/// One suite bound to a leaf, with the newest run or scan the store holds for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuiteRef {
    /// The bound suite.
    pub suite: String,
    /// ISO-8601 timestamp of its newest run or scan; absent when the store
    /// holds none for the suite.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub at: Option<String>,
}

/// What a solution leaf is bound to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeafEvidence {
    /// Bound suites, sorted.
    pub suites: Vec<SuiteRef>,
    /// The newest timestamp across `suites`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub newest: Option<String>,
    /// The auditor's verdict, in the matrix's terms (FR-115): `bound` only when
    /// the audit lists the obligation healthy, then `suspect`, `stale`, else
    /// `undischarged`. Absent when the caller supplied no `healthy` list and
    /// the auditor raised nothing.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub verdict: Option<EvidenceStatus>,
}

/// One run or scan the store holds, reduced to what the view shows.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    /// The suite the run or scan covered.
    pub suite: String,
    /// ISO-8601, as the store recorded it.
    pub timestamp: String,
}

/// Child id → the verbs it declares toward one parent.
pub(super) type Children = BTreeMap<String, BTreeMap<String, BTreeSet<String>>>;

/// Obligation id → every auditor finding against it, most serious first.
pub(super) type FindingsFor<'a> = BTreeMap<&'a str, Vec<&'a Finding>>;

/// What the store holds, as the leaves read it.
pub(super) struct Store<'a> {
    /// Obligation id → the suites bound to it.
    suites_for: BTreeMap<&'a str, BTreeSet<&'a str>>,
    /// Suite → its newest run or scan timestamp.
    newest_for_suite: BTreeMap<&'a str, &'a str>,
    /// The healthy list, when the caller supplied one.
    healthy: Option<BTreeSet<&'a str>>,
    /// Obligation id → the checks the audit could not evaluate.
    unevaluated_for: BTreeMap<&'a str, Vec<&'a UnevaluatedCheck>>,
}

/// Orders timestamps by the instant they name, so `…00.500Z` follows `…00Z`
/// and `+01:00` offsets compare correctly. A timestamp that is not RFC 3339
/// sorts before every instant, then lexically among its kind.
fn instant_key(text: &str) -> (i64, &str) {
    let millis = Instant::parse(text, InstantGrammar::Rfc3339).and_then(|i| i.epoch_millis());
    (millis.unwrap_or(i64::MIN), text)
}

impl<'a> Store<'a> {
    pub(super) fn new(input: &'a CaseInput) -> Self {
        let mut newest_for_suite: BTreeMap<&str, &str> = BTreeMap::new();
        for observation in input.observations.iter().flatten() {
            let newest = newest_for_suite
                .entry(&observation.suite)
                .or_insert(&observation.timestamp);
            if instant_key(&observation.timestamp) > instant_key(newest) {
                *newest = &observation.timestamp;
            }
        }
        let mut suites_for: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for binding in input.bindings.iter().flatten() {
            suites_for
                .entry(binding.obligation.as_str())
                .or_default()
                .insert(binding.suite.as_str());
        }
        let mut unevaluated_for: BTreeMap<&str, Vec<&UnevaluatedCheck>> = BTreeMap::new();
        for check in input.unevaluated.iter().flatten() {
            unevaluated_for
                .entry(check.obligation.as_str())
                .or_default()
                .push(check);
        }
        Self {
            suites_for,
            newest_for_suite,
            healthy: input
                .healthy
                .as_ref()
                .map(|ids| ids.iter().map(String::as_str).collect()),
            unevaluated_for,
        }
    }
}

/// The strategy a goal is argued by: its sub-goals, grouped by edge verb.
pub(super) fn strategy_for(
    goal: &str,
    edges: &[(&String, &BTreeSet<String>)],
    sub_nodes: Vec<CaseNode>,
) -> CaseNode {
    let mut by_verb: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (child, verbs) in edges {
        for verb in *verbs {
            by_verb.entry(verb).or_default().push(child);
        }
    }
    let argued: Vec<String> = by_verb
        .iter()
        .map(|(verb, ids)| format!("`{verb}`: {}", ids.join(", ")))
        .collect();
    let status = if sub_nodes.iter().any(|n| n.status == NodeStatus::Open) {
        NodeStatus::Open
    } else {
        NodeStatus::Supported
    };
    CaseNode {
        id: format!("{goal}/strategy"),
        kind: NodeKind::Strategy,
        statement: format!("argued over {}", argued.join(" | ")),
        status,
        because: None,
        evidence: None,
        analysis_evidence: None,
        children: sub_nodes,
    }
}

pub(super) fn solution_for(
    obligation: &Obligation,
    finding_for: &FindingsFor,
    store: &Store,
    analysis_evidence: Option<&super::super::matrix::AnalysisEvidenceInput>,
    all_obligations: &[&Obligation],
) -> CaseNode {
    let findings = finding_for
        .get(obligation.id.as_str())
        .map_or(&[][..], Vec::as_slice);
    let id = obligation.id.as_str();
    let finding = findings.first();
    let healthy = store.healthy.as_ref().map(|ids| ids.contains(id));
    let checks = store.unevaluated_for.get(id).map_or(&[][..], Vec::as_slice);
    // Healthy is the auditor's word, not "no finding": a ladder that stops
    // early or leaves a check unevaluated raises nothing and is still not
    // healthy. Only judged when the caller supplied the audit's list.
    let unhealthy = healthy == Some(false) && finding.is_none();
    let because = if let Some(f) = finding {
        Some(format!("{}: {}", f.kind, f.summary))
    } else if unhealthy && !checks.is_empty() {
        Some(
            checks
                .iter()
                .map(|c| format!("unevaluated {}: {}", c.check, c.reason))
                .collect::<Vec<_>>()
                .join(" | "),
        )
    } else if unhealthy {
        Some("not healthy: the audit reached no verdict for this obligation".to_owned())
    } else {
        None
    };
    let analysis_evidence = if obligation.method.as_deref() == Some("Analysis") {
        analysis_evidence.map(|evidence| analysis_items(obligation, all_obligations, evidence))
    } else {
        None
    };
    CaseNode {
        id: obligation.id.clone(),
        kind: NodeKind::Solution,
        statement: obligation.statement.clone(),
        status: if because.is_some() {
            NodeStatus::Open
        } else {
            NodeStatus::Supported
        },
        because,
        evidence: evidence_for(id, findings, healthy, store),
        analysis_evidence,
        children: Vec::new(),
    }
}

fn analysis_items(
    obligation: &Obligation,
    all_obligations: &[&Obligation],
    evidence: &super::super::matrix::AnalysisEvidenceInput,
) -> Vec<serde_json::Value> {
    let Some(identity) = obligation.identity.as_ref() else {
        return Vec::new();
    };
    if all_obligations
        .iter()
        .filter(|candidate| candidate.identity.as_ref() == Some(identity))
        .count()
        != 1
    {
        return Vec::new();
    }
    let mut items = Vec::new();
    for binder in &evidence.record_binders {
        if binder.get("target").and_then(target_locus) == Some(identity.clone()) {
            items.push(serde_json::json!({
                "status": "qualified-record",
                "record_binder": binder,
            }));
        }
    }
    for result in &evidence.evidence_results {
        if result.get("target").and_then(target_locus) == Some(identity.clone()) {
            if let Some(cause) = result.get("cause") {
                items.push(serde_json::json!({ "status": cause, "result": result }));
            }
        }
    }
    items.sort_by(|left, right| left.to_string().cmp(&right.to_string()));
    items
}

fn target_locus(value: &serde_json::Value) -> Option<TargetLocus> {
    serde_json::from_value(value.clone()).ok()
}

/// The leaf's bound suites and the auditor's verdict, when anything is bound.
fn evidence_for(
    id: &str,
    findings: &[&Finding],
    healthy: Option<bool>,
    store: &Store,
) -> Option<LeafEvidence> {
    let suites = store.suites_for.get(id)?;
    let suites: Vec<SuiteRef> = suites
        .iter()
        .map(|suite| SuiteRef {
            suite: (*suite).to_owned(),
            at: store.newest_for_suite.get(suite).map(|at| (*at).to_owned()),
        })
        .collect();
    let newest = suites
        .iter()
        .filter_map(|s| s.at.as_deref())
        .max_by_key(|at| instant_key(at))
        .map(str::to_owned);
    // The matrix's own precedence over the same findings (FR-115).
    let worst = findings.iter().map(|f| bucket(&f.kind)).min();
    let verdict = match (healthy, worst) {
        (None, None) => None,
        _ => Some(status_of(healthy == Some(true), worst)),
    };
    Some(LeafEvidence {
        suites,
        newest,
        verdict,
    })
}
