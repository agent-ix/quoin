// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! One advice pass over every obligation a coverage payload carries.
//!
//! # Why the loop lives here and not in the caller
//!
//! `src/commands/advise.ts` built each [`ObligationFacts`] itself, in
//! `factsFor`/`evidenceFor`, and called `advise` once per obligation. That is
//! fine inside one process and wrong across a boundary: the unit of IPC is a
//! command-shaped operation, so `quoin advise` must be ONE request, not one per
//! obligation. Moving the loop here is what makes that possible, and it moves
//! `scoresFor` off the wire entirely — the advisor and the auditor keep
//! agreeing about what a fault-detection score is by sharing
//! [`crate::audit::scores_for`], rather than by a caller passing scores in.
//!
//! Everything here is still [`crate::Inert`]: the caller opens the store and
//! runs the engine, and hands the answers in.
//!
//! Provenance: quoin#501

use std::collections::BTreeMap;

use quoin_evidence::types::{Binding, RunRecord};
use quoin_quire_types::Obligation;
use serde::{Deserialize, Serialize};

use crate::advise::{Advice, ObligationEvidence, ObligationFacts, UncataloguedMethods, advise};
use crate::audit::scores_for;
use crate::catalog::MethodCatalog;

/// What quire's `properties` view classified one criterion as.
///
/// Read from a second engine call rather than from the coverage payload, which
/// carries neither field. The caller supplies the map because that call spawns
/// a process and this module does no I/O.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PropertyShape {
    /// The FR-052 property shape, e.g. `round-trip`.
    pub property: String,
    /// The owning document's archetype, e.g. `FR`.
    pub archetype: String,
}

/// Advise every obligation, in the order the payload lists them.
///
/// `shapes` is keyed by obligation id; an obligation with no entry is advised
/// from its statement and its id prefix alone, which is a real limit rather
/// than a gap — no classifier ran on it.
#[must_use]
pub fn advise_all(
    catalog: &MethodCatalog,
    obligations: &[Obligation],
    shapes: &BTreeMap<String, PropertyShape>,
    bindings: &[Binding],
    runs: &[RunRecord],
    uncatalogued: &UncataloguedMethods,
) -> Vec<Advice> {
    obligations
        .iter()
        .map(|obligation| {
            let evidence = evidence_for(&obligation.id, bindings, runs);
            let facts = facts_for(
                obligation,
                shapes.get(&obligation.id),
                evidence,
                uncatalogued,
            );
            advise(catalog, &facts)
        })
        .collect()
}

/// What the store records about one obligation.
///
/// `scores_for` is the auditor's, reused rather than reimplemented: one
/// definition of what a fault-detection score is, so the auditor's finding and
/// the advisor's recommendation cannot disagree about the same run.
#[must_use]
pub fn evidence_for(id: &str, bindings: &[Binding], runs: &[RunRecord]) -> ObligationEvidence {
    let mine: Vec<&Binding> = bindings
        .iter()
        .filter(|binding| binding.obligation.as_str() == id)
        .collect();
    let runs: Vec<&RunRecord> = runs.iter().collect();
    ObligationEvidence {
        bound: !mine.is_empty(),
        fault_detection_scores: scores_for(&mine, &runs),
    }
}

/// Everything the advisor is told about one obligation.
#[must_use]
pub fn facts_for(
    obligation: &Obligation,
    shape: Option<&PropertyShape>,
    evidence: ObligationEvidence,
    uncatalogued: &UncataloguedMethods,
) -> ObligationFacts {
    ObligationFacts {
        id: obligation.id.clone(),
        statement: obligation.statement.clone(),
        authored_method: obligation.method.clone(),
        // Byte equality, deliberately: CR-091 guarantees the diagnostic's
        // `value` is the identical string, so any normalization here could
        // only disagree.
        uncatalogued_method: Some(
            obligation
                .method
                .as_ref()
                .is_some_and(|method| uncatalogued.values.contains(method)),
        ),
        property_shape: shape.map(|shape| shape.property.clone()),
        archetype: shape.map_or_else(
            || archetype_of(&obligation.id),
            |shape| Some(shape.archetype.clone()),
        ),
        object_types: Vec::new(),
        criticality: obligation.criticality.clone(),
        // The one STRUCTURED signal quire emits about an obligation. It was
        // typed and parsed and then dropped on this exact seam, so the advisor
        // guessed from prose while `{"target": "< 4 min"}` sat unread (#166).
        parameters: obligation.parameters.clone(),
        evidence: Some(evidence),
    }
}

/// `NFR-006-M-2` → `NFR`. The id prefix is the archetype for every ISO id.
#[must_use]
pub fn archetype_of(id: &str) -> Option<String> {
    let prefix = id.split('-').next().unwrap_or_default();
    if prefix.is_empty() {
        None
    } else {
        Some(prefix.to_owned())
    }
}
