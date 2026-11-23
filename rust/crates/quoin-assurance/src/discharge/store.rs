// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Discharge facts in the evidence store (FR-133).
//!
//! # Facts are persisted, obligations are not
//!
//! A discharge fact is a statement a person made at a point in time: this
//! clause is discharged by that evidence, or disposed of by this decision. The
//! attestation and, for temporary exceptions, the decision each carry an
//! expiry. Nothing can re-derive it, so it is
//! stored. Which clauses bind is the opposite: Quire decides it from the clause
//! set and the declared context every time (`clause-binding-v1`), so the store
//! holds no clause list, no force, no expected output and no applicability.
//! The fact is keyed by the clause set it was made against and its clause id;
//! the partition into evidence, disposition and open work is computed from the
//! live binding report on every read, which is what keeps it from drifting out
//! of agreement with the clause set (FR-030, governing principle).
//!
//! # One file per fact, last write wins
//!
//! `discharge/<authority>/<set>/<version>/<clause>.json`, each path component
//! encoded by [`quoin_evidence::paths::record_stem`] so no component can
//! escape its directory. Recording a clause again replaces its fact: that is
//! how an expired attestation is renewed, and git history is the log of what
//! it replaced. A fact is never merged with another.

use std::collections::{BTreeMap, BTreeSet};

use quoin_evidence::EvidenceError;
use quoin_evidence::paths::record_stem;
use quoin_evidence::source::EvidenceSource;
use quoin_quire_types::{ClauseBindingReport, ClauseSetKey, SetPolicy};
use serde::{Deserialize, Serialize};

use super::build::build_discharge_report_with_approval_requirements;
use super::completeness::{InwardCompleteness, build_inward_completeness};
use super::parse::parse_fact_with_approval_requirements;
use super::report::{BuildDischargeRequest, DischargeError, DischargeFact, DischargeReport};
use super::verify::{CheckError, CheckedReport, DischargeChecks, check_facts, check_report};

/// Where discharge facts live, under the store root.
pub const DISCHARGE_DIR: &str = "discharge";

/// The most clause ids, authority, set id or version bytes a path component
/// may name (FR-130's identifier bound).
pub const MAX_ID_BYTES: usize = 64;

/// The most stored fact files read for one clause set.
pub const MAX_STORED_FACTS: usize = 10_000;

/// The largest single stored fact file, in bytes.
pub const MAX_STORED_FACT_BYTES: usize = 1024 * 1024;

/// The one accepted `schemaVersion` of a stored discharge fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum StoredFactSchemaVersion {
    /// `discharge-fact-record-v1`.
    #[serde(rename = "discharge-fact-record-v1")]
    V1,
}

/// One discharge fact as the store holds it.
///
/// The clause set key is stored so a read can check the file belongs where its
/// path says; nothing else about the clause set is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StoredDischargeFact {
    /// Always `discharge-fact-record-v1`.
    pub schema_version: StoredFactSchemaVersion,
    /// The clause set the fact was made against.
    pub clause_set: ClauseSetKey,
    /// The fact itself, validated on every read by the same predicates a
    /// request goes through.
    pub fact: DischargeFact,
}

/// The persisted wire shape before policy-aware fact validation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct StoredDischargeFactValue {
    #[serde(rename = "schemaVersion")]
    _schema_version: StoredFactSchemaVersion,
    clause_set: ClauseSetKey,
    fact: serde_json::Value,
}

/// What `record_discharge_facts` is given.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RecordDischargeRequest {
    /// A validated `clause-binding-v1` report. Only its clause set key is
    /// stored; clause ids must be unique in it.
    pub binding: ClauseBindingReport,
    /// Unvalidated discharge facts, as `quoin discharge --facts` takes them.
    pub facts: Vec<serde_json::Value>,
}

/// One fact that was written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RecordedFact {
    /// The clause the fact is about.
    pub clause_id: String,
    /// Where it was written, relative to the store root.
    pub path: String,
}

/// Why the store refused a list of facts before writing any of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreRefusal {
    /// An id is empty or over [`MAX_ID_BYTES`].
    InvalidIdentifier,
    /// Two ids fold to one file name, or a name is held by a different id.
    CaseCollision,
    /// More facts than [`MAX_STORED_FACTS`].
    TooManyFacts,
}

impl StoreRefusal {
    /// The kind's spelling, as the error envelope carries it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidIdentifier => "invalid-identifier",
            Self::CaseCollision => "case-collision",
            Self::TooManyFacts => "too-many-facts",
        }
    }
}

/// Why a discharge store operation failed.
#[derive(Debug)]
pub enum DischargeStoreError {
    /// The facts, the binding report or the instant were refused by the FR-046
    /// predicates.
    Discharge(DischargeError),
    /// The store refused the list by name, before writing.
    Refused {
        /// Which rule refused it.
        kind: StoreRefusal,
        /// The offending component, in words.
        detail: String,
    },
    /// The store could not be read or written.
    Evidence(EvidenceError),
    /// engineering-assurance refused the allowed-signers file.
    Check(CheckError),
    /// The list holds a fact whose evidence or approval does not hold, so none
    /// of it was written. Each entry names the clause and the reason.
    FactsFailed(Vec<super::verify::CheckFailure>),
}

impl std::fmt::Display for DischargeStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Discharge(error) => error.fmt(f),
            Self::Refused { kind, detail } => write!(f, "{} ({detail})", kind.as_str()),
            Self::Evidence(error) => error.fmt(f),
            Self::Check(error) => error.fmt(f),
            Self::FactsFailed(failures) => {
                let listed = failures
                    .iter()
                    .map(|failure| format!("{}: {}", failure.clause_id, failure.reason))
                    .collect::<Vec<_>>()
                    .join("; ");
                write!(f, "no fact was written: {listed}")
            }
        }
    }
}

impl std::error::Error for DischargeStoreError {}

impl From<DischargeError> for DischargeStoreError {
    fn from(error: DischargeError) -> Self {
        Self::Discharge(error)
    }
}

impl From<CheckError> for DischargeStoreError {
    fn from(error: CheckError) -> Self {
        Self::Check(error)
    }
}

impl From<EvidenceError> for DischargeStoreError {
    fn from(error: EvidenceError) -> Self {
        Self::Evidence(error)
    }
}

fn refuse<T>(kind: StoreRefusal, detail: String) -> Result<T, DischargeStoreError> {
    Err(DischargeStoreError::Refused { kind, detail })
}

/// One path component: bounded, escaped so it cannot leave its directory, and
/// lowercased so a case-insensitive filesystem cannot alias two ids.
fn component(what: &str, raw: &str) -> Result<String, DischargeStoreError> {
    if raw.is_empty() || raw.len() > MAX_ID_BYTES {
        return refuse(
            StoreRefusal::InvalidIdentifier,
            format!(
                "{what} must be 1 to {MAX_ID_BYTES} bytes, got {}",
                raw.len()
            ),
        );
    }
    Ok(record_stem(raw).to_ascii_lowercase())
}

fn clause_set_dir(set: &ClauseSetKey) -> Result<String, DischargeStoreError> {
    Ok(format!(
        "{DISCHARGE_DIR}/{}/{}/{}",
        component("clause set authority", &set.authority)?,
        component("clause set id", &set.id)?,
        component("clause set version", &set.version)?
    ))
}

fn fact_path(set: &ClauseSetKey, clause_id: &str) -> Result<String, DischargeStoreError> {
    Ok(format!(
        "{}/{}.json",
        clause_set_dir(set)?,
        component("clause id", clause_id)?
    ))
}

fn canonical(record: &StoredDischargeFact) -> Result<Vec<u8>, EvidenceError> {
    let canonicalization = |detail: String| EvidenceError::Canonicalization {
        what: "discharge fact",
        detail,
    };
    let text = serde_json::to_string(record).map_err(|e| canonicalization(e.to_string()))?;
    let parsed =
        quoin_store::parse_strict_json_str(&text).map_err(|e| canonicalization(e.to_string()))?;
    quoin_store::canonical_json_bytes(&parsed).map_err(|e| canonicalization(e.to_string()))
}

fn unique_clauses(binding: &ClauseBindingReport) -> Result<(), DischargeStoreError> {
    let mut seen = BTreeSet::new();
    for clause in &binding.clauses {
        if !seen.insert(clause.clause_id.as_str()) {
            return Err(DischargeError(format!(
                "duplicate clause {} in the binding report",
                clause.clause_id
            ))
            .into());
        }
    }
    Ok(())
}

/// The clause ids whose policy declares approval roles.
fn approval_clause_ids(policy: &SetPolicy) -> BTreeSet<String> {
    policy
        .iter()
        .filter(|(_, clause)| clause.approval_class())
        .map(|(id, _)| id.clone())
        .collect()
}

/// Persist discharge facts for the binding report's clause set.
///
/// Every fact is validated before the first file is written: the FR-046
/// predicates (malformed fact, duplicate clause, duplicate clause in the
/// binding report), the id bound, case collisions both inside the list and
/// against facts already stored, and the FR-211 checks of its evidence and
/// approval. A refused list writes nothing. A fact for a clause that does not
/// currently bind is stored and reported as unused when read, because
/// applicability can change with the declared context.
///
/// `checks.policy` is the policy of the exact installed clause set: it admits a
/// blank `approvalRef` only for approval-class clauses, so FR-211 can name it.
///
/// # Errors
///
/// [`DischargeStoreError::Discharge`], [`DischargeStoreError::Refused`],
/// [`DischargeStoreError::FactsFailed`] or [`DischargeStoreError::Check`] when
/// the list is refused; [`DischargeStoreError::Evidence`] when a file cannot be
/// read or written.
pub fn record_discharge_facts(
    source: &mut dyn EvidenceSource,
    request: &RecordDischargeRequest,
    checks: &DischargeChecks<'_>,
) -> Result<Vec<RecordedFact>, DischargeStoreError> {
    let required_approval_clauses = approval_clause_ids(checks.policy);
    unique_clauses(&request.binding)?;
    if request.facts.len() > MAX_STORED_FACTS {
        return refuse(
            StoreRefusal::TooManyFacts,
            format!("at most {MAX_STORED_FACTS} facts per clause set"),
        );
    }
    let set = &request.binding.clause_set;
    let mut seen_ids = BTreeSet::new();
    let mut seen_paths = BTreeMap::new();
    let mut staged = Vec::with_capacity(request.facts.len());
    let mut parsed = Vec::with_capacity(request.facts.len());
    for value in &request.facts {
        let fact = fact_for_approval_policy(value, &required_approval_clauses)?;
        if !seen_ids.insert(fact.clause_id().to_owned()) {
            return Err(DischargeError(format!(
                "duplicate discharge fact for clause {}",
                fact.clause_id()
            ))
            .into());
        }
        let path = fact_path(set, fact.clause_id())?;
        if let Some(other) = seen_paths.insert(path.clone(), fact.clause_id().to_owned()) {
            return refuse(
                StoreRefusal::CaseCollision,
                format!(
                    "clause {:?} and {other:?} fold to one file name",
                    fact.clause_id()
                ),
            );
        }
        if let Some(text) = source.read(&path)? {
            let held: StoredDischargeFactValue =
                serde_json::from_str(&text).map_err(|e| EvidenceError::StoreRead {
                    path: path.clone(),
                    detail: e.to_string(),
                })?;
            let held_fact = fact_for_approval_policy(&held.fact, &required_approval_clauses)
                .map_err(|error| EvidenceError::StoreRead {
                    path: path.clone(),
                    detail: error.to_string(),
                })?;
            if &held.clause_set != set || held_fact.clause_id() != fact.clause_id() {
                return refuse(
                    StoreRefusal::CaseCollision,
                    format!(
                        "{path} is held by clause {:?} of another spelling",
                        held_fact.clause_id()
                    ),
                );
            }
        }
        let record = StoredDischargeFact {
            schema_version: StoredFactSchemaVersion::V1,
            clause_set: set.clone(),
            fact,
        };
        let bytes = canonical(&record)?;
        staged.push((path, record.fact.clause_id().to_owned(), bytes));
        parsed.push(record.fact);
    }
    let failures = check_facts(set, &parsed, Some(&*source), checks)?;
    if !failures.is_empty() {
        return Err(DischargeStoreError::FactsFailed(failures));
    }
    let mut recorded = Vec::with_capacity(staged.len());
    for (path, clause_id, bytes) in staged {
        source.write(&path, &bytes)?;
        recorded.push(RecordedFact { clause_id, path });
    }
    Ok(recorded)
}

fn fact_for_approval_policy(
    value: &serde_json::Value,
    required_approval_clauses: &BTreeSet<String>,
) -> Result<DischargeFact, DischargeStoreError> {
    Ok(parse_fact_with_approval_requirements(
        value,
        required_approval_clauses,
    )?)
}

/// Every stored fact for one clause set, ordered by clause id.
///
/// A set nobody recorded for reads as empty. A file that is not a valid record
/// of this set, or sits at the path of another clause, is an error naming the
/// file: skipping it would report its clause as open and hide that a fact was
/// lost. More than [`MAX_STORED_FACTS`] files, or a file over
/// [`MAX_STORED_FACT_BYTES`], is refused rather than read. `policy` admits a
/// blank `approvalRef` on approval-class clauses only.
///
/// # Errors
///
/// [`EvidenceError::StoreRead`] for an unreadable, oversized or inconsistent
/// file or too many files, or [`EvidenceError::StoreIo`] when the directory
/// cannot be listed.
pub fn read_discharge_facts<S: EvidenceSource + ?Sized>(
    source: &S,
    set: &ClauseSetKey,
    policy: &SetPolicy,
) -> Result<Vec<DischargeFact>, DischargeStoreError> {
    let required_approval_clauses = approval_clause_ids(policy);
    let directory = clause_set_dir(set)?;
    let names = source.list_files(&directory)?;
    if names.len() > MAX_STORED_FACTS {
        return Err(EvidenceError::StoreRead {
            path: directory,
            detail: format!("more than {MAX_STORED_FACTS} stored facts"),
        }
        .into());
    }
    let mut facts = Vec::new();
    for name in names {
        let path = format!("{directory}/{name}");
        let Some(text) = source.read(&path)? else {
            continue;
        };
        let unreadable = |detail: String| EvidenceError::StoreRead {
            path: path.clone(),
            detail,
        };
        if text.len() > MAX_STORED_FACT_BYTES {
            return Err(unreadable(format!("over {MAX_STORED_FACT_BYTES} bytes")).into());
        }
        let record: StoredDischargeFactValue =
            serde_json::from_str(&text).map_err(|e| unreadable(e.to_string()))?;
        let fact = fact_for_approval_policy(&record.fact, &required_approval_clauses)
            .map_err(|error| unreadable(error.to_string()))?;
        if &record.clause_set != set || fact_path(set, fact.clause_id())? != path {
            return Err(unreadable(
                "the record names another clause set or clause than its path".to_owned(),
            )
            .into());
        }
        facts.push(fact);
    }
    facts.sort_by(|a, b| a.clause_id().cmp(b.clause_id()));
    Ok(facts)
}

/// Partition the request's facts and apply the FR-211 checks against `store`.
///
/// `store` is `None` when no repository was supplied: every direct fact and
/// every approval-class clause is then open with `no-store`.
///
/// # Errors
///
/// [`DischargeStoreError::Discharge`] when `asOf`, a fact or the binding report
/// is refused; [`DischargeStoreError::Check`] when the verifier refuses its
/// trust file.
pub fn build_checked_discharge_report(
    request: &BuildDischargeRequest,
    store: Option<&dyn EvidenceSource>,
    checks: &DischargeChecks<'_>,
) -> Result<CheckedReport, DischargeStoreError> {
    let partition = build_discharge_report_with_approval_requirements(
        request,
        &approval_clause_ids(checks.policy),
    )?;
    Ok(check_report(
        partition,
        store,
        checks,
        Some(&request.binding.clauses),
    )?)
}

/// The discharge report for a live binding report, with the stored facts for
/// its clause set as the facts and the store as the evidence the facts cite.
///
/// This is the one read a view takes: the binding report is the live
/// obligation set, the store supplies the facts and the records they cite. A
/// fact for a clause that does not bind comes back only as an unused
/// `{clauseId, kind, reason}`; a caller that needs such a fact's content reads
/// it with [`read_discharge_facts`].
///
/// # Errors
///
/// [`DischargeStoreError::Evidence`] when the store cannot be read;
/// [`DischargeStoreError::Discharge`] when `as_of` or the binding report is
/// refused; [`DischargeStoreError::Check`] when the verifier refuses its trust
/// file.
pub fn discharge_report_from_store(
    source: &dyn EvidenceSource,
    binding: &ClauseBindingReport,
    as_of: &str,
    checks: &DischargeChecks<'_>,
) -> Result<DischargeReport, DischargeStoreError> {
    let facts = read_discharge_facts(source, &binding.clause_set, checks.policy)?
        .iter()
        .map(|fact| {
            serde_json::to_value(fact)
                .map_err(|e| DischargeError(format!("stored fact cannot be re-read: {e}")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(build_checked_discharge_report(
        &BuildDischargeRequest {
            binding: binding.clone(),
            facts,
            as_of: as_of.to_owned(),
        },
        Some(source),
        checks,
    )?
    .report)
}

/// Inward completeness for a live binding report and the stored facts.
///
/// Built from [`discharge_report_from_store`], so the populations add up by
/// construction and the obligation set is always the live one.
///
/// # Errors
///
/// As [`discharge_report_from_store`].
pub fn inward_completeness_from_store(
    source: &dyn EvidenceSource,
    binding: &ClauseBindingReport,
    as_of: &str,
    checks: &DischargeChecks<'_>,
) -> Result<InwardCompleteness, DischargeStoreError> {
    Ok(build_inward_completeness(&discharge_report_from_store(
        source, binding, as_of, checks,
    )?))
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use std::collections::BTreeSet;

    use ix_trace_rs::trace;
    use quoin_evidence::source::{EvidenceSource, MemoryEvidence};

    use super::super::fixtures::{binding, direct};
    use super::super::report::{DischargeFact, DischargeState};
    use super::{
        DischargeStoreError, MAX_STORED_FACT_BYTES, MAX_STORED_FACTS, RecordDischargeRequest,
        StoreRefusal, discharge_report_from_store,
        discharge_report_from_store_with_approval_requirements, inward_completeness_from_store,
        read_discharge_facts, record_discharge_facts,
        record_discharge_facts_with_approval_requirements,
    };

    const AS_OF: &str = "2026-08-15T00:00:00Z";

    fn record(facts: Vec<serde_json::Value>) -> RecordDischargeRequest {
        RecordDischargeRequest {
            binding: binding(),
            facts,
        }
    }

    fn fact_for(clause_id: &str) -> serde_json::Value {
        let mut fact = direct();
        fact["clauseId"] = clause_id.into();
        fact
    }

    fn files(store: &MemoryEvidence) -> usize {
        store
            .list_directories("discharge")
            .map_or(0, |dirs| dirs.len())
    }

    fn refusal(error: &DischargeStoreError) -> Option<StoreRefusal> {
        match error {
            DischargeStoreError::Refused { kind, .. } => Some(*kind),
            _ => None,
        }
    }

    #[trace("FR-133-AC-1")]
    #[test]
    fn a_recorded_fact_stores_the_fact_and_clause_set_and_nothing_else() {
        let mut store = MemoryEvidence::new();
        let written = record_discharge_facts(&mut store, &record(vec![direct()])).unwrap();
        assert_eq!(written.len(), 1);
        let text = store.read(&written[0].path).unwrap().unwrap();
        let stored: serde_json::Value = serde_json::from_str(&text).unwrap();
        let mut keys: Vec<&str> = stored
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, ["clauseSet", "fact", "schemaVersion"]);
        assert_eq!(stored["schemaVersion"], "discharge-fact-record-v1");
        assert_eq!(stored["fact"]["clauseId"], "SYN-001");
        // The clause's force and expected outputs are the binding report's.
        assert!(!text.contains("expectedOutputs") && !text.contains("mandatory"));
        assert!(!text.contains("SYN-005"), "no other clause is stored");
    }

    #[trace("FR-046-AC-9", "TC-626")]
    #[test]
    fn direct_approval_ref_is_validated_retained_and_stays_in_the_direct_population() {
        for approval_ref in ["", " \t\n"] {
            let mut fact = direct();
            fact["approvalRef"] = approval_ref.into();
            let mut store = MemoryEvidence::new();
            let error = record_discharge_facts(&mut store, &record(vec![fact]))
                .expect_err("empty approval references are refused");
            assert!(error.to_string().contains("approvalRef must not be empty"));
            assert_eq!(files(&store), 0, "a refused fact is not written");
        }

        let approval_ref = "  approval://synthetic/direct-1  ";
        let mut fact = direct();
        fact["approvalRef"] = approval_ref.into();
        let mut store = MemoryEvidence::new();
        let written = record_discharge_facts(&mut store, &record(vec![fact])).unwrap();
        let stored_text = store.read(&written[0].path).unwrap().unwrap();
        let stored: serde_json::Value = serde_json::from_str(&stored_text).unwrap();
        assert_eq!(stored["fact"]["approvalRef"], approval_ref);

        let facts = read_discharge_facts(&store, &binding().clause_set).unwrap();
        assert_eq!(facts.len(), 1);
        assert!(matches!(
            &facts[0],
            DischargeFact::Direct(direct) if direct.approval_ref.as_deref() == Some(approval_ref)
        ));
        let report = discharge_report_from_store(&store, &binding(), AS_OF).unwrap();
        assert_eq!(report.binding.direct.len(), 1);
        assert!(report.binding.dispositions.is_empty());
        assert_eq!(report.binding.direct[0].state, DischargeState::Direct);
        assert!(matches!(
            report.binding.direct[0].fact.as_ref(),
            Some(DischargeFact::Direct(direct))
                if direct.approval_ref.as_deref() == Some(approval_ref)
        ));

        let mut omitted_store = MemoryEvidence::new();
        let omitted = record_discharge_facts(&mut omitted_store, &record(vec![direct()])).unwrap();
        let omitted_text = omitted_store.read(&omitted[0].path).unwrap().unwrap();
        let omitted_record: serde_json::Value = serde_json::from_str(&omitted_text).unwrap();
        assert!(omitted_record["fact"].get("approvalRef").is_none());
        let omitted_facts = read_discharge_facts(&omitted_store, &binding().clause_set).unwrap();
        assert!(matches!(
            &omitted_facts[0],
            DischargeFact::Direct(direct) if direct.approval_ref.is_none()
        ));
        let omitted_report =
            discharge_report_from_store(&omitted_store, &binding(), AS_OF).unwrap();
        assert_eq!(omitted_report.binding.direct.len(), 1);
        assert!(omitted_report.binding.dispositions.is_empty());
        assert!(matches!(
            omitted_report.binding.direct[0].fact.as_ref(),
            Some(DischargeFact::Direct(direct)) if direct.approval_ref.is_none()
        ));
    }

    #[trace("FR-208-AC-1", "FR-208-AC-3")]
    #[test]
    fn policy_aware_store_keeps_required_blank_approval_open_and_generic_read_rejects_it() {
        let mut store = MemoryEvidence::new();
        let binding = binding();
        let required = BTreeSet::from(["SYN-001".to_owned()]);
        let mut blank = direct();
        blank["approvalRef"] = " \t\n".into();

        record_discharge_facts_with_approval_requirements(
            &mut store,
            &record(vec![blank]),
            &required,
        )
        .unwrap();
        let report = discharge_report_from_store_with_approval_requirements(
            &store, &binding, AS_OF, &required,
        )
        .unwrap();
        assert!(report.binding.direct.is_empty());
        assert_eq!(report.binding.open[0].clause_id, "SYN-001");
        assert_eq!(
            report.binding.open[0].reason.as_deref(),
            Some("required approval reference is missing")
        );
        assert!(read_discharge_facts(&store, &binding.clause_set).is_err());
    }

    #[trace("FR-133-AC-2")]
    #[test]
    fn the_report_partitions_the_live_binding_with_the_stored_facts() {
        let mut store = MemoryEvidence::new();
        let before = discharge_report_from_store(&store, &binding(), AS_OF).unwrap();
        assert_eq!(before.binding.open.len(), 1);
        record_discharge_facts(&mut store, &record(vec![direct()])).unwrap();
        let after = discharge_report_from_store(&store, &binding(), AS_OF).unwrap();
        assert_eq!(after.binding.direct.len(), 1);
        assert!(after.binding.open.is_empty());

        // Currency is decided at read time from the stored attestation.
        let expired =
            discharge_report_from_store(&store, &binding(), "2026-10-01T00:00:00Z").unwrap();
        assert_eq!(expired.binding.open.len(), 1);
        assert!(expired.binding.direct.is_empty());
    }

    #[trace("FR-133-AC-3")]
    #[test]
    fn recording_again_replaces_the_fact() {
        let mut store = MemoryEvidence::new();
        record_discharge_facts(&mut store, &record(vec![direct()])).unwrap();
        let mut renewed = direct();
        renewed["attestation"]["expiresAt"] = "2027-01-01T00:00:00.000Z".into();
        record_discharge_facts(&mut store, &record(vec![renewed])).unwrap();
        let facts = read_discharge_facts(&store, &binding().clause_set).unwrap();
        assert_eq!(facts.len(), 1);
        assert_eq!(
            facts[0].attestation().expires_at,
            "2027-01-01T00:00:00.000Z"
        );
    }

    #[trace("FR-133-AC-3")]
    #[test]
    fn a_refused_list_writes_nothing_whatever_the_refusal() {
        let long = "x".repeat(120);
        let mut bad_instant = direct();
        bad_instant["attestation"]["expiresAt"] = "not an instant".into();
        let mut unknown_field = direct();
        unknown_field["inventedScore"] = 1.into();
        let lists = vec![
            vec![direct(), direct()],
            vec![fact_for("SYN-001"), fact_for(&long)],
            vec![fact_for("C-1"), fact_for("c-1")],
            vec![direct(), bad_instant],
            vec![direct(), unknown_field],
        ];
        for facts in lists {
            let mut store = MemoryEvidence::new();
            assert!(record_discharge_facts(&mut store, &record(facts)).is_err());
            assert_eq!(files(&store), 0, "a refused list must write nothing");
        }
    }

    #[trace("FR-133-AC-3")]
    #[test]
    fn ids_are_bounded_and_case_collisions_are_typed_refusals() {
        let mut store = MemoryEvidence::new();
        let long = "x".repeat(65);
        let error = record_discharge_facts(&mut store, &record(vec![fact_for(&long)])).unwrap_err();
        assert_eq!(refusal(&error), Some(StoreRefusal::InvalidIdentifier));
        let exact = "x".repeat(64);
        record_discharge_facts(&mut store, &record(vec![fact_for(&exact)])).unwrap();

        let error =
            record_discharge_facts(&mut store, &record(vec![fact_for("C-1"), fact_for("c-1")]))
                .unwrap_err();
        assert_eq!(refusal(&error), Some(StoreRefusal::CaseCollision));

        record_discharge_facts(&mut store, &record(vec![fact_for("C-1")])).unwrap();
        let error = record_discharge_facts(&mut store, &record(vec![fact_for("c-1")])).unwrap_err();
        assert_eq!(refusal(&error), Some(StoreRefusal::CaseCollision));
        let facts = read_discharge_facts(&store, &binding().clause_set).unwrap();
        assert!(facts.iter().any(|f| f.clause_id() == "C-1"));
        assert!(facts.iter().all(|f| f.clause_id() != "c-1"));
    }

    #[trace("FR-133-AC-3")]
    #[test]
    fn a_binding_report_naming_a_clause_twice_is_refused() {
        let mut value = serde_json::to_value(binding()).unwrap();
        let first = value["clauses"][0].clone();
        value["clauses"].as_array_mut().unwrap().push(first);
        let doubled: quoin_quire_types::ClauseBindingReport =
            serde_json::from_value(value).unwrap();
        let mut store = MemoryEvidence::new();
        let request = RecordDischargeRequest {
            binding: doubled.clone(),
            facts: vec![direct()],
        };
        let error = record_discharge_facts(&mut store, &request).unwrap_err();
        assert!(
            error.to_string().contains("duplicate clause SYN-001"),
            "{error}"
        );
        let error = discharge_report_from_store(&store, &doubled, AS_OF).unwrap_err();
        assert!(
            error.to_string().contains("duplicate clause SYN-001"),
            "{error}"
        );
        let error = inward_completeness_from_store(&store, &doubled, AS_OF).unwrap_err();
        assert!(
            error.to_string().contains("duplicate clause SYN-001"),
            "{error}"
        );
    }

    #[trace("FR-133-AC-4")]
    #[test]
    fn a_misplaced_or_corrupt_stored_file_is_an_error_not_an_open_clause() {
        let mut store = MemoryEvidence::new();
        let written = record_discharge_facts(&mut store, &record(vec![direct()])).unwrap();
        let text = store.read(&written[0].path).unwrap().unwrap();
        let path = &written[0].path;

        let moved = path.replace("syn-001", "syn-002");
        store.write(&moved, text.as_bytes()).unwrap();
        let error = read_discharge_facts(&store, &binding().clause_set).unwrap_err();
        assert!(error.to_string().contains("syn-002"), "{error}");

        let mut other_set = MemoryEvidence::new();
        let wrong_set = text.replace("synthetic-widget-rules", "another-set");
        other_set.write(path, wrong_set.as_bytes()).unwrap();
        let error = read_discharge_facts(&other_set, &binding().clause_set).unwrap_err();
        assert!(error.to_string().contains("another clause set"), "{error}");

        let mut corrupt = MemoryEvidence::new();
        corrupt.write(path, b"{not json").unwrap();
        assert!(read_discharge_facts(&corrupt, &binding().clause_set).is_err());
    }

    #[trace("FR-133-AC-4")]
    #[test]
    fn the_stored_fact_count_and_file_size_are_bounded() {
        let mut store = MemoryEvidence::new();
        let written = record_discharge_facts(&mut store, &record(vec![direct()])).unwrap();
        let dir = written[0].path.rsplit_once('/').unwrap().0.to_owned();
        let huge = vec![b' '; MAX_STORED_FACT_BYTES + 1];
        store.write(&written[0].path, &huge).unwrap();
        let error = read_discharge_facts(&store, &binding().clause_set).unwrap_err();
        assert!(error.to_string().contains("bytes"), "{error}");

        let mut crowded = MemoryEvidence::new();
        for n in 0..=MAX_STORED_FACTS {
            crowded.write(&format!("{dir}/f{n}.json"), b"{}").unwrap();
        }
        let error = read_discharge_facts(&crowded, &binding().clause_set).unwrap_err();
        assert!(error.to_string().contains("more than"), "{error}");

        let many = vec![direct(); MAX_STORED_FACTS + 1];
        let mut store = MemoryEvidence::new();
        let error = record_discharge_facts(&mut store, &record(many)).unwrap_err();
        assert_eq!(refusal(&error), Some(StoreRefusal::TooManyFacts));
    }
}
