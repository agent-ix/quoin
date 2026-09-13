// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The `clause-binding-v1` wire format, as quoin reads it (quoin#384, FR-046).
//!
//! Ported from the `ClauseSetKey` … `ClauseBindingReport` block of
//! `src/quire/types.ts`. **quire owns this format**; the crate header's rule
//! applies unchanged — a field here is a reader, and its spelling is quire's,
//! not ours.
//!
//! # Why these carry `Serialize` as well as `Deserialize`
//!
//! [`Obligation`](crate::Obligation) is read and never re-emitted, so it is
//! `Deserialize`-only. These are not: `buildDischargeReport` copies
//! `clauseSet`, `clauseSetDigest`, `context` and every clause's `force` and
//! `expectedOutputs` straight into the `clause-discharge-v1` document it
//! emits. A `Deserialize`-only type would have forced a second, parallel set
//! of output structs whose only job was to spell the same five fields again —
//! which is the drift surface this crate exists to remove.
//!
//! # Why every reader here denies unknown fields
//!
//! `clause-binding-v1` arrives on untrusted stdin as the `binding` half of an
//! `assurance.build_discharge` request, so rust-style's rule applies: a field
//! the boundary silently drops is a field the caller believes it sent. It is
//! not a bet against quire evolving the format —
//! [`ClauseBindingSchemaVersion`] is closed to `clause-binding-v1`, so a new
//! field IS a new version, and a `clause-binding-v2` payload already has to
//! fail to read rather than be read with v1 semantics.
//!
//! # `context` is a `BTreeMap`, not a `HashMap`
//!
//! `clause-discharge-v1` is canonical JSON on the way out (rust-style, the
//! boundary contract), and `serde_json::Map` is a `BTreeMap` for the same
//! reason. A `HashMap` here would put a random key order into the emitted
//! document and make the difftest comparison against the retained
//! implementation fail intermittently rather than never.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Exact identity of a module-supplied clause set (quire-rs FR-067).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ClauseSetKey {
    /// The authority that publishes the clause set.
    pub authority: String,
    /// The clause set's id within that authority.
    pub id: String,
    /// The clause set's version.
    pub version: String,
}

/// How strongly a clause binds.
///
/// Closed, for the reason quoin#425 settled: the retained `parseClauseBinding`
/// validates membership against the pinned schema and refuses anything else,
/// so refusing an unlisted value IS the retained behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ClauseForce {
    /// The clause must be satisfied.
    Mandatory,
    /// The clause is advised.
    Recommended,
    /// The clause is allowed but not required.
    Permitted,
}

impl ClauseForce {
    /// The wire spelling, for rendering.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mandatory => "mandatory",
            Self::Recommended => "recommended",
            Self::Permitted => "permitted",
        }
    }
}

/// Whether a clause applies to the evaluated context.
///
/// `Unresolved` is the load-bearing member and is deliberately **not** a
/// synonym for `NotBinding`: FR-046 keeps unresolved applicability outside the
/// discharge denominator entirely, because collapsing the two would let a
/// missing-context clause read as a decided one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum ClauseBindingOutcome {
    /// The clause applies.
    Binding,
    /// The clause does not apply.
    NotBinding,
    /// Applicability could not be decided from the supplied context.
    Unresolved,
}

/// Why the binder reached the outcome it did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ClauseBindingReason {
    /// A stable machine code for the reason.
    pub code: String,
    /// The context dimension the reason is about, when it names one.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub dimension: Option<String>,
    /// The reason in the binder's own words. `buildDischargeReport` joins
    /// these with `"; "` when a clause is unresolved.
    pub message: String,
}

/// One clause and the binder's verdict on it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ClauseBinding {
    /// The clause's id within its clause set.
    pub clause_id: String,
    /// How strongly it binds.
    pub force: ClauseForce,
    /// Whether it applies.
    pub outcome: ClauseBindingOutcome,
    /// May be empty, which is why the discharge report falls back to
    /// `"applicability is unresolved"`.
    pub reasons: Vec<ClauseBindingReason>,
    /// The outputs the clause expects. May be empty.
    pub expected_outputs: Vec<String>,
}

/// Which executable and engine produced a payload.
///
/// # Why a struct and not a `serde_json::Value`
///
/// `src/quire/types.ts` declares `EngineProvenance` with three concrete
/// fields — `cli`, `engine`, `capabilities` — so a struct is the honest
/// model: it says what the format is, and the crate's rule is that a reader
/// spells quire's field names exactly. A `Value` would have been the honest
/// choice only if the shape were open or undeclared, and it is neither.
///
/// The field is `Option` on [`ClauseBindingReport`] rather than required
/// because the retained type declares it `engine?:`. That is the one
/// documented exception to the crate header's "fields quoin reads are
/// required": quoin does not read it at all — `buildDischargeReport` never
/// looks at it — so there is no blank row for an absence to produce.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct EngineProvenance {
    /// The executable that ran.
    pub cli: String,
    /// The engine behind it.
    pub engine: String,
    /// What that engine declares it can do.
    pub capabilities: Vec<String>,
}

/// The one accepted `schemaVersion` of a clause-binding report.
///
/// Closed rather than a `String`: a `clause-binding-v2` payload must fail to
/// read, not be read with v1 semantics and reported as a clean partition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ClauseBindingSchemaVersion {
    /// `clause-binding-v1`.
    #[serde(rename = "clause-binding-v1")]
    V1,
}

/// Validated output of `quire clauses evaluate --format json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ClauseBindingReport {
    /// Always `clause-binding-v1`.
    pub schema_version: ClauseBindingSchemaVersion,
    /// Which clause set was evaluated.
    pub clause_set: ClauseSetKey,
    /// The digest of the clause set that produced these verdicts.
    pub clause_set_digest: String,
    /// The evaluated context, verbatim. Ordered, so the discharge report that
    /// copies it serialises to stable bytes.
    pub context: BTreeMap<String, String>,
    /// Every clause the set declares, in the set's own order. That order is
    /// significant downstream: FR-046's `unusedFacts` lists clause-ordered
    /// entries before fact-ordered ones.
    pub clauses: Vec<ClauseBinding>,
    /// Which executable and engine produced the report, when stated.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub engine: Option<EngineProvenance>,
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{ClauseBindingOutcome, ClauseBindingReport, ClauseForce};

    fn wire() -> serde_json::Value {
        serde_json::json!({
            "schemaVersion": "clause-binding-v1",
            "clauseSet": {
                "authority": "example.invalid",
                "id": "synthetic-widget-rules",
                "version": "1.0.0"
            },
            "clauseSetDigest": format!("sha256:{}", "a".repeat(64)),
            "context": { "product": "widget", "deployment": "test" },
            "clauses": [{
                "clauseId": "SYN-004",
                "force": "mandatory",
                "outcome": "unresolved",
                "reasons": [{
                    "code": "missing-context",
                    "dimension": "environment",
                    "message": "environment is not known"
                }],
                "expectedOutputs": ["environment-record"]
            }],
            "engine": {
                "cli": "quire",
                "engine": "quire-rs",
                "capabilities": ["clause_sets"]
            }
        })
    }

    /// The camelCase spellings are quire's, not ours; a reader that invents a
    /// name reads nothing and reports empty.
    #[test]
    fn a_clause_binding_report_round_trips_with_quires_own_field_spellings() {
        let parsed: ClauseBindingReport =
            serde_json::from_value(wire()).expect("the pinned wire shape reads");
        assert_eq!(parsed.clauses[0].clause_id, "SYN-004");
        assert_eq!(parsed.clauses[0].force, ClauseForce::Mandatory);
        assert_eq!(parsed.clauses[0].outcome, ClauseBindingOutcome::Unresolved);
        assert_eq!(
            parsed.clauses[0].reasons[0].dimension.as_deref(),
            Some("environment")
        );
        assert_eq!(parsed.context["deployment"], "test");

        let re_emitted = serde_json::to_value(&parsed).expect("it serialises");
        assert_eq!(re_emitted, wire());
    }

    /// An absent `dimension` must not become `"dimension": null` on the way
    /// out; the retained type omits the key.
    #[test]
    fn an_absent_optional_key_is_omitted_rather_than_nulled() {
        let mut value = wire();
        value["clauses"][0]["reasons"][0]
            .as_object_mut()
            .unwrap()
            .remove("dimension");
        value.as_object_mut().unwrap().remove("engine");

        let parsed: ClauseBindingReport =
            serde_json::from_value(value.clone()).expect("both keys are optional");
        assert_eq!(serde_json::to_value(&parsed).expect("it serialises"), value);
    }

    /// A v2 payload must fail to read rather than be read with v1 semantics.
    #[test]
    fn an_unknown_schema_version_is_refused() {
        let mut value = wire();
        value["schemaVersion"] = serde_json::json!("clause-binding-v2");
        assert!(serde_json::from_value::<ClauseBindingReport>(value).is_err());
    }

    /// Every reader in `clause-binding-v1` refuses a field it does not know,
    /// at every depth. The report arrives on untrusted stdin as the `binding`
    /// half of an `assurance.build_discharge` request, so a key silently
    /// dropped here is a clause, a force or a reason the caller believes quoin
    /// weighed.
    ///
    /// The key is injected where it is REACHABLE rather than each struct being
    /// read standalone: an isolated struct refusing it says nothing about the
    /// path a caller actually reaches it by.
    #[test]
    fn every_reachable_clause_binding_type_refuses_an_unknown_field() {
        serde_json::from_value::<ClauseBindingReport>(wire()).expect("the fixture reads");

        for pointer in [
            "",
            "/clauseSet",
            "/clauses/0",
            "/clauses/0/reasons/0",
            "/engine",
        ] {
            let mut forged = wire();
            let target = if pointer.is_empty() {
                &mut forged
            } else {
                forged
                    .pointer_mut(pointer)
                    .unwrap_or_else(|| panic!("{pointer} is a path this report has"))
            };
            target
                .as_object_mut()
                .unwrap_or_else(|| panic!("{pointer} names an object"))
                .insert("inventedField".to_owned(), serde_json::json!(1));

            assert!(
                serde_json::from_value::<ClauseBindingReport>(forged).is_err(),
                "an unknown field at {pointer} must be refused"
            );
        }
    }
}
