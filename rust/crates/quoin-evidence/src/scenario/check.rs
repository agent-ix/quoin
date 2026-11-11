// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The intake's admission rules: contract version, typed shape, coherence.

use std::collections::BTreeSet;

use serde_json::Value;

use super::{
    INTAKE_CONTRACT, LABEL, PinnedEntry, ResolutionState, ScenarioEvidence, ScenarioRefusal,
};
use crate::error::EvidenceError;

fn refuse(kind: ScenarioRefusal, detail: impl Into<String>) -> EvidenceError {
    EvidenceError::ScenarioRefused {
        kind,
        detail: detail.into(),
    }
}

/// Check the contract version first, then the typed shape, then coherence.
pub(super) fn admit(document: &Value) -> Result<ScenarioEvidence, EvidenceError> {
    match document.get("contract").and_then(Value::as_str) {
        Some(INTAKE_CONTRACT) => {}
        Some(other) => {
            return Err(refuse(
                ScenarioRefusal::WrongContract,
                format!("contract {other:?} is not {INTAKE_CONTRACT:?}"),
            ));
        }
        None => {
            return Err(refuse(
                ScenarioRefusal::MissingComponent,
                "contract is absent",
            ));
        }
    }
    let record: ScenarioEvidence = serde_json::from_value(document.clone())
        .map_err(|error| EvidenceError::invalid(LABEL, error.to_string()))?;
    check(&record)?;
    Ok(record)
}

fn nonblank(field: &str, value: &str) -> Result<(), EvidenceError> {
    if value.trim().is_empty() {
        return Err(refuse(
            ScenarioRefusal::MissingComponent,
            format!("{field} is blank"),
        ));
    }
    Ok(())
}

fn path_safe(field: &str, value: &str) -> Result<(), EvidenceError> {
    nonblank(field, value)?;
    if !value
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err(refuse(
            ScenarioRefusal::CrossWired,
            format!("{field} {value:?} may hold only ASCII letters, digits and '-'"),
        ));
    }
    Ok(())
}

fn pinned(entry: &PinnedEntry, field: &str) -> Result<(), EvidenceError> {
    nonblank(&format!("{field}.owner"), &entry.owner)?;
    nonblank(&format!("{field}.contractVersion"), &entry.contract_version)?;
    nonblank(&format!("{field}.revision"), &entry.revision)
}

fn unique_entries<'a>(
    entries: &'a [PinnedEntry],
    field: &str,
) -> Result<BTreeSet<(&'a str, &'a str)>, EvidenceError> {
    let mut keys = BTreeSet::new();
    for entry in entries {
        pinned(entry, field)?;
        if !keys.insert((entry.owner.as_str(), entry.contract_version.as_str())) {
            return Err(refuse(
                ScenarioRefusal::DuplicateComponent,
                format!(
                    "{field} holds ({}, {}) twice",
                    entry.owner, entry.contract_version
                ),
            ));
        }
    }
    Ok(keys)
}

fn check(record: &ScenarioEvidence) -> Result<(), EvidenceError> {
    path_safe("scenario", &record.scenario)?;
    let prefix = format!("{}-", record.integration.as_str());
    if !record.scenario.starts_with(&prefix) {
        return Err(refuse(
            ScenarioRefusal::CrossWired,
            format!(
                "scenario {:?} does not belong to {}",
                record.scenario,
                record.integration.as_str()
            ),
        ));
    }

    let attributable: Vec<&PinnedEntry> = match &record.resolution {
        ResolutionState::Full { entries } => {
            if entries.is_empty() {
                return Err(refuse(
                    ScenarioRefusal::MissingComponent,
                    "resolution.entries is empty",
                ));
            }
            unique_entries(entries, "resolution.entries")?;
            entries.iter().collect()
        }
        ResolutionState::Partial {
            reached,
            causing_entry,
        } => {
            let keys = unique_entries(reached, "resolution.reached")?;
            nonblank("resolution.causingEntry.owner", &causing_entry.owner)?;
            nonblank(
                "resolution.causingEntry.contractVersion",
                &causing_entry.contract_version,
            )?;
            nonblank("resolution.causingEntry.reason", &causing_entry.reason)?;
            if keys.contains(&(
                causing_entry.owner.as_str(),
                causing_entry.contract_version.as_str(),
            )) {
                return Err(refuse(
                    ScenarioRefusal::CrossWired,
                    "the causing entry is also listed as reached",
                ));
            }
            reached.iter().collect()
        }
        ResolutionState::LegacyCompatibility { selection } => {
            pinned(selection, "resolution.selection")?;
            vec![selection]
        }
    };

    let mut values = BTreeSet::new();
    for owner in &record.value_owners {
        nonblank("valueOwners.value", &owner.value)?;
        nonblank("valueOwners.owner", &owner.owner)?;
        nonblank("valueOwners.contractVersion", &owner.contract_version)?;
        nonblank("valueOwners.revision", &owner.revision)?;
        if !values.insert(owner.value.as_str()) {
            return Err(refuse(
                ScenarioRefusal::DuplicateComponent,
                format!("value {:?} has two owners", owner.value),
            ));
        }
        let attributed = attributable.iter().any(|entry| {
            entry.owner == owner.owner
                && entry.contract_version == owner.contract_version
                && entry.revision == owner.revision
        });
        if !attributed {
            return Err(refuse(
                ScenarioRefusal::CrossWired,
                format!(
                    "value {:?} names ({}, {}, {}), which the resolution state does not hold",
                    owner.value, owner.owner, owner.contract_version, owner.revision
                ),
            ));
        }
    }

    nonblank("outcome.operation", &record.outcome.operation)?;
    for claim in &record.outcome.claims {
        nonblank("outcome.claims.identity", claim.identity())?;
    }

    if record.budget.is_empty() {
        return Err(refuse(ScenarioRefusal::MissingComponent, "budget is empty"));
    }
    let mut dimensions = BTreeSet::new();
    for figure in &record.budget {
        if !dimensions.insert(figure.dimension) {
            return Err(refuse(
                ScenarioRefusal::DuplicateComponent,
                format!("budget holds dimension {:?} twice", figure.dimension),
            ));
        }
    }

    nonblank("traceBinding.criterion", &record.trace_binding.criterion)
}
