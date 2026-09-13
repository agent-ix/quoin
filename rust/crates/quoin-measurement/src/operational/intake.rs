// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Admitting an operational record, or a linked pair, into the store.
//!
//! Ports `operational.ts:70-125` (`writeOperationalRecord`,
//! `writeOperationalPair`) and `operational.ts:232-294` (`validateForIntake`,
//! `writeOne`, `writeAtomic`, `linked`).
//!
//! # Everything here happens under the lock
//!
//! Read-back, admission and write are one critical section, because the
//! admission checks are *about the store*: "no other record already holds this
//! identity" is not a fact about the candidate, and a second writer between the
//! read and the write makes the answer stale. [`OperationalWriteLock`] is taken
//! first and released when the guard drops, including on a refusal.
//!
//! # Why the clock is a parameter
//!
//! The lock's ten-second deadline is measured on it. Production passes
//! [`crate::source::SystemClock`]; a test passes a clock that does not sleep,
//! which is the only way the busy refusal is assertable at all.
//!
//! # `linked` is an equality, not a canonicalisation
//!
//! `operational.ts:280-288` compares a capability's subject and scope to an
//! exercise's by canonicalising both and comparing the strings, because
//! TypeScript has no structural equality. Rust does, and
//! [`crate::common::producer::Subject`] and
//! [`crate::operational::record::OperationalScope`] derive it, so the
//! comparison is the comparison and there is no serialization in the middle of
//! it.

use std::path::{Path, PathBuf};

use quoin_store::store::{store_root, write_content_addressed};
use serde_json::{Map, Value};

use crate::intervention::intake::{InterventionIntakeError, InterventionRefusalCode};
use crate::json_bridge::from_serde;
use crate::operational::lock::OperationalWriteLock;
use crate::operational::paths::{operational_pair_path, operational_path};
use crate::operational::read::{read_operational_entries, read_operational_records};
use crate::operational::record::{
    OperationalEvidenceRecord, OperationalExerciseRecord, StandingCapabilityRecord,
};
use crate::operational::validate::{ValidOperationalRecord, validate_operational_record};
use crate::plans::{PlanLoadOptions, load_measurement_plans};
use crate::raw_evidence::{assert_governing_definition, verify_raw_evidence_references};
use crate::source::{Clock, DiskMeasurement};

/// The envelope version a pair file is written under. `operational.ts:108`.
const PAIR_SCHEMA_VERSION: u64 = 1;

/// Admit one record, returning the file it is retained in.
///
/// Re-publishing byte-identical content is idempotent and returns the retained
/// path, as `operational.ts:79-84` is.
///
/// # Errors
///
/// [`InterventionRefusalCode::InvalidRecord`] when the candidate is not a
/// record or its linked capability does not match,
/// [`InterventionRefusalCode::GoverningPlanAbsent`] and
/// [`InterventionRefusalCode::DefinitionMismatch`] from the governance check,
/// [`InterventionRefusalCode::RawEvidenceMismatch`] when a claimed file has
/// moved or changed, [`InterventionRefusalCode::RecordIdCollision`] when the
/// identity is taken by different bytes, and
/// [`InterventionRefusalCode::IntakeBusy`] when the store-wide lock is held.
pub fn write_operational_record(
    repo: &Path,
    clock: &dyn Clock,
    candidate: &Value,
) -> Result<PathBuf, InterventionIntakeError> {
    let candidate = validate_operational_record(candidate)?;
    let _lock = OperationalWriteLock::acquire(&store_root(repo), clock)?;
    let entries = read_operational_entries(repo)?;
    let retained: Vec<&ValidOperationalRecord> =
        entries.iter().map(|entry| &entry.record).collect();
    validate_for_intake(repo, &candidate, &retained)?;
    let bytes = candidate.canonical_bytes()?;
    for entry in &entries {
        if entry.record.record_id() == candidate.record_id()
            && entry.record.canonical_bytes()? == bytes
        {
            return Ok(entry.path.clone());
        }
    }
    let path = operational_path(repo, candidate.record_id());
    write_once(&path, &bytes)?;
    Ok(path)
}

/// Admit a linked capability and exercise as one file.
///
/// `operational.ts:87-125`. The two records are written together or not at all:
/// a store holding the exercise but not the capability it names is a store the
/// linkage check would refuse to add to, and a two-step write can produce one.
///
/// # Errors
///
/// As [`write_operational_record`], plus
/// [`InterventionRefusalCode::InvalidRecord`] when the two records are not one
/// standing capability followed by one exercise.
pub fn write_operational_pair(
    repo: &Path,
    clock: &dyn Clock,
    capability: &Value,
    exercise: &Value,
) -> Result<PathBuf, InterventionIntakeError> {
    let capability = validate_operational_record(capability)?;
    let exercise = validate_operational_record(exercise)?;
    if !matches!(
        (capability.record(), exercise.record()),
        (
            OperationalEvidenceRecord::StandingCapability(_),
            OperationalEvidenceRecord::Exercise(_)
        )
    ) {
        return Err(invalid(vec![
            "operational pair requires one standing capability followed by one exercise".to_owned(),
        ]));
    }
    let _lock = OperationalWriteLock::acquire(&store_root(repo), clock)?;
    let existing = read_operational_records(repo)?;
    let path = operational_pair_path(repo, capability.record_id(), exercise.record_id());
    let bytes = pair_bytes(&capability, &exercise)?;
    let retained: Vec<&ValidOperationalRecord> = existing.iter().collect();
    validate_for_intake(repo, &capability, &retained)?;
    let mut with_capability = retained.clone();
    with_capability.push(&capability);
    validate_for_intake(repo, &exercise, &with_capability)?;
    if path.exists() {
        let retained_bytes =
            std::fs::read(&path).map_err(|error| invalid(vec![error.to_string()]))?;
        if retained_bytes == bytes {
            return Ok(path);
        }
        return Err(collision(vec![path.display().to_string()]));
    }
    for record in [&capability, &exercise] {
        if existing
            .iter()
            .any(|item| item.record_id() == record.record_id())
        {
            return Err(collision(vec![record.record_id().to_string()]));
        }
    }
    write_once(&path, &bytes)?;
    Ok(path)
}

/// The pair envelope's canonical bytes. `operational.ts:107-110`.
fn pair_bytes(
    capability: &ValidOperationalRecord,
    exercise: &ValidOperationalRecord,
) -> Result<Vec<u8>, InterventionIntakeError> {
    let mut envelope = Map::new();
    envelope.insert(
        "schema_version".to_owned(),
        Value::Number(PAIR_SCHEMA_VERSION.into()),
    );
    envelope.insert(
        "records".to_owned(),
        Value::Array(vec![
            capability.document().clone(),
            exercise.document().clone(),
        ]),
    );
    let stored =
        from_serde(&Value::Object(envelope)).map_err(|error| invalid(vec![error.to_string()]))?;
    quoin_store::canonical_json_bytes(&stored).map_err(|error| invalid(vec![error.to_string()]))
}

/// Everything intake asks of a record that the schema cannot.
///
/// `operational.ts:232-267`.
fn validate_for_intake(
    repo: &Path,
    candidate: &ValidOperationalRecord,
    retained: &[&ValidOperationalRecord],
) -> Result<(), InterventionIntakeError> {
    let source = DiskMeasurement::new(repo);
    // `From<MeasurementError>` (quoin#471) is the one place a crate refusal
    // becomes an intake refusal. It is an exhaustive match, so a code added to
    // `MeasurementErrorCode` is placed deliberately; the per-call-site matches
    // quoin#472 wrote here agreed with it on every code these three calls can
    // return, and would have silently disagreed on the next one. It also keeps
    // the rendered sentence as the finding when a refusal carries none, where
    // `error.findings().to_vec()` handed the caller an empty refusal.
    let plans = load_measurement_plans(&source, PlanLoadOptions::default())?;
    let base = candidate.record().base();
    assert_governing_definition(&plans, &base.producer.definition_version)?;
    verify_raw_evidence_references(&source, &base.raw_evidence)?;
    let candidate_bytes = candidate.canonical_bytes()?;
    for item in retained {
        if item.record_id() == candidate.record_id() && item.canonical_bytes()? != candidate_bytes {
            return Err(collision(vec![candidate.record_id().to_string()]));
        }
    }
    if let OperationalEvidenceRecord::Exercise(exercise) = candidate.record() {
        check_linkage(exercise, retained)?;
    }
    Ok(())
}

/// `operational.ts:255-266`: an exercise that names a capability names one that
/// is retained and that it matches.
fn check_linkage(
    exercise: &OperationalExerciseRecord,
    retained: &[&ValidOperationalRecord],
) -> Result<(), InterventionIntakeError> {
    let Some(named) = exercise.exercise.capability_record_id.as_ref() else {
        return Ok(());
    };
    let matched = retained.iter().any(|item| match item.record() {
        OperationalEvidenceRecord::StandingCapability(capability) => {
            &capability.base.record_id == named && linked(capability, exercise)
        }
        OperationalEvidenceRecord::Exercise(_) => false,
    });
    if matched {
        Ok(())
    } else {
        Err(invalid(vec![
            "/exercise/capability_record_id: linked capability does not match control, kind, \
             subject, and scope"
                .to_owned(),
        ]))
    }
}

/// Whether a capability and an exercise are about the same control, in the same
/// place, on the same subject. `operational.ts:280-289`.
fn linked(capability: &StandingCapabilityRecord, exercise: &OperationalExerciseRecord) -> bool {
    capability.capability.control_id == exercise.exercise.control_id
        && capability.base.control_kind == exercise.base.control_kind
        && capability.base.subject == exercise.base.subject
        && capability.base.scope == exercise.base.scope
}

/// `operational.ts:273-278`: write, and refuse to replace.
///
/// [`write_content_addressed`] links rather than renames and `fsync`s both the
/// file and its directory — the same strengthening the measurement store took
/// in quoin#468, declared in `DIVERGENCE.md`.
fn write_once(path: &Path, bytes: &[u8]) -> Result<(), InterventionIntakeError> {
    write_content_addressed(path, bytes).map_err(|error| match error {
        quoin_store::StoreError::ContentCollision { .. } => {
            collision(vec![format!("{}: retained bytes differ", path.display())])
        }
        other => invalid(vec![other.to_string()]),
    })?;
    Ok(())
}

fn invalid(findings: Vec<String>) -> InterventionIntakeError {
    InterventionIntakeError::new(InterventionRefusalCode::InvalidRecord, findings)
}

fn collision(findings: Vec<String>) -> InterventionIntakeError {
    InterventionIntakeError::new(InterventionRefusalCode::RecordIdCollision, findings)
}
