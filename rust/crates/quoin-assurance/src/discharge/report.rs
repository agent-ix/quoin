// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The discharge report's declared shape, and the one rejection type the
//! module reports through (quoin#384, FR-046).
//!
//! Types and their wire spellings only. The predicates that admit a document
//! into one of these live in [`super::parse`]; the report is assembled in
//! [`super::build`] and rendered in [`super::render`].

use std::collections::BTreeMap;

use quoin_quire_types::{ClauseBindingReport, ClauseSetKey};
use serde::{Deserialize, Serialize};

use super::parse::{parse_attestation, parse_fact};

/// A rejection, carrying the retained implementation's own message.
///
/// Shaped exactly like [`crate::argument::ArgumentError`] and for the same
/// reason: the retained `buildDischargeReport` throws one `Error` for all
/// eleven predicates below, every one of them leaves the boundary as
/// `CORE_BAD_REQUEST`, and no caller can branch on which fired. The message is
/// reproduced because it costs nothing and helps a human, but the native fixture suite
/// compares `(code, context keys)` and never the prose (quoin#373).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DischargeError(pub String);

impl std::fmt::Display for DischargeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for DischargeError {}

pub(super) type Checked<T> = Result<T, DischargeError>;

pub(super) fn reject<T>(message: impl Into<String>) -> Checked<T> {
    Err(DischargeError(message.into()))
}

/// Who attested to a discharge, under what authority, and for how long.
///
/// Deserialised through [`parse_attestation`] and never by a derived reader —
/// see the module header. The wire form is the permissive
/// [`serde_json::Value`]; the predicates decide whether it becomes one of
/// these.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DischargeAttestation {
    /// The actor making the attestation.
    pub attested_by: String,
    /// The authority they hold to make it.
    pub authority: String,
    /// When it was made. An instant.
    pub attested_at: String,
    /// When it stops being current. An instant, strictly after
    /// [`DischargeAttestation::attested_at`].
    pub expires_at: String,
    /// The revision the attestation is about.
    pub source_revision: String,
    /// `sha256:<64 lowercase hex>`.
    pub evidence_digest: String,
}

/// The one door: every read of an attestation runs [`parse_attestation`].
///
/// Hand-written rather than `#[serde(try_from = "serde_json::Value")]`, which
/// says the same thing to `serde` and a DIFFERENT thing to `schemars`: that
/// attribute also replaces the generated `$defs` entry with the schema of
/// `serde_json::Value`, so the boundary schema — and the TypeScript generated
/// from it — would describe this type as `unknown`. The reader is the same
/// predicate either way; only the published shape differs.
impl<'de> Deserialize<'de> for DischargeAttestation {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        Self::try_from(value).map_err(|error| serde::de::Error::custom(error.0))
    }
}

impl TryFrom<serde_json::Value> for DischargeAttestation {
    type Error = DischargeError;

    fn try_from(value: serde_json::Value) -> Checked<Self> {
        parse_attestation(Some(&value))
    }
}

/// Which of the two fact shapes this is.
///
/// Separate from [`DischargeFact`] because the retained
/// `UnusedDischargeFact.kind` is declared `DischargeFact["kind"]` — the
/// discriminant without the payload — and an unused fact reports only that.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum FactKind {
    /// Evidence that the clause's expected output exists.
    Direct,
    /// An authorised decision about the clause, which is not the same thing.
    Disposition,
}

impl FactKind {
    /// Every variant, in the order the retained `oneOf` table listed them —
    /// which is the order the refusal message names them in.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[Self::Direct, Self::Disposition]
    }

    /// The wire spelling. The ONE source of it: `parse_fact`'s membership
    /// table is built from this, and `tc_447_452` pins it against what
    /// `#[serde(rename_all)]` emits.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Disposition => "disposition",
        }
    }
}

/// What an approved disposition decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum DispositionDecision {
    /// The risk is knowingly carried.
    AcceptedRisk,
    /// An exception with an expiry.
    TemporaryException,
    /// Responsibility passed to another party.
    Delegated,
}

impl DispositionDecision {
    /// Every variant, in the retained `oneOf` table's order.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[
            Self::AcceptedRisk,
            Self::TemporaryException,
            Self::Delegated,
        ]
    }

    /// The wire spelling, and the one source of it. See [`FactKind::as_str`].
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AcceptedRisk => "accepted_risk",
            Self::TemporaryException => "temporary_exception",
            Self::Delegated => "delegated",
        }
    }
}

/// Evidence that a clause's expected output exists.
///
/// `Serialize` only: it is reachable from the wire exclusively as the payload
/// of a [`DischargeFact`], whose one door is [`parse_fact`]. A derived
/// `Deserialize` here would be that second, permissive door one level down.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DirectDischargeFact {
    /// The clause this discharges.
    pub clause_id: String,
    /// Non-empty, and its entries unique.
    pub evidence_refs: Vec<String>,
    /// Who attested, and for how long.
    pub attestation: DischargeAttestation,
}

/// An authorised decision about a clause.
///
/// FR-046's constraint, restated where the type is: a disposition is evidence
/// of an authorised decision, **not** evidence that the clause's expected
/// output exists. That is why it never joins the `direct` population.
///
/// `Serialize` only, for the reason [`DirectDischargeFact`] states.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DispositionFact {
    /// The clause this disposes of.
    pub clause_id: String,
    /// What was decided.
    pub decision: DispositionDecision,
    /// Why.
    pub rationale: String,
    /// A reference to the approval itself.
    pub approval_ref: String,
    /// Who attested, and for how long.
    pub attestation: DischargeAttestation,
}

/// One discharge fact.
///
/// # Why an internally tagged enum
///
/// The retained type is a discriminated union on `kind` whose members carry
/// their fields **flat** beside the discriminant:
/// `{"kind":"direct","clauseId":…,"evidenceRefs":[…],"attestation":{…}}`.
/// `#[serde(tag = "kind")]` is the one serde representation that reproduces
/// those bytes — externally tagged would nest the payload under a `"direct"`
/// key and adjacently tagged would add a second one, and either would change
/// the `clause-discharge-v1` document that `ClauseDischarge::fact` embeds.
/// A struct with a `kind` field would have reproduced the JSON too, at the
/// cost of making `evidenceRefs` and `approvalRef` simultaneously optional in
/// the type — which is the invariant the union exists to state.
///
/// # Why the READER is hand-written and not derived
///
/// The tagged representation above describes what this type EMITS. What it
/// ACCEPTS is [`parse_fact`], reached through the hand-written
/// [`Deserialize`] below, so the predicates a
/// `build_discharge` request goes through are the same ones a
/// `render_discharge` request goes through. See the module header for the
/// forged report that made the difference observable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum DischargeFact {
    /// Evidence.
    Direct(DirectDischargeFact),
    /// A decision.
    Disposition(DispositionFact),
}

/// The one door: every read of a fact runs [`parse_fact`].
///
/// Hand-written for the reason [`DischargeAttestation`]'s reader gives — the
/// `try_from` attribute would have published this union as `unknown`.
impl<'de> Deserialize<'de> for DischargeFact {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        Self::try_from(value).map_err(|error| serde::de::Error::custom(error.0))
    }
}

impl TryFrom<serde_json::Value> for DischargeFact {
    type Error = DischargeError;

    fn try_from(value: serde_json::Value) -> Checked<Self> {
        parse_fact(&value)
    }
}

impl DischargeFact {
    /// The clause this fact is about.
    #[must_use]
    pub fn clause_id(&self) -> &str {
        match self {
            Self::Direct(fact) => &fact.clause_id,
            Self::Disposition(fact) => &fact.clause_id,
        }
    }

    /// The discriminant alone.
    #[must_use]
    pub fn kind(&self) -> FactKind {
        match self {
            Self::Direct(_) => FactKind::Direct,
            Self::Disposition(_) => FactKind::Disposition,
        }
    }

    /// The attestation, whichever shape carries it.
    #[must_use]
    pub fn attestation(&self) -> &DischargeAttestation {
        match self {
            Self::Direct(fact) => &fact.attestation,
            Self::Disposition(fact) => &fact.attestation,
        }
    }
}

/// Where one clause landed in the partition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum DischargeState {
    /// Discharged by evidence.
    Direct,
    /// Discharged by an authorised decision.
    Disposition,
    /// Binding and not discharged.
    Open,
    /// Applicability could not be decided.
    Unresolved,
    /// Does not apply.
    NotBinding,
}

impl DischargeState {
    /// Every variant, so a test can walk the whole set.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[
            Self::Direct,
            Self::Disposition,
            Self::Open,
            Self::Unresolved,
            Self::NotBinding,
        ]
    }

    /// The wire spelling, and the one source of it. See [`FactKind::as_str`].
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Disposition => "disposition",
            Self::Open => "open",
            Self::Unresolved => "unresolved",
            Self::NotBinding => "not_binding",
        }
    }
}

/// One clause, with the state the accounting put it in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ClauseDischarge {
    /// The clause's id.
    pub clause_id: String,
    /// Copied from the binding report.
    pub force: quoin_quire_types::ClauseForce,
    /// Where it landed.
    pub state: DischargeState,
    /// Copied from the binding report. May be empty.
    pub expected_outputs: Vec<String>,
    /// Why, when there is a why. The key is **absent**, not null, when there
    /// is not — the retained `entry()` spreads `...(reason ? { reason } : {})`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// The fact that decided it, when one did. Absent, not null, otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fact: Option<DischargeFact>,
}

/// Why a supplied fact was not spent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum UnusedFactReason {
    /// No clause in the binding report carries that id.
    UnknownClause,
    /// The clause does not apply.
    NotBinding,
    /// The clause's applicability is undecided, so nothing can be spent on it.
    Unresolved,
}

impl UnusedFactReason {
    /// Every variant, so a test can walk the whole set.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[Self::UnknownClause, Self::NotBinding, Self::Unresolved]
    }

    /// The wire spelling, and the one source of it. See [`FactKind::as_str`].
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnknownClause => "unknown_clause",
            Self::NotBinding => "not_binding",
            Self::Unresolved => "unresolved",
        }
    }
}

/// A fact that was supplied and not spent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UnusedDischargeFact {
    /// The clause the fact named.
    pub clause_id: String,
    /// The fact's discriminant, without its payload.
    pub kind: FactKind,
    /// Why it was not spent.
    pub reason: UnusedFactReason,
}

/// The binding population, partitioned three ways.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct DischargeBinding {
    /// Discharged by evidence.
    pub direct: Vec<ClauseDischarge>,
    /// Discharged by an authorised decision.
    pub dispositions: Vec<ClauseDischarge>,
    /// Binding and not discharged.
    pub open: Vec<ClauseDischarge>,
}

/// The one accepted `schemaVersion` of a discharge report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum DischargeSchemaVersion {
    /// `clause-discharge-v1`.
    #[serde(rename = "clause-discharge-v1")]
    V1,
}

/// A complete, non-scored discharge partition.
///
/// Every input population appears. There is no aggregate anywhere in this
/// type, and FR-046-AC-2 asserts the absence directly — a score over a
/// denominator that excludes unresolved applicability would read as a
/// measurement of something nobody measured.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DischargeReport {
    /// Always `clause-discharge-v1`.
    pub schema_version: DischargeSchemaVersion,
    /// Copied from the binding report.
    pub clause_set: ClauseSetKey,
    /// Copied from the binding report.
    pub clause_set_digest: String,
    /// A copy of the binding report's context, ordered so the serialised
    /// document is byte-stable.
    pub context: BTreeMap<String, String>,
    /// The ORIGINAL request string, not a re-rendering of the parsed instant.
    /// `buildDischargeReport` compares `instant(request.asOf)` and emits
    /// `request.asOf`, so `2026-08-15T00:00:00.000Z` is echoed with its
    /// fraction and `+00:00` is echoed as `+00:00`.
    pub as_of: String,
    /// The binding population.
    pub binding: DischargeBinding,
    /// Undecided applicability, kept out of the binding denominator.
    pub unresolved: Vec<ClauseDischarge>,
    /// Clauses that do not apply.
    pub not_binding: Vec<ClauseDischarge>,
    /// Facts supplied and not spent. Clause-ordered `unresolved`/`not_binding`
    /// entries first, then fact-ordered `unknown_clause` entries — the two
    /// loops of the retained implementation, in its order.
    pub unused_facts: Vec<UnusedDischargeFact>,
}

/// What `buildDischargeReport` is given.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BuildDischargeRequest {
    /// A validated `clause-binding-v1` report.
    pub binding: ClauseBindingReport,
    /// Unvalidated discharge facts. See the module header for why these are
    /// untyped.
    pub facts: Vec<serde_json::Value>,
    /// Explicit evaluation instant; this layer never reads the wall clock.
    pub as_of: String,
}
