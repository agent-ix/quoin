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

use std::collections::{BTreeMap, BTreeSet};

use quoin_evidence::EvidenceError;
use quoin_evidence::paths::record_stem;
use quoin_evidence::source::EvidenceSource;
use quoin_quire_types::{ClauseBindingReport, ClauseSetKey};
use serde::{Deserialize, Serialize};

use super::build::build_discharge_report;
use super::completeness::{InwardCompleteness, build_inward_completeness};
use super::report::{BuildDischargeRequest, DischargeError, DischargeFact, DischargeReport};

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
}

impl std::fmt::Display for DischargeStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Discharge(error) => error.fmt(f),
            Self::Refused { kind, detail } => write!(f, "{} ({detail})", kind.as_str()),
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

/// Persist discharge facts for the binding report's clause set.
///
/// Every fact is validated before the first file is written: the FR-046
/// predicates (malformed fact, duplicate clause, duplicate clause in the
/// binding report), the id bound, and case collisions both inside the list and
/// against facts already stored. A refused list writes nothing. A fact for a
/// clause that does not currently bind is stored and reported as unused when
/// read, because applicability can change with the declared context.
///
/// # Errors
///
/// [`DischargeStoreError::Discharge`] or [`DischargeStoreError::Refused`] when
/// the list is refused; [`DischargeStoreError::Evidence`] when a file cannot be
/// read or written.
pub fn record_discharge_facts<S: EvidenceSource + ?Sized>(
    source: &mut S,
    request: &RecordDischargeRequest,
) -> Result<Vec<RecordedFact>, DischargeStoreError> {
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
    for value in &request.facts {
        let fact = DischargeFact::try_from(value.clone())?;
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
            let held: StoredDischargeFact =
                serde_json::from_str(&text).map_err(|e| EvidenceError::StoreRead {
                    path: path.clone(),
                    detail: e.to_string(),
                })?;
            if &held.clause_set != set || held.fact.clause_id() != fact.clause_id() {
                return refuse(
                    StoreRefusal::CaseCollision,
                    format!(
                        "{path} is held by clause {:?} of another spelling",
                        held.fact.clause_id()
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
    }
    let mut recorded = Vec::with_capacity(staged.len());
    for (path, clause_id, bytes) in staged {
        source.write(&path, &bytes)?;
        recorded.push(RecordedFact { clause_id, path });
    }
    Ok(recorded)
}

/// Every stored fact for one clause set, ordered by clause id.
///
/// A set nobody recorded for reads as empty. A file that is not a valid record
/// of this set, or sits at the path of another clause, is an error naming the
/// file: skipping it would report its clause as open and hide that a fact was
/// lost. More than [`MAX_STORED_FACTS`] files, or a file over
/// [`MAX_STORED_FACT_BYTES`], is refused rather than read.
///
/// # Errors
///
/// [`EvidenceError::StoreRead`] for an unreadable, oversized or inconsistent
/// file or too many files, or [`EvidenceError::StoreIo`] when the directory
/// cannot be listed.
pub fn read_discharge_facts<S: EvidenceSource + ?Sized>(
    source: &S,
    set: &ClauseSetKey,
) -> Result<Vec<DischargeFact>, DischargeStoreError> {
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
        let record: StoredDischargeFact =
            serde_json::from_str(&text).map_err(|e| unreadable(e.to_string()))?;
        if &record.clause_set != set || fact_path(set, record.fact.clause_id())? != path {
            return Err(unreadable(
                "the record names another clause set or clause than its path".to_owned(),
            )
            .into());
        }
        facts.push(record.fact);
    }
    facts.sort_by(|a, b| a.clause_id().cmp(b.clause_id()));
    Ok(facts)
}

/// The discharge report for a live binding report, with the stored facts for
/// its clause set as the facts.
///
/// This is the one read a view takes: the binding report is the live
/// obligation set, the store supplies only facts. A fact for a clause that does
/// not bind comes back only as an unused `{clauseId, kind, reason}`; a caller
/// that needs such a fact's content reads it with [`read_discharge_facts`].
///
/// # Errors
///
/// [`DischargeStoreError::Evidence`] when the store cannot be read;
/// [`DischargeStoreError::Discharge`] when `as_of` or the binding report is
/// refused.
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

/// Inward completeness for a live binding report and the stored facts.
///
/// Built from [`discharge_report_from_store`], so the populations add up by
/// construction and the obligation set is always the live one.
///
/// # Errors
///
/// As [`discharge_report_from_store`].
pub fn inward_completeness_from_store<S: EvidenceSource + ?Sized>(
    source: &S,
    binding: &ClauseBindingReport,
    as_of: &str,
) -> Result<InwardCompleteness, DischargeStoreError> {
    Ok(build_inward_completeness(&discharge_report_from_store(
        source, binding, as_of,
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
    use ix_trace_rs::trace;
    use quoin_evidence::source::{EvidenceSource, MemoryEvidence};

    use super::super::fixtures::{binding, direct};
    use super::{
        DischargeStoreError, MAX_STORED_FACT_BYTES, MAX_STORED_FACTS, RecordDischargeRequest,
        StoreRefusal, discharge_report_from_store, inward_completeness_from_store,
        read_discharge_facts, record_discharge_facts,
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
