// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! What the advisor is told, and what it answers.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// What the evidence store records about one obligation.
///
/// Only what the advisor needs to answer a single question: has anything
/// measured that this obligation's tests would catch a fault?
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ObligationEvidence {
    /// True when at least one binding names this obligation.
    pub bound: bool,
    /// Fault-detection scores from runs bound to it — entries whose declared
    /// `metric` says they are mutation scores (agent-ix/quoin#138).
    ///
    /// Empty while `bound` is true is the interesting case, not an error: the
    /// obligation is exercised and nothing says the exercise discriminates.
    #[serde(default)]
    pub fault_detection_scores: Vec<f64>,
}

/// What the advisor knows about one obligation before it recommends anything.
///
/// Every field is supplied by the caller. `advise` performs no I/O — the
/// command opens the store and hands the answer in, which is what makes the
/// whole advisor [`Inert`](crate::Inert).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ObligationFacts {
    /// The obligation id.
    pub id: String,
    /// The criterion text, as authored.
    pub statement: String,
    /// The authored `Verification` cell, when the row has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authored_method: Option<String>,
    /// The FR-052 property shape quire classified the criterion as.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub property_shape: Option<String>,
    /// The owning document's archetype (`FR`, `NFR`, `StR`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archetype: Option<String>,
    /// Object types the spec declares near this requirement, if any.
    #[serde(default)]
    pub object_types: Vec<String>,
    /// What the evidence store already knows about this obligation.
    ///
    /// Absent means "not consulted", which is different from "consulted and
    /// empty" and must not mint anything.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<ObligationEvidence>,
    /// The obligation's declared criticality, verbatim (`P0`, `high`, …).
    ///
    /// Carried rather than interpreted. `mutation-testing` is keyed on
    /// `high-criticality`, and the engine does not get to decide which values
    /// count as high — CR-008 deleted a hardcoded `["P0"]` for exactly that
    /// reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub criticality: Option<String>,
    /// The obligation's structured `parameters`, as quire emits them.
    ///
    /// The only *structured* signal quire provides about an obligation, and it
    /// was typed, parsed and read nowhere while the advisor re-derived
    /// everything by regex over the statement prose (agent-ix/quoin#166).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<BTreeMap<String, String>>,
    /// True when the engine's `uncatalogued-verification-method` diagnostic
    /// names this obligation's authored method (agent-ix/quoin#168).
    ///
    /// Supplied by the caller, never derived here. Absent means the
    /// classification was unavailable — an engine predating CR-091 — and the
    /// advisor degrades to the two-state behaviour rather than guessing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncatalogued_method: Option<bool>,
}

/// Why a method was recommended — the rule and the value that matched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct MatchReason {
    /// The applicability axis.
    pub rule: String,
    /// The value on that axis the obligation carries.
    pub value: String,
}

/// One recommendation for one obligation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct Recommendation {
    /// The recommended method's id.
    pub method: String,
    /// Its IADT class.
    pub class: String,
    /// The evidence kind it produces, when the module declared one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_kind: Option<String>,
    /// Every rule that matched, sorted by rule then value.
    pub reasons: Vec<MatchReason>,
}

/// The advisor's verdict for one obligation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Advice {
    /// The obligation the verdict is about.
    pub obligation: String,
    /// Deterministic recommendations, strongest (most rules matched) first.
    pub recommended: Vec<Recommendation>,
    /// The authored method, normalized.
    pub authored: Option<String>,
    /// A genuine disagreement — *did you mean to inspect this rather than test
    /// it?* Set when the authored method **is a declared method or class**, is
    /// not among the recommendations, AND the advisor had rules to go on.
    ///
    /// Never set for an uncatalogued value (agent-ix/quoin#168): a word the
    /// catalog has never heard of is a vocabulary problem, not a choice the
    /// advisor disagrees with.
    pub mismatch: bool,
    /// The authored value is in neither the catalog's method set nor its class
    /// set, per the engine's own `uncatalogued-verification-method` diagnostic.
    ///
    /// Orthogonal to `inconclusive`: the vocabulary fact does not disappear
    /// because the advisor was silent.
    pub uncatalogued: bool,
    /// True when no rule matched anything. The honest outcome — an advisor
    /// that recommends `Test` because it found nothing is the habit this
    /// replaces.
    pub inconclusive: bool,
}
