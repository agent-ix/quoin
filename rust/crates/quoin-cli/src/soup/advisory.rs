// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The advisory join (FR-142): `cargo-audit` and `cargo-deny` findings laid
//! onto `soup_record` entries at read time.
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
//! carries no purl, so the match is the crate `<name>@<version>` its path
//! records against the record's `pkg:cargo/<name>@<version>` purl. A scanner's
//! severity word is carried unchanged and nothing is scored (FR-034-CON-2).

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use quoin_assurance::{
    ClauseDischarge, DischargeFact, DischargeReport, DischargeState, DispositionDecision,
    instant_epoch_millis,
};
use quoin_evidence::adapters::{AdapterSelection, select_finding_adapter};
use quoin_evidence::store::scan_is_vacuous;
use quoin_evidence::types::{Finding, FindingRecord};

use super::plan::{Existing, parse_purl};

/// The rule-id prefixes of a `cargo-deny` diagnostic that is an advisory. Any
/// other `cargo-deny` diagnostic (ban, licence, source, `unclassified`) is not.
const DENY_ADVISORY_PREFIXES: [&str; 3] = ["RUSTSEC-", "GHSA-", "CVE-"];

/// A scanner the join reads, in the order the join lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Tool {
    /// `cargo audit --json`.
    CargoAudit,
    /// `cargo deny --format json check`.
    CargoDeny,
}

impl Tool {
    const ALL: [Self; 2] = [Self::CargoAudit, Self::CargoDeny];

    /// The registry name of the tool's adapter.
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::CargoAudit => "cargo-audit",
            Self::CargoDeny => "cargo-deny",
        }
    }

    /// The tool a retained scan belongs to, chosen the way recording chooses its
    /// adapter. `None` for a scan of any other tool.
    fn of(scan: &FindingRecord) -> Option<Self> {
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
        }
    }
}

/// What the join found of one tool's newest retained scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ScanState {
    /// The tool.
    pub(super) tool: Tool,
    /// What became of its scan.
    pub(super) outcome: ScanOutcome,
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
    /// The newest scan ran with rules, and was read.
    Read {
        /// The scan's suite.
        suite: String,
        /// The scan's timestamp.
        timestamp: String,
        /// Distinct advisories read: attached plus unmatched. A repeated
        /// finding with one rule id and path counts once.
        advisories: usize,
        /// Distinct diagnostics that are not advisories, which the join does
        /// not read (`cargo-deny` ban, licence, source and `unclassified`).
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
    /// A timestamp could not be ordered, so no attestation can be said to
    /// follow the scan.
    #[error("the scan timestamp or the attestation is not an instant")]
    UnorderedInstants,
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
    /// left no ambiguous finding that could belong to it. The only way a
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
    /// One entry per tool, `cargo-audit` first.
    pub(super) scans: Vec<ScanState>,
    /// Every record, sorted by purl then path.
    pub(super) records: Vec<RecordJoin>,
    /// Findings attached to no record, sorted by tool, rule id, package.
    pub(super) unmatched: Vec<Unmatched>,
}

/// Join the newest retained `cargo-audit` and `cargo-deny` scans onto `records`
/// (FR-142).
///
/// `scans` is every retained finding-shaped record the store holds; the newest
/// by `timestamp` of each tool is read and the rest, and scans of other tools,
/// are ignored. `discharge` is the discharge report of the `--binding` report's
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
    let mut ambiguous_for: BTreeSet<(Tool, usize)> = BTreeSet::new();
    let mut states = Vec::new();

    for tool in Tool::ALL {
        let outcome = match newest(scans, tool) {
            None => ScanOutcome::NotRun,
            Some(scan) if scan_is_vacuous(scan) == Some(true) => ScanOutcome::Vacuous {
                suite: scan.suite.as_str().to_owned(),
                timestamp: scan.timestamp.clone(),
            },
            Some(scan) => {
                let read = read_scan(tool, scan, &index);
                for (finding, hit) in read.advisories {
                    match hit {
                        Hit::One(record) => pending.push(Pending {
                            tool,
                            record,
                            finding,
                            scan_timestamp: scan.timestamp.clone(),
                        }),
                        Hit::Not(reason, candidates) => {
                            ambiguous_for.extend(candidates.into_iter().map(|c| (tool, c)));
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
        states.push(ScanState { tool, outcome });
    }

    let mut attached = attach(records, &pending, discharge);
    let read_tools: Vec<Tool> = states
        .iter()
        .filter(|state| matches!(state.outcome, ScanOutcome::Read { .. }))
        .map(|state| state.tool)
        .collect();
    let mut joined: Vec<RecordJoin> = records
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
                    advisories.iter().all(|a| a.tool != *tool)
                        && !ambiguous_for.contains(&(*tool, position))
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
        .collect();
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
    let mut newest_report: BTreeMap<(usize, &str), Option<i64>> = BTreeMap::new();
    for item in pending {
        let millis = instant_epoch_millis(&item.scan_timestamp);
        newest_report
            .entry((item.record.0, item.finding.rule_id.as_str()))
            .and_modify(|seen| *seen = seen.zip(millis).map(|(a, b)| a.max(b)))
            .or_insert(millis);
    }

    let mut attached: BTreeMap<usize, Vec<Attached>> = BTreeMap::new();
    for item in pending {
        let Some(record) = records.get(item.record.0) else {
            continue;
        };
        let scan_millis = newest_report
            .get(&(item.record.0, item.finding.rule_id.as_str()))
            .copied()
            .flatten();
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

/// An advisory matched to one record, before its resolution is decided.
struct Pending {
    tool: Tool,
    record: RecordRef,
    finding: Finding,
    scan_timestamp: String,
}

/// A position in the `records` slice.
#[derive(Clone, Copy)]
struct RecordRef(usize);

/// The newest retained scan of `tool` by timestamp, whatever its file name. A
/// timestamp that is not an instant sorts before every one that is; a tie is
/// broken by suite so the choice never depends on input order.
fn newest(scans: &[FindingRecord], tool: Tool) -> Option<&FindingRecord> {
    scans
        .iter()
        .filter(|scan| Tool::of(scan) == Some(tool))
        .max_by(|a, b| {
            (
                instant_epoch_millis(&a.timestamp),
                &a.timestamp,
                a.suite.as_str(),
            )
                .cmp(&(
                    instant_epoch_millis(&b.timestamp),
                    &b.timestamp,
                    b.suite.as_str(),
                ))
        })
}

/// What reading one scan produced.
struct Read {
    advisories: Vec<(Finding, Hit)>,
    distinct: usize,
    set_aside: usize,
}

/// Where one advisory finding went.
enum Hit {
    /// To exactly one record.
    One(RecordRef),
    /// To none; the candidates are the records an ambiguous finding could be.
    Not(UnmatchedReason, Vec<usize>),
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
            let hit = index.find(finding.path.as_deref());
            advisories.push((finding.clone(), hit));
        } else {
            set_aside = set_aside.saturating_add(1);
        }
    }
    Read {
        distinct: advisories.len(),
        advisories,
        set_aside,
    }
}

/// Records by the identity a finding's path names: the purl without qualifiers
/// and subpath, and its version.
struct RecordIndex {
    by_identity: BTreeMap<(String, String), Vec<usize>>,
}

impl RecordIndex {
    fn new(records: &[Existing]) -> Self {
        let mut by_identity: BTreeMap<(String, String), Vec<usize>> = BTreeMap::new();
        for (position, record) in records.iter().enumerate() {
            if let Some(purl) = parse_purl(&record.purl)
                && let Some(version) = purl.version
            {
                by_identity
                    .entry((purl.base, version))
                    .or_default()
                    .push(position);
            }
        }
        Self { by_identity }
    }

    /// The record a finding path selects: `<name>@<version>` as the cargo crate
    /// it names, or a purl compared as it is.
    fn find(&self, path: Option<&str>) -> Hit {
        let Some(identity) = path.and_then(identity_of) else {
            return Hit::Not(UnmatchedReason::NoVersion, Vec::new());
        };
        match self.by_identity.get(&identity).map(Vec::as_slice) {
            None | Some([]) => Hit::Not(UnmatchedReason::NoRecord, Vec::new()),
            Some([only]) => Hit::One(RecordRef(*only)),
            Some(many) => Hit::Not(UnmatchedReason::Ambiguous, many.to_vec()),
        }
    }
}

/// The `(purl base, version)` a finding path names. `None` when it names no
/// version.
fn identity_of(path: &str) -> Option<(String, String)> {
    if path.starts_with("pkg:") {
        let purl = parse_purl(path)?;
        return Some((purl.base, purl.version?));
    }
    let (name, version) = path.rsplit_once('@')?;
    (!name.is_empty() && !version.is_empty())
        .then(|| (format!("pkg:cargo/{name}"), version.to_owned()))
}

/// Decide one attached advisory against the fact under its record's clause.
fn resolve(
    clause: Option<&str>,
    discharge: &DischargeReport,
    scan_millis: Option<i64>,
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
    let attested = instant_epoch_millis(&fact.attestation.attested_at);
    match attested.zip(scan_millis) {
        None => Resolution::Unresolved(Unresolved::UnorderedInstants),
        Some((attested, scan)) if attested < scan => {
            Resolution::Unresolved(Unresolved::ScanNewerThanAttestation)
        }
        Some(_) => Resolution::Resolved(Disposition {
            decision: fact.decision,
            rationale: fact.rationale.clone(),
            approval_ref: fact.approval_ref.clone(),
            attested_by: fact.attestation.attested_by.clone(),
        }),
    }
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
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use std::path::{Path, PathBuf};

    use ix_trace_rs::trace;
    use quoin_assurance::{
        BuildDischargeRequest, DischargeReport, DispositionDecision, build_discharge_report,
    };
    use quoin_evidence::adapters::{AdapterSelection, parse_cargo_deny, select_finding_adapter};
    use quoin_evidence::types::FindingRecord;
    use quoin_evidence::{STORE_SCHEMA_VERSION, SuiteId};
    use serde_json::{Value, json};

    use super::{
        Join, RecordJoin, Resolution, ScanOutcome, Tool, Unmatched, UnmatchedReason, Unresolved,
        join,
    };
    use crate::soup::plan::Existing;

    const BEFORE: &str = "2026-07-31T00:00:00.000Z";
    const AFTER: &str = "2026-08-02T00:00:00.000Z";

    fn evidence_fixtures() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tests/fixtures/evidence")
    }

    /// The real `cargo audit --json` output, as a document to extend.
    fn audit_document() -> Value {
        let raw =
            std::fs::read_to_string(evidence_fixtures().join("cargo-audit-real.json")).unwrap();
        serde_json::from_str(&raw).unwrap()
    }

    fn audit_entry(id: &str, name: &str, version: &str) -> Value {
        json!({
            "advisory": { "id": id, "title": format!("{id} title") },
            "package": { "name": name, "version": version }
        })
    }

    /// The real fixture with its warnings replaced and `vulnerabilities`
    /// listing `vulns`; `warnings` is `(kind, entry)` pairs.
    fn audit_text(vulns: &[Value], warnings: &[(&str, Value)]) -> String {
        let mut document = audit_document();
        document["vulnerabilities"] = json!({
            "found": !vulns.is_empty(), "count": vulns.len(), "list": vulns
        });
        let mut by_kind = serde_json::Map::new();
        for (kind, entry) in warnings {
            by_kind
                .entry((*kind).to_owned())
                .or_insert_with(|| json!([]))
                .as_array_mut()
                .unwrap()
                .push(entry.clone());
        }
        document["warnings"] = Value::Object(by_kind);
        document.to_string()
    }

    /// One `cargo deny --format json check` diagnostic in the shape of the
    /// real `cargo-deny-real.jsonl` fixture.
    fn deny_diagnostic(
        key: Option<(&str, &str)>,
        name: &str,
        version: &str,
        severity: &str,
    ) -> Value {
        let mut fields = json!({
            "graphs": [{ "Krate": { "name": name, "version": version } }],
            "message": "a diagnostic",
            "severity": severity
        });
        match key {
            Some(("advisory", id)) => fields["advisory"] = json!({ "id": id }),
            Some((_, code)) => fields["code"] = json!(code),
            None => {}
        }
        json!({ "fields": fields, "type": "diagnostic" })
    }

    fn deny_text(diagnostics: &[Value]) -> String {
        let mut lines: Vec<String> = diagnostics.iter().map(Value::to_string).collect();
        lines.push(
            json!({ "fields": { "advisories": { "errors": 0, "warnings": 0 } }, "type": "summary" })
                .to_string(),
        );
        lines.join("\n")
    }

    /// A retained scan, made the way recording makes one: through the adapter
    /// the registry selects for `tool`.
    fn scan(tool: &str, suite: &str, timestamp: &str, raw: &str) -> FindingRecord {
        let adapter = select_finding_adapter(AdapterSelection {
            adapter: None,
            tool: Some(tool),
        })
        .unwrap();
        let parsed = (adapter.parse)(raw).unwrap();
        FindingRecord {
            schema_version: STORE_SCHEMA_VERSION,
            suite: SuiteId::new(suite),
            tool: tool.to_owned(),
            evidence_kind: None,
            timestamp: timestamp.to_owned(),
            ruleset: parsed.ruleset,
            rules_evaluated: parsed.rules_evaluated,
            findings: parsed.findings,
        }
    }

    fn record(purl: &str, clause: Option<&str>) -> Existing {
        Existing {
            path: PathBuf::from(format!("{purl}.md")),
            purl: purl.to_owned(),
            id: None,
            disposition_clause: clause.map(str::to_owned),
        }
    }

    fn attestation(at: &str, until: &str) -> Value {
        json!({
            "attestedBy": "reviewer-1", "authority": "quality-lead",
            "attestedAt": at, "expiresAt": until
        })
    }

    fn disposition(clause: &str, at: &str, until: &str) -> Value {
        json!({
            "kind": "disposition", "clauseId": clause, "decision": "accepted_risk",
            "rationale": "no reachable path", "approvalRef": "approval://CAB-7",
            "attestation": attestation(at, until)
        })
    }

    /// A discharge report over one clause set at 2026-08-15: `C-DISP` holds a
    /// current disposition attested 2026-08-01, `C-OLD` an expired one,
    /// `C-DIRECT` a direct fact, `C-NONE` no fact, `C-NB` does not bind and
    /// `C-UNRES` has undecided applicability.
    fn discharge() -> DischargeReport {
        discharge_with(&disposition(
            "C-DISP",
            "2026-08-01T00:00:00.000Z",
            "2026-09-01T00:00:00.000Z",
        ))
    }

    fn discharge_with(fact: &Value) -> DischargeReport {
        let binding_clause = |id: &str, outcome: &str, reasons: Value| {
            json!({ "clauseId": id, "force": "mandatory", "outcome": outcome,
                    "reasons": reasons, "expectedOutputs": [] })
        };
        let request: BuildDischargeRequest = serde_json::from_value(json!({
            "binding": {
                "schemaVersion": "clause-binding-v1",
                "clauseSet": { "authority": "example.invalid", "id": "soup", "version": "1.0.0" },
                "context": { "product": "widget" },
                "clauses": [
                    binding_clause("C-DISP", "binding", json!([])),
                    binding_clause("C-OLD", "binding", json!([])),
                    binding_clause("C-DIRECT", "binding", json!([])),
                    binding_clause("C-NONE", "binding", json!([])),
                    binding_clause("C-NB", "not_binding", json!([])),
                    binding_clause("C-UNRES", "unresolved", json!([{
                        "code": "unknown-context", "message": "deployment is not declared"
                    }]))
                ]
            },
            "facts": [
                fact,
                disposition("C-OLD", "2026-01-01T00:00:00.000Z", "2026-02-01T00:00:00.000Z"),
                {
                    "kind": "direct", "clauseId": "C-DIRECT",
                    "evidenceRefs": ["evidence://run/one"],
                    "attestation": attestation("2026-08-01T00:00:00.000Z", "2026-09-01T00:00:00.000Z")
                }
            ],
            "asOf": "2026-08-15T00:00:00.000Z"
        }))
        .unwrap();
        build_discharge_report(&request).unwrap()
    }

    fn anyhow_audit(timestamp: &str) -> FindingRecord {
        scan(
            "cargo-audit",
            "audit",
            timestamp,
            &audit_text(
                &[],
                &[(
                    "unsound",
                    audit_entry("RUSTSEC-2026-0190", "anyhow", "1.0.102"),
                )],
            ),
        )
    }

    fn only_record<'a>(joined: &'a Join, purl: &str) -> &'a RecordJoin {
        joined.records.iter().find(|r| r.purl == purl).unwrap()
    }

    fn state(joined: &Join, tool: Tool) -> &ScanOutcome {
        &joined
            .scans
            .iter()
            .find(|s| s.tool == tool)
            .unwrap()
            .outcome
    }

    /// Distinct advisories and set-aside diagnostics of a scan that was read.
    fn read_counts(outcome: &ScanOutcome) -> Option<(usize, usize)> {
        match outcome {
            ScanOutcome::Read {
                advisories,
                set_aside,
                ..
            } => Some((*advisories, *set_aside)),
            _ => None,
        }
    }

    #[trace("FR-142-AC-1", "FR-142-AC-9")]
    #[test]
    fn tc_1516_001_a_cargo_audit_advisory_attaches_to_its_crate_only() {
        let records = [
            record("pkg:cargo/anyhow@1.0.102", None),
            record("pkg:cargo/anyhow@1.0.101", None),
            record("pkg:cargo/serde@1.0.228", None),
        ];
        let joined = join(&records, &[anyhow_audit(BEFORE)], &discharge());
        let hit = only_record(&joined, "pkg:cargo/anyhow@1.0.102");
        assert_eq!(hit.advisories.len(), 1);
        assert_eq!(hit.advisories[0].rule_id, "RUSTSEC-2026-0190");
        assert_eq!(hit.advisories[0].package, "anyhow@1.0.102");
        // The scanner's own word, not a class of ours.
        assert_eq!(hit.advisories[0].severity.as_deref(), Some("unsound"));
        assert!(
            only_record(&joined, "pkg:cargo/anyhow@1.0.101")
                .advisories
                .is_empty()
        );
        assert!(
            only_record(&joined, "pkg:cargo/serde@1.0.228")
                .advisories
                .is_empty()
        );
        assert!(joined.unmatched.is_empty());
    }

    #[trace("FR-142-AC-2")]
    #[test]
    fn tc_1516_002_cargo_deny_advisories_and_audit_warnings_attach() {
        let deny = scan(
            "cargo-deny",
            "deny",
            BEFORE,
            &deny_text(&[deny_diagnostic(
                Some(("advisory", "RUSTSEC-2024-1")),
                "a",
                "1.0.0",
                "error",
            )]),
        );
        let audit = scan(
            "cargo-audit",
            "audit",
            BEFORE,
            &audit_text(
                &[],
                &[("unmaintained", audit_entry("RUSTSEC-2025-2", "b", "2.0.0"))],
            ),
        );
        let records = [
            record("pkg:cargo/a@1.0.0", None),
            record("pkg:cargo/b@2.0.0", None),
        ];
        let joined = join(&records, &[deny, audit], &discharge());
        let a = &only_record(&joined, "pkg:cargo/a@1.0.0").advisories;
        assert_eq!(
            (a.len(), a[0].tool, a[0].severity.as_deref()),
            (1, Tool::CargoDeny, Some("error"))
        );
        let b = &only_record(&joined, "pkg:cargo/b@2.0.0").advisories;
        assert_eq!(
            (b.len(), b[0].tool, b[0].severity.as_deref()),
            (1, Tool::CargoAudit, Some("unmaintained"))
        );
    }

    #[trace("FR-142-AC-3")]
    #[test]
    fn tc_1516_003_unmatched_findings_are_listed_once_and_all_are_counted() {
        let audit = scan(
            "cargo-audit",
            "audit",
            BEFORE,
            &audit_text(
                &[
                    audit_entry("RUSTSEC-1", "known", "1.0.0"),
                    audit_entry("RUSTSEC-2", "stranger", "3.0.0"),
                    audit_entry("RUSTSEC-3", "twice", "1.0.0"),
                    // The same finding again counts once.
                    audit_entry("RUSTSEC-2", "stranger", "3.0.0"),
                ],
                &[(
                    "yanked",
                    json!({
                        "advisory": { "id": "RUSTSEC-4" },
                        "package": { "name": "unversioned" }
                    }),
                )],
            ),
        );
        let records = [
            record("pkg:cargo/known@1.0.0", None),
            record("pkg:cargo/twice@1.0.0", None),
            record("pkg:cargo/twice@1.0.0?arch=x86", None),
        ];
        let joined = join(&records, &[audit], &discharge());
        let reasons: Vec<(&str, Option<&str>, UnmatchedReason)> = joined
            .unmatched
            .iter()
            .map(|u: &Unmatched| (u.rule_id.as_str(), u.package.as_deref(), u.reason))
            .collect();
        assert_eq!(
            reasons,
            [
                (
                    "RUSTSEC-2",
                    Some("stranger@3.0.0"),
                    UnmatchedReason::NoRecord
                ),
                ("RUSTSEC-3", Some("twice@1.0.0"), UnmatchedReason::Ambiguous),
                ("RUSTSEC-4", Some("unversioned"), UnmatchedReason::NoVersion),
            ]
        );
        assert_eq!(UnmatchedReason::Ambiguous.to_string(), "ambiguous");
        let attached: usize = joined.records.iter().map(|r| r.advisories.len()).sum();
        let (advisories, _) = read_counts(state(&joined, Tool::CargoAudit)).unwrap();
        assert_eq!(attached + joined.unmatched.len(), advisories);
        assert_eq!(advisories, 4);
        // An ambiguous finding could be either record, so neither is free of it.
        assert!(
            only_record(&joined, "pkg:cargo/twice@1.0.0")
                .free_of
                .is_empty()
        );
    }

    #[trace("FR-142-AC-4", "FR-142-AC-5")]
    #[test]
    fn tc_1516_004_not_run_is_not_clean() {
        let records = [record("pkg:cargo/a@1.0.0", None)];
        let none = join(&records, &[], &discharge());
        let clean = join(
            &records,
            &[scan("cargo-audit", "audit", BEFORE, &audit_text(&[], &[]))],
            &discharge(),
        );
        assert_eq!(*state(&none, Tool::CargoAudit), ScanOutcome::NotRun);
        assert!(
            clean
                .scans
                .iter()
                .any(|s| s.tool == Tool::CargoAudit && s.outcome.is_clean())
        );
        assert!(!state(&none, Tool::CargoAudit).is_clean());
        assert!(none.records[0].free_of.is_empty());
        assert_eq!(clean.records[0].free_of, [Tool::CargoAudit]);
        assert_ne!(none, clean);

        // A deny log whose summary counts errors its diagnostics do not show is
        // refused at recording, so no scan is retained and deny is not run.
        let refused = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../quoin-evidence/tests/fixtures/cargo-deny-failed-real.jsonl"),
        )
        .unwrap();
        assert!(parse_cargo_deny(&refused).is_err());
        assert_eq!(*state(&clean, Tool::CargoDeny), ScanOutcome::NotRun);
        assert!(!clean.records[0].free_of.contains(&Tool::CargoDeny));
    }

    #[trace("FR-142-AC-6")]
    #[test]
    fn tc_1516_005_a_scan_that_evaluated_no_rules_proves_nothing() {
        let mut vacuous = scan("cargo-audit", "audit", BEFORE, &audit_text(&[], &[]));
        vacuous.rules_evaluated = Some(0);
        let joined = join(
            &[record("pkg:cargo/a@1.0.0", None)],
            &[vacuous],
            &discharge(),
        );
        assert!(matches!(
            state(&joined, Tool::CargoAudit),
            ScanOutcome::Vacuous { .. }
        ));
        assert!(!state(&joined, Tool::CargoAudit).is_clean());
        assert!(joined.records[0].free_of.is_empty());
    }

    fn resolved_for(clause: &str, scan_at: &str) -> Resolution {
        let records = [record("pkg:cargo/anyhow@1.0.102", Some(clause))];
        let joined = join(&records, &[anyhow_audit(scan_at)], &discharge());
        joined.records[0].advisories[0].resolution.clone()
    }

    #[trace("FR-142-AC-7")]
    #[test]
    fn tc_1516_006_a_current_disposition_at_or_after_the_scan_resolves() {
        let found = match resolved_for("C-DISP", BEFORE) {
            Resolution::Resolved(found) => Some(found),
            Resolution::Unresolved(_) => None,
        }
        .unwrap();
        assert_eq!(found.decision, DispositionDecision::AcceptedRisk);
        assert_eq!(found.rationale, "no reachable path");
        assert_eq!(found.approval_ref, "approval://CAB-7");
        assert_eq!(found.attested_by, "reviewer-1");
        // At the same instant is still resolved.
        assert!(matches!(
            resolved_for("C-DISP", "2026-08-01T00:00:00.000Z"),
            Resolution::Resolved(_)
        ));
    }

    #[trace("FR-142-AC-8")]
    #[test]
    fn tc_1516_007_each_way_a_disposition_fails_names_itself() {
        let why = |clause: &str| {
            match resolved_for(clause, BEFORE) {
                Resolution::Unresolved(reason) => Some(reason),
                Resolution::Resolved(_) => None,
            }
            .unwrap()
        };
        assert!(matches!(why("C-OLD"), Unresolved::NotCurrent(text) if text.contains("expired")));
        assert_eq!(why("C-NONE"), Unresolved::NoFact);
        assert_eq!(why("C-DIRECT"), Unresolved::NotApprovedDisposition);
        assert!(matches!(why("C-NB"), Unresolved::ClauseDoesNotBind(_)));
        assert!(matches!(
            why("C-UNRES"),
            Unresolved::ClauseDoesNotBind(text) if text.contains("deployment is not declared")
        ));
        assert_eq!(why("C-ABSENT"), Unresolved::ClauseNotInBindingReport);

        let records = [record("pkg:cargo/anyhow@1.0.102", None)];
        let joined = join(&records, &[anyhow_audit(BEFORE)], &discharge());
        assert_eq!(
            joined.records[0].advisories[0].resolution,
            Resolution::Unresolved(Unresolved::NoDispositionClause)
        );
        assert_eq!(
            Unresolved::NoDispositionClause.to_string(),
            "no disposition clause"
        );
    }

    #[trace("FR-142-AC-12")]
    #[test]
    fn tc_1516_008_a_newer_scan_unresolves_every_advisory_until_attested_again() {
        let audit_two = |at: &str| {
            scan(
                "cargo-audit",
                "audit",
                at,
                &audit_text(
                    &[
                        audit_entry("RUSTSEC-A", "x", "1.0.0"),
                        audit_entry("RUSTSEC-B", "x", "1.0.0"),
                    ],
                    &[],
                ),
            )
        };
        let records = [record("pkg:cargo/x@1.0.0", Some("C-DISP"))];
        let newer = join(&records, &[audit_two(AFTER)], &discharge());
        assert_eq!(newer.records[0].advisories.len(), 2);
        for advisory in &newer.records[0].advisories {
            assert_eq!(
                advisory.resolution,
                Resolution::Unresolved(Unresolved::ScanNewerThanAttestation)
            );
        }
        assert_eq!(
            Unresolved::ScanNewerThanAttestation.to_string(),
            "scan newer than attestation"
        );
        // Attesting again after the scan resolves both.
        let again = discharge_with(&disposition(
            "C-DISP",
            "2026-08-03T00:00:00.000Z",
            "2026-09-01T00:00:00.000Z",
        ));
        let renewed = join(&records, &[audit_two(AFTER)], &again);
        assert!(
            renewed.records[0]
                .advisories
                .iter()
                .all(|a| matches!(a.resolution, Resolution::Resolved(_)))
        );
    }

    #[trace("FR-142-AC-13")]
    #[test]
    fn tc_1516_009_deny_diagnostics_that_are_not_advisories_are_set_aside() {
        let deny = scan(
            "cargo-deny",
            "deny",
            BEFORE,
            &deny_text(&[
                deny_diagnostic(Some(("code", "rejected")), "a", "1.0.0", "error"),
                deny_diagnostic(Some(("code", "banned")), "a", "1.0.0", "error"),
                deny_diagnostic(None, "a", "1.0.0", "warning"),
                deny_diagnostic(
                    Some(("advisory", "GHSA-xxxx-yyyy-zzzz")),
                    "a",
                    "1.0.0",
                    "error",
                ),
                deny_diagnostic(Some(("advisory", "CVE-2026-1")), "a", "1.0.0", "error"),
            ]),
        );
        let joined = join(&[record("pkg:cargo/a@1.0.0", None)], &[deny], &discharge());
        assert_eq!(read_counts(state(&joined, Tool::CargoDeny)), Some((2, 3)));
        assert_eq!(joined.records[0].advisories.len(), 2);
        assert!(joined.unmatched.is_empty());
    }

    #[trace("FR-142-AC-14", "FR-142-AC-10", "FR-142-CON-1")]
    #[test]
    fn tc_1516_010_only_the_crate_selects_and_no_record_file_changes() {
        let directory = tempfile::tempdir().unwrap();
        let body = |purl: &str, extra: &str| {
            format!(
                "---\nid: SOUP-1\ntype: soup_record\npurl: \"{purl}\"\nname: anyhow\n\
                 disposition_clause: C-DISP\n{extra}---\n# body\n"
            )
        };
        let chosen = directory.path().join("anyhow.md");
        let other = directory.path().join("other.md");
        std::fs::write(
            &chosen,
            body(
                "pkg:cargo/anyhow@1.0.102",
                "finding_refs: [\"RUSTSEC-2026-9999\"]\nvulnerability: \"ix://vuln/V-1\"\n",
            ),
        )
        .unwrap();
        std::fs::write(
            &other,
            body(
                "pkg:cargo/serde@1.0.228",
                "finding_refs: [\"RUSTSEC-2026-0190\"]\n",
            ),
        )
        .unwrap();
        let before = (
            std::fs::read(&chosen).unwrap(),
            std::fs::read(&other).unwrap(),
        );
        let records = crate::soup::read_records(directory.path()).ok().unwrap();

        // The same finding twice, as a re-run may report it.
        let mut doubled = anyhow_audit(BEFORE);
        doubled.findings.extend(doubled.findings.clone());
        let joined = join(&records, &[doubled], &discharge());

        assert_eq!(
            only_record(&joined, "pkg:cargo/anyhow@1.0.102")
                .advisories
                .len(),
            1
        );
        assert!(
            only_record(&joined, "pkg:cargo/serde@1.0.228")
                .advisories
                .is_empty()
        );
        assert!(joined.unmatched.is_empty());
        assert_eq!(
            before,
            (
                std::fs::read(&chosen).unwrap(),
                std::fs::read(&other).unwrap()
            )
        );
    }

    #[trace("FR-142-AC-15")]
    #[test]
    fn tc_1516_011_only_the_scan_with_the_newer_timestamp_is_read() {
        let old = scan(
            "cargo-audit",
            "zzz-newest-name",
            BEFORE,
            &audit_text(&[audit_entry("RUSTSEC-OLD", "a", "1.0.0")], &[]),
        );
        let new = scan(
            "cargo-audit",
            "aaa-oldest-name",
            AFTER,
            &audit_text(&[audit_entry("RUSTSEC-NEW", "a", "1.0.0")], &[]),
        );
        let records = [record("pkg:cargo/a@1.0.0", None)];
        for scans in [[old.clone(), new.clone()], [new, old]] {
            let joined = join(&records, &scans, &discharge());
            let ids: Vec<&str> = joined.records[0]
                .advisories
                .iter()
                .map(|a| a.rule_id.as_str())
                .collect();
            assert_eq!(ids, ["RUSTSEC-NEW"]);
        }
    }

    #[test]
    fn a_purl_path_matches_by_purl_ignoring_qualifiers() {
        let mut audit = anyhow_audit(BEFORE);
        audit.findings[0].path = Some("pkg:cargo/anyhow@1.0.102?arch=x86#sub".to_owned());
        let joined = join(
            &[record("pkg:cargo/anyhow@1.0.102", None)],
            &[audit],
            &discharge(),
        );
        assert_eq!(joined.records[0].advisories.len(), 1);
    }
}
