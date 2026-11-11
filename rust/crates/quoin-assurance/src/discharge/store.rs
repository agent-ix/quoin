// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Discharge facts in the evidence store (FR-133).
//!
//! # Facts are persisted, obligations are not
//!
//! A discharge fact is a statement a person made at a point in time: this
//! clause is discharged by that evidence, or disposed of by this decision, and
//! the attestation carries an expiry. Nothing can re-derive it, so it is
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

use quoin_evidence::EvidenceError;
use quoin_evidence::paths::record_stem;
use quoin_evidence::source::EvidenceSource;
use quoin_quire_types::{ClauseBindingReport, ClauseSetKey};
use serde::{Deserialize, Serialize};

use super::build::build_discharge_report;
use super::report::{BuildDischargeRequest, DischargeError, DischargeFact, DischargeReport};

/// Where discharge facts live, under the store root.
pub const DISCHARGE_DIR: &str = "discharge";

/// The tool that wrote a record, as FR-030-AC-17 states an identity: a name and
/// an exact version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct RecordingTool {
    /// Always `quoin`.
    pub name: String,
    /// The exact version of the build that wrote the record.
    pub version: String,
}

/// The one accepted `schemaVersion` of a stored discharge fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum StoredFactSchemaVersion {
    /// `discharge-fact-record-v1`.
    #[serde(rename = "discharge-fact-record-v1")]
    V1,
}

/// One discharge fact as the store holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct StoredDischargeFact {
    /// Always `discharge-fact-record-v1`.
    pub schema_version: StoredFactSchemaVersion,
    /// The clause set the fact was made against.
    pub clause_set: ClauseSetKey,
    /// The caller's instant for the recording. Never the wall clock.
    pub recorded_at: String,
    /// What wrote the record.
    pub tool: RecordingTool,
    /// The fact itself, validated on every read by the same predicates a
    /// request goes through.
    pub fact: DischargeFact,
}

/// What `record_discharge_facts` is given.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RecordDischargeRequest {
    /// A validated `clause-binding-v1` report. Only its clause set key is
    /// stored; the fact list is checked against it.
    pub binding: ClauseBindingReport,
    /// Unvalidated discharge facts, as `quoin discharge --facts` takes them.
    pub facts: Vec<serde_json::Value>,
    /// The caller's instant for this recording.
    pub recorded_at: String,
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

/// Why a discharge store operation failed.
#[derive(Debug)]
pub enum DischargeStoreError {
    /// The facts or the instant were refused by the FR-046 predicates.
    Discharge(DischargeError),
    /// The store could not be read or written.
    Evidence(EvidenceError),
}

impl std::fmt::Display for DischargeStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Discharge(error) => error.fmt(f),
            Self::Evidence(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for DischargeStoreError {}

impl From<DischargeError> for DischargeStoreError {
    fn from(error: DischargeError) -> Self {
        Self::Discharge(error)
    }
}

impl From<EvidenceError> for DischargeStoreError {
    fn from(error: EvidenceError) -> Self {
        Self::Evidence(error)
    }
}

fn clause_set_dir(set: &ClauseSetKey) -> String {
    format!(
        "{DISCHARGE_DIR}/{}/{}/{}",
        record_stem(&set.authority),
        record_stem(&set.id),
        record_stem(&set.version)
    )
}

fn fact_path(set: &ClauseSetKey, clause_id: &str) -> String {
    format!("{}/{}.json", clause_set_dir(set), record_stem(clause_id))
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

/// Persist discharge facts for the binding report's clause set.
///
/// The whole list is validated first, as `build_discharge_report` validates it
/// (malformed fact, duplicate clause, bad instant), so a refused list writes
/// nothing. A fact whose clause the binding report does not hold as binding is
/// still stored: applicability can change with the declared context, and the
/// report names such a fact as unused rather than losing it.
///
/// # Errors
///
/// [`DischargeStoreError::Discharge`] when the facts or `recordedAt` are
/// refused; [`DischargeStoreError::Evidence`] when a file cannot be written.
pub fn record_discharge_facts<S: EvidenceSource + ?Sized>(
    source: &mut S,
    request: &RecordDischargeRequest,
) -> Result<Vec<RecordedFact>, DischargeStoreError> {
    build_discharge_report(&BuildDischargeRequest {
        binding: request.binding.clone(),
        facts: request.facts.clone(),
        as_of: request.recorded_at.clone(),
    })?;
    let set = &request.binding.clause_set;
    let mut writes = Vec::with_capacity(request.facts.len());
    for value in &request.facts {
        let fact = DischargeFact::try_from(value.clone())?;
        let path = fact_path(set, fact.clause_id());
        let record = StoredDischargeFact {
            schema_version: StoredFactSchemaVersion::V1,
            clause_set: set.clone(),
            recorded_at: request.recorded_at.clone(),
            tool: RecordingTool {
                name: "quoin".to_owned(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
            },
            fact,
        };
        writes.push((path, record));
    }
    // Every record is encoded before the first write, so an encoding failure
    // cannot leave half a list on disk.
    let encoded = writes
        .iter()
        .map(|(_, record)| canonical(record))
        .collect::<Result<Vec<_>, _>>()?;
    let mut recorded = Vec::with_capacity(writes.len());
    for ((path, record), bytes) in writes.into_iter().zip(encoded) {
        source.write(&path, &bytes)?;
        recorded.push(RecordedFact {
            clause_id: record.fact.clause_id().to_owned(),
            path,
        });
    }
    Ok(recorded)
}

/// Every stored fact for one clause set, ordered by clause id.
///
/// A set nobody recorded for reads as empty. A file that is not a valid record
/// of this set, or sits at the path of another clause, is an error naming the
/// file: skipping it would report its clause as open and hide that a fact was
/// lost.
///
/// # Errors
///
/// [`EvidenceError::StoreRead`] for an unreadable or inconsistent file, or
/// [`EvidenceError::StoreIo`] when the directory cannot be listed.
pub fn read_discharge_facts<S: EvidenceSource + ?Sized>(
    source: &S,
    set: &ClauseSetKey,
) -> Result<Vec<DischargeFact>, EvidenceError> {
    let directory = clause_set_dir(set);
    let mut facts = Vec::new();
    for name in source.list_files(&directory)? {
        let path = format!("{directory}/{name}");
        let Some(text) = source.read(&path)? else {
            continue;
        };
        let unreadable = |detail: String| EvidenceError::StoreRead {
            path: path.clone(),
            detail,
        };
        let record: StoredDischargeFact =
            serde_json::from_str(&text).map_err(|e| unreadable(e.to_string()))?;
        if &record.clause_set != set || fact_path(set, record.fact.clause_id()) != path {
            return Err(unreadable(
                "the record names another clause set or clause than its path".to_owned(),
            ));
        }
        facts.push(record.fact);
    }
    facts.sort_by(|a, b| a.clause_id().cmp(b.clause_id()));
    Ok(facts)
}

/// The discharge report for a live binding report, with the stored facts for
/// its clause set as the facts.
///
/// This is the read a view takes: the binding report is the live obligation
/// set, the store supplies only facts.
///
/// # Errors
///
/// [`DischargeStoreError::Evidence`] when the store cannot be read;
/// [`DischargeStoreError::Discharge`] when `as_of` is refused.
pub fn discharge_report_from_store<S: EvidenceSource + ?Sized>(
    source: &S,
    binding: &ClauseBindingReport,
    as_of: &str,
) -> Result<DischargeReport, DischargeStoreError> {
    let facts = read_discharge_facts(source, &binding.clause_set)?
        .iter()
        .map(|fact| {
            serde_json::to_value(fact)
                .map_err(|e| DischargeError(format!("stored fact cannot be re-read: {e}")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(build_discharge_report(&BuildDischargeRequest {
        binding: binding.clone(),
        facts,
        as_of: as_of.to_owned(),
    })?)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use ix_trace_rs::trace;
    use quoin_evidence::source::{EvidenceSource, MemoryEvidence};

    use super::super::fixtures::{binding, direct};
    use super::{
        RecordDischargeRequest, discharge_report_from_store, read_discharge_facts,
        record_discharge_facts,
    };

    fn record(facts: Vec<serde_json::Value>) -> RecordDischargeRequest {
        RecordDischargeRequest {
            binding: binding(),
            facts,
            recorded_at: "2026-08-02T00:00:00Z".to_owned(),
        }
    }

    #[trace("FR-133-AC-1", "TC-2600")]
    #[test]
    fn a_recorded_fact_stores_the_fact_and_no_obligation() {
        let mut store = MemoryEvidence::new();
        let written = record_discharge_facts(&mut store, &record(vec![direct()])).unwrap();
        assert_eq!(written.len(), 1);
        let text = store.read(&written[0].path).unwrap().unwrap();
        let stored: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(stored["schemaVersion"], "discharge-fact-record-v1");
        assert_eq!(stored["tool"]["name"], "quoin");
        assert_eq!(stored["fact"]["clauseId"], "SYN-001");
        // The clause's force and expected outputs are the binding report's.
        assert!(!text.contains("expectedOutputs") && !text.contains("mandatory"));
        assert!(!text.contains("SYN-005"), "no other clause is stored");
    }

    #[trace("FR-133-AC-2", "TC-2601")]
    #[test]
    fn the_report_partitions_the_live_binding_with_the_stored_facts() {
        let mut store = MemoryEvidence::new();
        let before =
            discharge_report_from_store(&store, &binding(), "2026-08-15T00:00:00Z").unwrap();
        assert_eq!(before.binding.open.len(), 1);
        record_discharge_facts(&mut store, &record(vec![direct()])).unwrap();
        let after =
            discharge_report_from_store(&store, &binding(), "2026-08-15T00:00:00Z").unwrap();
        assert_eq!(after.binding.direct.len(), 1);
        assert!(after.binding.open.is_empty());

        // The same stored fact, read after its expiry, leaves the clause open:
        // currency is decided at read time from the stored attestation.
        let expired =
            discharge_report_from_store(&store, &binding(), "2026-10-01T00:00:00Z").unwrap();
        assert_eq!(expired.binding.open.len(), 1);
        assert!(expired.binding.direct.is_empty());
    }

    #[trace("FR-133-AC-3", "TC-2602")]
    #[test]
    fn recording_again_replaces_the_fact_and_a_refused_list_writes_nothing() {
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

        let mut other = MemoryEvidence::new();
        let refused = record_discharge_facts(&mut other, &record(vec![direct(), direct()]));
        assert!(refused.is_err());
        assert!(
            read_discharge_facts(&other, &binding().clause_set)
                .unwrap()
                .is_empty()
        );
    }

    #[trace("FR-133-AC-4", "TC-2603")]
    #[test]
    fn a_stored_file_at_the_wrong_path_is_an_error_not_an_open_clause() {
        let mut store = MemoryEvidence::new();
        let written = record_discharge_facts(&mut store, &record(vec![direct()])).unwrap();
        let text = store.read(&written[0].path).unwrap().unwrap();
        let moved = written[0].path.replace("SYN-001", "SYN-002");
        store.write(&moved, text.as_bytes()).unwrap();
        let error = read_discharge_facts(&store, &binding().clause_set).unwrap_err();
        assert!(error.to_string().contains("SYN-002"), "{error}");

        let mut corrupt = MemoryEvidence::new();
        corrupt.write(&written[0].path, b"{not json").unwrap();
        assert!(read_discharge_facts(&corrupt, &binding().clause_set).is_err());
    }
}
