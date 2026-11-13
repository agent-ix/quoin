// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The policy-finding join (FR-142): `licence-policy` findings and `cargo-deny`
//! ban and licence findings laid onto `soup_record` entries at read time.
//!
//! Pure, like [`super::advisory`], and kept apart from it: a policy finding is
//! attached and counted in lists of its own, never merged into an advisory
//! count (FR-142-AC-34). The `licence-policy` scan is read through its
//! `evaluated` list, so a record the scan did not evaluate is stated not
//! evaluated and never as having no policy findings (FR-142-AC-35); a record is
//! never stated allowed (NFR-030-AC-5).

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use quoin_assurance::DischargeReport;
use quoin_evidence::adapters::{AdapterSelection, select_finding_adapter};
use quoin_evidence::purl::PurlIdentity;
use quoin_evidence::store::scan_is_vacuous;
use quoin_evidence::types::{Finding, FindingRecord};

use super::advisory::{
    Hit, Newest, RecordIndex, RecordRef, Resolution, Tool, UnmatchedReason, newest, resolve,
};
use super::plan::Existing;

/// The adapter name of the policy scanner the join reads besides `cargo-deny`.
const LICENCE_POLICY: &str = "licence-policy";

/// The `cargo-deny` codes that are policy findings: the `licenses` check's
/// `rejected` and `unlicensed`, the `bans` check's `banned` and `not-allowed`.
/// A claim about cargo-deny 0.19.8, confirmed against captured output.
const DENY_POLICY_CODES: [&str; 4] = ["rejected", "unlicensed", "banned", "not-allowed"];

/// Whether a finding of `tool` is a policy finding.
pub(super) fn is_policy(tool: Tool, finding: &Finding) -> bool {
    tool == Tool::CargoDeny && DENY_POLICY_CODES.contains(&finding.rule_id.as_str())
}

/// The scanner that reported a policy finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum PolicyTool {
    /// `quoin evidence record --adapter licence-policy`.
    LicencePolicy,
    /// `cargo deny --format json check`.
    CargoDeny,
}

impl PolicyTool {
    /// The registry name.
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::LicencePolicy => LICENCE_POLICY,
            Self::CargoDeny => Tool::CargoDeny.name(),
        }
    }
}

fn is_licence_policy(scan: &FindingRecord) -> bool {
    select_finding_adapter(AdapterSelection {
        adapter: None,
        tool: Some(&scan.tool),
    })
    .is_some_and(|adapter| adapter.name == LICENCE_POLICY)
}

/// What became of the newest retained `licence-policy` scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum LicenceOutcome {
    /// None is retained, or its recording was refused (a refusal retains nothing).
    NotRun,
    /// The SBOM listed nothing (FR-034-AC-10).
    Vacuous {
        /// The scan's suite.
        suite: String,
        /// The scan's timestamp.
        timestamp: String,
    },
    /// A retained scan has a `timestamp` that is not an instant; none was read.
    Unordered {
        /// The suites of the scans with such a timestamp.
        suites: Vec<String>,
    },
    /// The newest scan was read.
    Read {
        /// The scan's suite.
        suite: String,
        /// The scan's timestamp.
        timestamp: String,
        /// The policy the scan was made against, as it recorded it.
        ruleset: Option<String>,
        /// Components the scan evaluated.
        evaluated: usize,
        /// Distinct findings read: attached plus unmatched.
        findings: usize,
    },
}

/// The `licence-policy` scan state, and the older suites not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LicenceState {
    /// What became of the scan.
    pub(super) outcome: LicenceOutcome,
    /// Suites of older retained scans, which the join did not read.
    pub(super) unread: Vec<String>,
}

/// A policy finding attached to a record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PolicyFinding {
    /// The scanner.
    pub(super) tool: PolicyTool,
    /// Its rule id.
    pub(super) rule_id: String,
    /// Its own severity word, unchanged.
    pub(super) severity: Option<String>,
    /// Its message, unchanged.
    pub(super) message: Option<String>,
    /// The finding's path.
    pub(super) package: String,
    /// The timestamp of the scan that reported it.
    pub(super) scan_timestamp: String,
    /// Resolved or unresolved, against the scan that reported this finding.
    pub(super) resolution: Resolution,
}

/// The one thing a row says of its policy status when it has no finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PolicyStatus {
    /// Findings are attached; the row lists them.
    Findings,
    /// The newest scan was read, evaluated the record, and attached nothing.
    NoFindings {
        /// The scan's timestamp.
        timestamp: String,
        /// The scan's ruleset.
        ruleset: Option<String>,
    },
    /// No `licence-policy` scan is retained.
    NotRun,
    /// The newest scan evaluated nothing.
    Vacuous,
    /// The scans cannot be put in order.
    Unordered,
    /// The scan read does not list the record in `evaluated`.
    NotEvaluated,
    /// An unmatched finding could belong to the record.
    Blocked,
}

/// The policy findings of one record and its status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RecordPolicy {
    /// Attached findings, sorted by rule id, package, tool.
    pub(super) findings: Vec<PolicyFinding>,
    /// The status stated when there are none.
    pub(super) status: PolicyStatus,
}

/// A policy finding that attached to no record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PolicyUnmatched {
    /// The scanner.
    pub(super) tool: PolicyTool,
    /// The rule id.
    pub(super) rule_id: String,
    /// The finding's path, when it has one.
    pub(super) package: Option<String>,
    /// Its severity word.
    pub(super) severity: Option<String>,
    /// Why.
    pub(super) reason: UnmatchedReason,
}

/// The whole policy join.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PolicyJoin {
    /// The `licence-policy` scan state.
    pub(super) licence: LicenceState,
    /// Distinct `cargo-deny` policy findings read, `None` when its scan was not read.
    pub(super) deny_read: Option<usize>,
    /// Per record, keyed by the record file.
    pub(super) records: BTreeMap<PathBuf, RecordPolicy>,
    /// Findings attached to no record, sorted by tool, rule id, package.
    pub(super) unmatched: Vec<PolicyUnmatched>,
}

/// A matched finding before its resolution is decided.
struct Pending {
    tool: PolicyTool,
    record: usize,
    finding: Finding,
    scan_timestamp: String,
    scan_millis: i64,
}

/// The findings of a scan, each distinct `(rule id, path)` once.
fn distinct(scan: &FindingRecord) -> impl Iterator<Item = &Finding> {
    let mut seen: BTreeSet<(&str, Option<&str>)> = BTreeSet::new();
    scan.findings
        .iter()
        .filter(move |finding| seen.insert((finding.rule_id.as_str(), finding.path.as_deref())))
}

/// Join the newest retained `licence-policy` scan and the policy findings of the
/// newest `cargo-deny` scan onto `records` (FR-142).
///
/// `records` must already be checked for duplicate purls by the caller.
pub(super) fn join(
    records: &[Existing],
    scans: &[FindingRecord],
    discharge: &DischargeReport,
) -> PolicyJoin {
    let mut pending: Vec<Pending> = Vec::new();
    let mut unmatched: Vec<PolicyUnmatched> = Vec::new();
    // Records an unmatched `licence-policy` finding could belong to.
    let mut could_belong: BTreeSet<usize> = BTreeSet::new();
    let mut evaluated: BTreeSet<PurlIdentity> = BTreeSet::new();

    let (licence_scan, unread) = match newest(scans, is_licence_policy) {
        Newest::None => (Err(LicenceOutcome::NotRun), Vec::new()),
        Newest::Unordered(suites) => (Err(LicenceOutcome::Unordered { suites }), Vec::new()),
        Newest::Scan {
            scan,
            millis,
            unread,
        } => (Ok((scan, millis)), unread),
    };
    let outcome = match licence_scan {
        Err(outcome) => outcome,
        Ok((scan, _)) if scan_is_vacuous(scan) == Some(true) => LicenceOutcome::Vacuous {
            suite: scan.suite.as_str().to_owned(),
            timestamp: scan.timestamp.clone(),
        },
        Ok((scan, millis)) => {
            let index = RecordIndex::new(records);
            evaluated.extend(
                scan.evaluated
                    .iter()
                    .flatten()
                    .filter_map(|symbol| PurlIdentity::parse(symbol)),
            );
            let mut findings = 0_usize;
            for finding in distinct(scan) {
                findings = findings.saturating_add(1);
                match index.find_purl(finding.path.as_deref()) {
                    Hit::One(RecordRef(record)) => pending.push(Pending {
                        tool: PolicyTool::LicencePolicy,
                        record,
                        finding: finding.clone(),
                        scan_timestamp: scan.timestamp.clone(),
                        scan_millis: millis,
                    }),
                    Hit::Not(reason, candidates) => {
                        could_belong.extend(candidates);
                        unmatched.push(unmatched_of(PolicyTool::LicencePolicy, finding, reason));
                    }
                }
            }
            LicenceOutcome::Read {
                suite: scan.suite.as_str().to_owned(),
                timestamp: scan.timestamp.clone(),
                ruleset: scan.ruleset.clone(),
                evaluated: evaluated.len(),
                findings,
            }
        }
    };

    let deny_read = read_deny(records, scans, &mut pending, &mut unmatched);
    unmatched
        .sort_by(|a, b| (a.tool, &a.rule_id, &a.package).cmp(&(b.tool, &b.rule_id, &b.package)));

    let mut attached = attach(records, pending, discharge);
    let per_record = records
        .iter()
        .enumerate()
        .map(|(position, record)| {
            let mut findings = attached.remove(&position).unwrap_or_default();
            findings.sort_by(|a, b| {
                (&a.rule_id, &a.package, a.tool).cmp(&(&b.rule_id, &b.package, b.tool))
            });
            let status = if findings.is_empty() {
                status_of(
                    &outcome,
                    record,
                    &evaluated,
                    could_belong.contains(&position),
                )
            } else {
                PolicyStatus::Findings
            };
            (record.path.clone(), RecordPolicy { findings, status })
        })
        .collect();
    PolicyJoin {
        licence: LicenceState { outcome, unread },
        deny_read,
        records: per_record,
        unmatched,
    }
}

fn unmatched_of(tool: PolicyTool, finding: &Finding, reason: UnmatchedReason) -> PolicyUnmatched {
    PolicyUnmatched {
        tool,
        rule_id: finding.rule_id.clone(),
        package: finding.path.clone(),
        severity: finding.severity.clone(),
        reason,
    }
}

/// The policy findings of the newest `cargo-deny` scan, when it is read. Its
/// state is the advisory join's; only its policy findings are read here.
fn read_deny(
    records: &[Existing],
    scans: &[FindingRecord],
    pending: &mut Vec<Pending>,
    unmatched: &mut Vec<PolicyUnmatched>,
) -> Option<usize> {
    let Newest::Scan { scan, millis, .. } =
        newest(scans, |scan| Tool::of(scan) == Some(Tool::CargoDeny))
    else {
        return None;
    };
    if scan_is_vacuous(scan) == Some(true) {
        return None;
    }
    let index = RecordIndex::new(records);
    let mut read = 0_usize;
    for finding in distinct(scan).filter(|finding| is_policy(Tool::CargoDeny, finding)) {
        read = read.saturating_add(1);
        match index.find(finding.path.as_deref()) {
            Hit::One(RecordRef(record)) => pending.push(Pending {
                tool: PolicyTool::CargoDeny,
                record,
                finding: finding.clone(),
                scan_timestamp: scan.timestamp.clone(),
                scan_millis: millis,
            }),
            Hit::Not(reason, _) => {
                unmatched.push(unmatched_of(PolicyTool::CargoDeny, finding, reason));
            }
        }
    }
    Some(read)
}

/// Resolve every matched finding against its record's clause, each against the
/// scan that reported it: one fact covers the findings of scans at or before its
/// attestation and none reported later.
fn attach(
    records: &[Existing],
    pending: Vec<Pending>,
    discharge: &DischargeReport,
) -> BTreeMap<usize, Vec<PolicyFinding>> {
    let mut attached: BTreeMap<usize, Vec<PolicyFinding>> = BTreeMap::new();
    for item in pending {
        let Some(record) = records.get(item.record) else {
            continue;
        };
        attached
            .entry(item.record)
            .or_default()
            .push(PolicyFinding {
                tool: item.tool,
                rule_id: item.finding.rule_id,
                severity: item.finding.severity,
                message: item.finding.message,
                package: item.finding.path.unwrap_or_default(),
                scan_timestamp: item.scan_timestamp,
                resolution: resolve(
                    record.disposition_clause.as_deref(),
                    discharge,
                    item.scan_millis,
                ),
            });
    }
    attached
}

/// The status of a record with no attached finding.
fn status_of(
    outcome: &LicenceOutcome,
    record: &Existing,
    evaluated: &BTreeSet<PurlIdentity>,
    blocked: bool,
) -> PolicyStatus {
    match outcome {
        LicenceOutcome::NotRun => PolicyStatus::NotRun,
        LicenceOutcome::Vacuous { .. } => PolicyStatus::Vacuous,
        LicenceOutcome::Unordered { .. } => PolicyStatus::Unordered,
        LicenceOutcome::Read {
            timestamp, ruleset, ..
        } => {
            let listed =
                PurlIdentity::parse(&record.purl).is_some_and(|id| evaluated.contains(&id));
            if !listed {
                PolicyStatus::NotEvaluated
            } else if blocked {
                PolicyStatus::Blocked
            } else {
                PolicyStatus::NoFindings {
                    timestamp: timestamp.clone(),
                    ruleset: ruleset.clone(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
