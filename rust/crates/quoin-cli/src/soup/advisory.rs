// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The advisory join (FR-142): scanner findings laid onto `soup_record` entries at read time.
//!
//! Pure. [`join`] takes the records, the retained [`FindingRecord`]s and the
//! discharge report of the `--binding` clause set, and returns a [`Join`]; it
//! reads no file, writes nothing and never touches a record (FR-142-CON-1).
//! `quoin soup report` (FR-144) reads the records, the scans and the facts and
//! renders the result.
//!
//! There is no advisory parser here. The scans were parsed when they were
//! recorded, by the FR-034 adapters of the FR-033 registry; this module only
//! asks the registry which adapter a retained scan's tool selects. A finding
//! Cargo findings match `<name>@<version>` against cargo purls; OSV findings
//! match purls by identity. A scanner's severity word is carried unchanged.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use quoin_assurance::{
    ClauseDischarge, DischargeFact, DischargeReport, DischargeState, DispositionDecision,
};
use quoin_evidence::adapters::{AdapterSelection, select_finding_adapter};
use quoin_evidence::store::scan_is_vacuous;
use quoin_evidence::types::{Finding, FindingRecord};
use quoin_store::instant::{Instant, InstantGrammar};

use super::plan::Existing;
mod index;
use super::policy::is_policy;
pub(super) use index::{Hit, RecordIndex, RecordRef};

/// The rule-id prefixes of a `cargo-deny` diagnostic that is an advisory. A
/// ban or licence diagnostic is a policy finding (`policy`, FR-142); source,
/// `unclassified` and every other code are set aside.
const DENY_ADVISORY_PREFIXES: [&str; 3] = ["RUSTSEC-", "GHSA-", "CVE-"];

/// A scanner the join reads, in the order the join lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Tool {
    /// `cargo audit --json`.
    CargoAudit,
    /// `cargo deny --format json check`.
    CargoDeny,
    /// OSV-Scanner JSON report.
    OsvScanner,
}

impl Tool {
    pub(super) const ALL: [Self; 3] = [Self::CargoAudit, Self::CargoDeny, Self::OsvScanner];

    /// The registry name of the tool's adapter.
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::CargoAudit => "cargo-audit",
            Self::CargoDeny => "cargo-deny",
            Self::OsvScanner => "osv-scanner",
        }
    }

    /// The tool a retained scan belongs to, chosen the way recording chooses its
    /// adapter. `None` for a scan of any other tool.
    pub(super) fn of(scan: &FindingRecord) -> Option<Self> {
        let adapter = select_finding_adapter(AdapterSelection {
            adapter: None,
            tool: Some(&scan.tool),
        })?;
        Self::ALL
            .into_iter()
            .find(|tool| tool.name() == adapter.name)
    }

    /// Whether a finding of this tool is an advisory.
    fn is_advisory(self, finding: &Finding) -> bool {
        match self {
            Self::CargoAudit => true,
            Self::CargoDeny => DENY_ADVISORY_PREFIXES
                .iter()
                .any(|prefix| finding.rule_id.starts_with(prefix)),
            Self::OsvScanner => true,
        }
    }

    /// Whether this scanner assesses the record's package-url type.
    pub(super) fn assesses(self, purl: &str) -> bool {
        match self {
            Self::CargoAudit | Self::CargoDeny => index::purl_type(purl) == Some("cargo"),
            Self::OsvScanner => {
                matches!(
                    index::purl_type(purl),
                    Some("npm" | "pypi" | "golang" | "deb")
                )
            }
        }
    }
}

/// What the join found of one tool's newest retained scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ScanState {
    /// The tool.
    pub(super) tool: Tool,
    /// The ruleset or database identity exposed by the retained report.
    pub(super) ruleset: Option<String>,
    /// What became of its scan.
    pub(super) outcome: ScanOutcome,
    /// The suites of the tool's older retained scans, which the join did not
    /// read: it reads the newest scan of a tool across all suites.
    pub(super) unread: Vec<String>,
}

/// The state of a tool's scan. Only [`ScanOutcome::Read`] is evidence about a
/// record; the other two say nothing was learned and never read as clean.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ScanOutcome {
    /// No scan of the tool is retained, or its log was refused at recording
    /// (a refused log leaves no record).
    NotRun,
    /// The newest scan evaluated no rules (FR-034-AC-10).
    Vacuous {
        /// The scan's suite.
        suite: String,
        /// The scan's timestamp.
        timestamp: String,
    },
    /// A retained scan has a `timestamp` that is not an instant, so the scans of
    /// the tool cannot be put in order. None was read.
    Unordered {
        /// The suites of the scans with such a timestamp.
        suites: Vec<String>,
    },
    /// The newest scan ran with rules, and was read.
    Read {
        /// The scan's suite.
        suite: String,
        /// The scan's timestamp.
        timestamp: String,
        /// Distinct advisories read: attached plus unmatched. A repeated
        /// finding with one rule id and path counts once.
        advisories: usize,
        /// Distinct diagnostics that are neither advisories nor policy
        /// findings, which the join does not read (`cargo-deny` source,
        /// `unclassified`, `duplicate`, `wildcard` and the other codes).
        set_aside: usize,
    },
}

impl ScanOutcome {
    /// A scan that was read and reported no advisory: the one state that may
    /// be reported as a clean scan.
    pub(super) fn is_clean(&self) -> bool {
        matches!(self, Self::Read { advisories: 0, .. })
    }
}

/// An advisory finding attached to a record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Attached {
    /// The scanner that reported it.
    pub(super) tool: Tool,
    /// The scanner's rule id (the advisory id).
    pub(super) rule_id: String,
    /// The scanner's own severity word.
    pub(super) severity: Option<String>,
    /// The scanner's message.
    pub(super) message: Option<String>,
    /// The finding's path: `<name>@<version>`.
    pub(super) package: String,
    /// The timestamp of the scan that reported it.
    pub(super) scan_timestamp: String,
    /// Resolved or unresolved, and why.
    pub(super) resolution: Resolution,
}

/// Whether an attached advisory is resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Resolution {
    /// A current approved disposition, attested at or after the scan.
    Resolved(Disposition),
    /// Not resolved, for the stated reason.
    Unresolved(Unresolved),
}

/// The fact's own words, copied unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Disposition {
    /// What was decided.
    pub(super) decision: DispositionDecision,
    /// Why.
    pub(super) rationale: String,
    /// The reference to the approval.
    pub(super) approval_ref: String,
    /// Who attested.
    pub(super) attested_by: String,
}

/// Why an advisory is unresolved.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(super) enum Unresolved {
    /// The record names no `disposition_clause`.
    #[error("no disposition clause")]
    NoDispositionClause,
    /// The clause is not in the binding report.
    #[error("the disposition clause is not a clause of the binding report")]
    ClauseNotInBindingReport,
    /// The clause is in the report and does not bind; the string is Quire's
    /// reason, or why the clause is out.
    #[error("the disposition clause does not bind: {0}")]
    ClauseDoesNotBind(String),
    /// Nothing is stored under the clause.
    #[error("no fact is stored under the disposition clause")]
    NoFact,
    /// The fact under the clause is a direct discharge, not a disposition.
    #[error("the fact under the disposition clause is not an approved disposition")]
    NotApprovedDisposition,
    /// The disposition has expired or is not yet valid at `asOf`; the string is
    /// the discharge report's reason.
    #[error("the disposition is not current: {0}")]
    NotCurrent(String),
    /// The fact was attested before the newest scan that reports the advisory.
    #[error("scan newer than attestation")]
    ScanNewerThanAttestation,
}

/// A record and the advisories attached to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RecordJoin {
    /// Where the record lives.
    pub(super) path: PathBuf,
    /// The record's `id`.
    pub(super) id: Option<String>,
    /// The record's purl.
    pub(super) purl: String,
    /// The record's `disposition_clause`.
    pub(super) clause: Option<String>,
    /// Attached advisories, sorted by rule id, package, then tool.
    pub(super) advisories: Vec<Attached>,
    /// The tools whose scan was read, reported no advisory on this record, and
    /// left no unmatched finding that could belong to it (ambiguous, naming its
    /// crate without a version, or with no path). The only way a
    /// record is reported free of a tool's advisories.
    pub(super) free_of: Vec<Tool>,
}

/// Why a finding is listed instead of attached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(super) enum UnmatchedReason {
    /// The finding's path carries no version (or no path).
    #[error("no version")]
    NoVersion,
    /// No record has the finding's purl.
    #[error("no record")]
    NoRecord,
    /// More than one record has the finding's purl.
    #[error("ambiguous")]
    Ambiguous,
    /// The finding names no purl, so no record can be selected (FR-142).
    #[error("no purl")]
    NoPurl,
}

/// An advisory finding that attached to no record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Unmatched {
    /// The scanner.
    pub(super) tool: Tool,
    /// The rule id.
    pub(super) rule_id: String,
    /// The finding's path, when it has one.
    pub(super) package: Option<String>,
    /// The scanner's own severity word.
    pub(super) severity: Option<String>,
    /// Why.
    pub(super) reason: UnmatchedReason,
}

/// The whole join.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Join {
    /// One entry per tool, in [`Tool::ALL`] order.
    pub(super) scans: Vec<ScanState>,
    /// Every record, sorted by purl then path.
    pub(super) records: Vec<RecordJoin>,
    /// Findings attached to no record, sorted by tool, rule id, package.
    pub(super) unmatched: Vec<Unmatched>,
}

/// Join the newest retained advisory scans onto `records`
/// (FR-142).
///
/// `scans` is every retained finding-shaped record the store holds; the newest
/// by `timestamp` of each tool is read (the others are listed in
/// [`ScanState::unread`]) and scans of other tools are ignored. If any scan of
/// a tool has a `timestamp` that is not an instant, no scan of that tool is
/// read ([`ScanOutcome::Unordered`]). `discharge` is the discharge report of the `--binding` report's
/// clause set at the report's `asOf` (FR-133). `records` must already be
/// checked for duplicate purls by the caller, as `soup promote` does; two with
/// one purl make a finding of that crate `ambiguous`.
///
/// `finding_refs` and the `vulnerability` object are not inputs.
pub(super) fn join(
    records: &[Existing],
    scans: &[FindingRecord],
    discharge: &DischargeReport,
) -> Join {
    let index = RecordIndex::new(records);
    let mut pending: Vec<Pending> = Vec::new();
    let mut unmatched: Vec<Unmatched> = Vec::new();
    let mut could_belong: BTreeSet<(Tool, usize)> = BTreeSet::new();
    let mut states = Vec::new();

    for tool in Tool::ALL {
        let (chosen, unread) = match newest(scans, |scan| Tool::of(scan) == Some(tool)) {
            Newest::None => (None, Vec::new()),
            Newest::Unordered(suites) => {
                states.push(ScanState {
                    tool,
                    ruleset: None,
                    outcome: ScanOutcome::Unordered { suites },
                    unread: Vec::new(),
                });
                continue;
            }
            Newest::Scan {
                scan,
                millis,
                unread,
            } => (Some((scan, millis)), unread),
        };
        let outcome = match chosen {
            None => ScanOutcome::NotRun,
            Some((scan, _)) if scan_is_vacuous(scan) == Some(true) => ScanOutcome::Vacuous {
                suite: scan.suite.as_str().to_owned(),
                timestamp: scan.timestamp.clone(),
            },
            Some((scan, millis)) => {
                let read = read_scan(tool, scan, &index);
                for (finding, hit) in read.advisories {
                    match hit {
                        Hit::One(record) => pending.push(Pending {
                            tool,
                            record,
                            finding,
                            scan_timestamp: scan.timestamp.clone(),
                            scan_millis: millis,
                        }),
                        Hit::Not(reason, candidates) => {
                            could_belong.extend(candidates.into_iter().map(|c| (tool, c)));
                            unmatched.push(Unmatched {
                                tool,
                                rule_id: finding.rule_id,
                                package: finding.path,
                                severity: finding.severity,
                                reason,
                            });
                        }
                    }
                }
                ScanOutcome::Read {
                    suite: scan.suite.as_str().to_owned(),
                    timestamp: scan.timestamp.clone(),
                    advisories: read.distinct,
                    set_aside: read.set_aside,
                }
            }
        };
        states.push(ScanState {
            tool,
            ruleset: chosen.and_then(|(scan, _)| scan.ruleset.clone()),
            outcome,
            unread,
        });
    }

    let mut attached = attach(records, &pending, discharge);
    let read_tools: Vec<Tool> = states
        .iter()
        .filter(|state| matches!(state.outcome, ScanOutcome::Read { .. }))
        .map(|state| state.tool)
        .collect();
    let mut joined = record_joins(records, &mut attached, &read_tools, &could_belong);
    joined.sort_by(|a, b| (&a.purl, &a.path).cmp(&(&b.purl, &b.path)));
    unmatched
        .sort_by(|a, b| (a.tool, &a.rule_id, &a.package).cmp(&(b.tool, &b.rule_id, &b.package)));
    Join {
        scans: states,
        records: joined,
        unmatched,
    }
}

/// Resolve every matched advisory against its record's clause. The scan time an
/// advisory is held to is that of the newest scan reporting it on its record,
/// over both tools.
fn attach(
    records: &[Existing],
    pending: &[Pending],
    discharge: &DischargeReport,
) -> BTreeMap<usize, Vec<Attached>> {
    // The newest scan reporting one advisory on one record, over both tools.
    let mut newest_report: BTreeMap<(usize, &str), i64> = BTreeMap::new();
    for item in pending {
        newest_report
            .entry((item.record.0, item.finding.rule_id.as_str()))
            .and_modify(|seen| *seen = (*seen).max(item.scan_millis))
            .or_insert(item.scan_millis);
    }

    let mut attached: BTreeMap<usize, Vec<Attached>> = BTreeMap::new();
    for item in pending {
        let Some(record) = records.get(item.record.0) else {
            continue;
        };
        let scan_millis = newest_report
            .get(&(item.record.0, item.finding.rule_id.as_str()))
            .copied()
            .unwrap_or(item.scan_millis);
        attached.entry(item.record.0).or_default().push(Attached {
            tool: item.tool,
            rule_id: item.finding.rule_id.clone(),
            severity: item.finding.severity.clone(),
            message: item.finding.message.clone(),
            package: item.finding.path.clone().unwrap_or_default(),
            scan_timestamp: item.scan_timestamp.clone(),
            resolution: resolve(record.disposition_clause.as_deref(), discharge, scan_millis),
        });
    }

    attached
}

/// One [`RecordJoin`] per record: its advisories, and the read tools it is free of.
fn record_joins(
    records: &[Existing],
    attached: &mut BTreeMap<usize, Vec<Attached>>,
    read_tools: &[Tool],
    could_belong: &BTreeSet<(Tool, usize)>,
) -> Vec<RecordJoin> {
    records
        .iter()
        .enumerate()
        .map(|(position, record)| {
            let mut advisories = attached.remove(&position).unwrap_or_default();
            advisories.sort_by(|a, b| {
                (&a.rule_id, &a.package, a.tool).cmp(&(&b.rule_id, &b.package, b.tool))
            });
            let free_of = read_tools
                .iter()
                .copied()
                .filter(|tool| {
                    tool.assesses(&record.purl)
                        && advisories.iter().all(|a| a.tool != *tool)
                        && !could_belong.contains(&(*tool, position))
                })
                .collect();
            RecordJoin {
                path: record.path.clone(),
                id: record.id.clone(),
                purl: record.purl.clone(),
                clause: record.disposition_clause.clone(),
                advisories,
                free_of,
            }
        })
        .collect()
}

/// An advisory matched to one record, before its resolution is decided.
struct Pending {
    tool: Tool,
    record: RecordRef,
    finding: Finding,
    scan_timestamp: String,
    scan_millis: i64,
}

/// What the retained scans of one tool come to.
pub(super) enum Newest<'a> {
    /// None is retained.
    None,
    /// A scan has a timestamp that is not an instant: the suites of those.
    Unordered(Vec<String>),
    /// The scan with the newest instant, whatever its file name; of two at one
    /// instant, the one whose suite id sorts last. `unread` is every other
    /// suite of the tool.
    Scan {
        scan: &'a FindingRecord,
        millis: i64,
        unread: Vec<String>,
    },
}

pub(super) fn newest(
    scans: &[FindingRecord],
    is_tool: impl Fn(&FindingRecord) -> bool,
) -> Newest<'_> {
    let of_tool: Vec<&FindingRecord> = scans.iter().filter(|scan| is_tool(scan)).collect();
    let mut unordered: Vec<String> = of_tool
        .iter()
        .filter(|scan| instant_epoch_millis(&scan.timestamp).is_none())
        .map(|scan| scan.suite.as_str().to_owned())
        .collect();
    if !unordered.is_empty() {
        unordered.sort();
        return Newest::Unordered(unordered);
    }
    let chosen = of_tool
        .iter()
        .filter_map(|scan| Some((instant_epoch_millis(&scan.timestamp)?, *scan)))
        .max_by(|a, b| (a.0, a.1.suite.as_str()).cmp(&(b.0, b.1.suite.as_str())));
    let Some((millis, scan)) = chosen else {
        return Newest::None;
    };
    let mut unread: Vec<String> = of_tool
        .iter()
        .filter(|other| !std::ptr::eq(**other, scan))
        .map(|other| other.suite.as_str().to_owned())
        .collect();
    unread.sort();
    Newest::Scan {
        scan,
        millis,
        unread,
    }
}

/// Milliseconds since the epoch of an instant in the grammar the discharge gate
/// reads `attestedAt` in, so a scan and an attestation are ordered by one rule.
pub(super) fn instant_epoch_millis(value: &str) -> Option<i64> {
    Instant::parse(value, InstantGrammar::CalendarInstant)?.epoch_millis()
}

/// What reading one scan produced.
struct Read {
    advisories: Vec<(Finding, Hit)>,
    distinct: usize,
    set_aside: usize,
}

/// Each distinct finding once, split into advisories (each matched) and the
/// diagnostics that are not advisories (counted).
fn read_scan(tool: Tool, scan: &FindingRecord, index: &RecordIndex) -> Read {
    let mut seen: BTreeSet<(&str, Option<&str>)> = BTreeSet::new();
    let mut advisories = Vec::new();
    let mut set_aside = 0_usize;
    for finding in &scan.findings {
        if !seen.insert((finding.rule_id.as_str(), finding.path.as_deref())) {
            continue;
        }
        if tool.is_advisory(finding) {
            let hit = match tool {
                Tool::OsvScanner => index.find_osv(finding.path.as_deref()),
                Tool::CargoAudit | Tool::CargoDeny => index.find(finding.path.as_deref()),
            };
            advisories.push((finding.clone(), hit));
        } else if !is_policy(tool, finding) {
            set_aside = set_aside.saturating_add(1);
        }
    }
    Read {
        distinct: advisories.len(),
        advisories,
        set_aside,
    }
}

/// Decide one attached advisory against the fact under its record's clause.
pub(super) fn resolve(
    clause: Option<&str>,
    discharge: &DischargeReport,
    scan_millis: i64,
) -> Resolution {
    let Some(clause) = clause else {
        return Resolution::Unresolved(Unresolved::NoDispositionClause);
    };
    let Some(entry) = find_clause(discharge, clause) else {
        return Resolution::Unresolved(Unresolved::ClauseNotInBindingReport);
    };
    let reason = |fallback: &str| entry.reason.clone().unwrap_or_else(|| fallback.to_owned());
    let fact = match (entry.state, entry.fact.as_ref()) {
        (DischargeState::Unresolved, _) => {
            return Resolution::Unresolved(Unresolved::ClauseDoesNotBind(reason(
                "applicability is unresolved",
            )));
        }
        (DischargeState::NotBinding, _) => {
            return Resolution::Unresolved(Unresolved::ClauseDoesNotBind(reason(
                "the clause does not apply",
            )));
        }
        (_, None) => return Resolution::Unresolved(Unresolved::NoFact),
        (_, Some(DischargeFact::Direct(_))) => {
            return Resolution::Unresolved(Unresolved::NotApprovedDisposition);
        }
        (DischargeState::Open, Some(_)) => {
            return Resolution::Unresolved(Unresolved::NotCurrent(reason("not current")));
        }
        (_, Some(DischargeFact::Disposition(fact))) => fact,
    };
    // The discharge gate already refused an `attestedAt` that is not an instant.
    let Some(attested) = instant_epoch_millis(&fact.attestation.attested_at) else {
        return Resolution::Unresolved(Unresolved::NotCurrent(
            "attestation is not an instant".to_owned(),
        ));
    };
    if attested < scan_millis {
        return Resolution::Unresolved(Unresolved::ScanNewerThanAttestation);
    }
    Resolution::Resolved(Disposition {
        decision: fact.decision,
        rationale: fact.rationale.clone(),
        approval_ref: fact.approval_ref.clone(),
        attested_by: fact.attestation.attested_by.clone(),
    })
}

/// The report's entry for a clause, in whichever population it landed.
fn find_clause<'a>(discharge: &'a DischargeReport, clause: &str) -> Option<&'a ClauseDischarge> {
    discharge
        .binding
        .direct
        .iter()
        .chain(&discharge.binding.dispositions)
        .chain(&discharge.binding.open)
        .chain(&discharge.unresolved)
        .chain(&discharge.not_binding)
        .find(|entry| entry.clause_id == clause)
}

#[cfg(test)]
pub(in crate::soup) mod tests;

#[cfg(test)]
pub(in crate::soup) mod osv_tests;
