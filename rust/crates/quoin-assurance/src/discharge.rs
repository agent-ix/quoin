// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Clause discharge accounting over a validated Quire binding report
//! (FR-046, ported from `src/assurance/discharge.ts`).
//!
//! Applicability and discharge are deliberately separate facts. Quire decides
//! whether a clause is binding, not binding, or unresolved. Quoin partitions
//! only the binding population into direct evidence, an approved disposition,
//! or open. Unresolved applicability stays outside that denominator and no
//! aggregate score is manufactured.
//!
//! # Why `facts` is a `Vec<serde_json::Value>` and not a `Vec<DischargeFact>`
//!
//! The retained signature says `facts: DischargeFact[]`, and the retained body
//! immediately calls `parseFact(value: unknown)` on every element. The type is
//! a compile-time convenience; the runtime contract is `unknown`. A
//! `Vec<DischargeFact>` here would delete `parseFact` entirely — the four
//! rejections `tests/discharge.test.ts` asserts (`kind must be one of …`,
//! `unknown field inventedScore`, `authority must not be empty`, the digest
//! rule) are exactly the ones a typed parameter cannot express, and
//! `tests/discharge.test.ts` reaches three of them by casting
//! `as unknown as DischargeFact`. So the input is untyped and the validation
//! is the port, the same decision [`crate::argument`] records for the same
//! reason.
//!
//! [`DischargeFact`] is therefore `Serialize` only: it is what `parseFact`
//! *produces*, and a derived `Deserialize` would be a second, permissive door
//! into the same struct that skips every predicate below.
//!
//! # The three helpers this module does not own
//!
//! [`crate::argument::js_trim_is_empty`], [`crate::argument::js_trim_end`] and
//! [`crate::argument::instant_epoch_millis`] are `pub(crate)` and reused
//! verbatim. Writing any of them a second time is the specific failure they
//! document: `str::trim` disagrees with JavaScript's on two code points in
//! opposite directions, and `Date.parse` ROLLED an impossible calendar day
//! rather than rejecting it until quoin#436. A second copy is a second chance
//! to get those wrong, and the two would diverge silently because both would
//! look right.
//!
//! `instant_epoch_millis` returns the NUMBER rather than a verdict, which is
//! what this module needs: `currentAttestation` orders three instants against
//! each other, and a predicate cannot order anything.

use std::collections::BTreeMap;

use quoin_quire_types::{ClauseBinding, ClauseBindingOutcome, ClauseBindingReport, ClauseSetKey};
use serde::{Deserialize, Serialize};

use crate::argument::{instant_epoch_millis, js_trim_end, js_trim_is_empty};

/// A rejection, carrying the retained implementation's own message.
///
/// Shaped exactly like [`crate::argument::ArgumentError`] and for the same
/// reason: the retained `buildDischargeReport` throws one `Error` for all
/// eleven predicates below, every one of them leaves the boundary as
/// `CORE_BAD_REQUEST`, and no caller can branch on which fired. The message is
/// reproduced because it costs nothing and helps a human, but `quoin-difftest`
/// compares `(code, context keys)` and never the prose (quoin#373).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DischargeError(pub String);

impl std::fmt::Display for DischargeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for DischargeError {}

type Checked<T> = Result<T, DischargeError>;

fn reject<T>(message: impl Into<String>) -> Checked<T> {
    Err(DischargeError(message.into()))
}

/// Who attested to a discharge, under what authority, and for how long.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// Evidence that a clause's expected output exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum DischargeFact {
    /// Evidence.
    Direct(DirectDischargeFact),
    /// A decision.
    Disposition(DispositionFact),
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

/// One clause, with the state the accounting put it in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
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

/// A fact that was supplied and not spent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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

/// Build a complete, non-scored discharge partition.
///
/// # Errors
///
/// [`DischargeError`] when `asOf` is not an instant, when any supplied fact
/// fails `parseFact`/`parseAttestation`, or when two facts name the same
/// clause. FR-046-AC-5: a duplicate is rejected, never resolved by ordering.
#[expect(
    clippy::too_many_lines,
    reason = "the retained buildDischargeReport is one function, and splitting \
              it would put the partition's reading order somewhere other than \
              the order the oracle decides in, which is what a reader compares"
)]
pub fn build_discharge_report(request: &BuildDischargeRequest) -> Checked<DischargeReport> {
    let as_of = instant("asOf", &request.as_of)?;

    // Every fact is parsed BEFORE the duplicate check, because the retained
    // code is `request.facts.map(parseFact)` followed by the loop that fills
    // the map. A malformed second fact therefore beats a duplicate first one,
    // and reversing that would change which message a caller sees.
    let mut parsed_facts = Vec::with_capacity(request.facts.len());
    for value in &request.facts {
        parsed_facts.push(parse_fact(value)?);
    }
    let mut facts: BTreeMap<&str, &DischargeFact> = BTreeMap::new();
    for fact in &parsed_facts {
        if facts.contains_key(fact.clause_id()) {
            return reject(format!(
                "duplicate discharge fact for clause {}",
                fact.clause_id()
            ));
        }
        facts.insert(fact.clause_id(), fact);
    }

    let mut direct = Vec::new();
    let mut dispositions = Vec::new();
    let mut open = Vec::new();
    let mut unresolved = Vec::new();
    let mut not_binding = Vec::new();
    let mut unused_facts = Vec::new();
    let mut known: Vec<&str> = Vec::with_capacity(request.binding.clauses.len());

    for clause in &request.binding.clauses {
        known.push(clause.clause_id.as_str());
        let fact = facts.get(clause.clause_id.as_str()).copied();
        match clause.outcome {
            ClauseBindingOutcome::Unresolved => {
                unresolved.push(entry(
                    clause,
                    DischargeState::Unresolved,
                    Some(reason_for(clause)),
                    None,
                ));
                if let Some(fact) = fact {
                    unused_facts.push(UnusedDischargeFact {
                        clause_id: clause.clause_id.clone(),
                        kind: fact.kind(),
                        reason: UnusedFactReason::Unresolved,
                    });
                }
            }
            ClauseBindingOutcome::NotBinding => {
                not_binding.push(entry(clause, DischargeState::NotBinding, None, None));
                if let Some(fact) = fact {
                    unused_facts.push(UnusedDischargeFact {
                        clause_id: clause.clause_id.clone(),
                        kind: fact.kind(),
                        reason: UnusedFactReason::NotBinding,
                    });
                }
            }
            ClauseBindingOutcome::Binding => {
                let Some(fact) = fact else {
                    open.push(entry(
                        clause,
                        DischargeState::Open,
                        Some("no discharge fact".to_owned()),
                        None,
                    ));
                    continue;
                };
                if let Some(current) = current_attestation(fact.attestation(), as_of)? {
                    open.push(entry(
                        clause,
                        DischargeState::Open,
                        Some(current),
                        Some(fact),
                    ));
                    continue;
                }
                match fact {
                    DischargeFact::Direct(_) => {
                        direct.push(entry(clause, DischargeState::Direct, None, Some(fact)));
                    }
                    DischargeFact::Disposition(_) => {
                        dispositions.push(entry(
                            clause,
                            DischargeState::Disposition,
                            None,
                            Some(fact),
                        ));
                    }
                }
            }
        }
    }

    for fact in &parsed_facts {
        if !known.contains(&fact.clause_id()) {
            unused_facts.push(UnusedDischargeFact {
                clause_id: fact.clause_id().to_owned(),
                kind: fact.kind(),
                reason: UnusedFactReason::UnknownClause,
            });
        }
    }

    Ok(DischargeReport {
        schema_version: DischargeSchemaVersion::V1,
        clause_set: request.binding.clause_set.clone(),
        clause_set_digest: request.binding.clause_set_digest.clone(),
        context: request.binding.context.clone(),
        as_of: request.as_of.clone(),
        binding: DischargeBinding {
            direct,
            dispositions,
            open,
        },
        unresolved,
        not_binding,
        unused_facts,
    })
}

/// Render the same complete partition as compact, deterministic Markdown.
#[must_use]
pub fn render_discharge_report(report: &DischargeReport) -> String {
    let mut lines = vec![
        format!(
            "# Clause discharge: {}/{}@{}",
            report.clause_set.authority, report.clause_set.id, report.clause_set.version
        ),
        String::new(),
        format!("- Clause-set digest: `{}`", report.clause_set_digest),
        format!("- Evaluated as of: `{}`", report.as_of),
        String::new(),
    ];
    section(&mut lines, "Direct evidence", &report.binding.direct);
    section(
        &mut lines,
        "Approved dispositions",
        &report.binding.dispositions,
    );
    section(&mut lines, "Open binding clauses", &report.binding.open);
    section(&mut lines, "Unresolved applicability", &report.unresolved);
    section(&mut lines, "Not binding", &report.not_binding);
    if !report.unused_facts.is_empty() {
        lines.push("## Unused facts".to_owned());
        lines.push(String::new());
        for fact in &report.unused_facts {
            lines.push(format!(
                "- `{}` ({}): {}",
                fact.clause_id,
                fact_kind_str(fact.kind),
                unused_reason_str(fact.reason)
            ));
        }
        lines.push(String::new());
    }
    let joined = lines.join("\n");
    format!("{}\n", js_trim_end(&joined))
}

/// The wire spelling of a fact kind, for rendering.
fn fact_kind_str(kind: FactKind) -> &'static str {
    match kind {
        FactKind::Direct => "direct",
        FactKind::Disposition => "disposition",
    }
}

/// The wire spelling of an unused-fact reason, for rendering.
fn unused_reason_str(reason: UnusedFactReason) -> &'static str {
    match reason {
        UnusedFactReason::UnknownClause => "unknown_clause",
        UnusedFactReason::NotBinding => "not_binding",
        UnusedFactReason::Unresolved => "unresolved",
    }
}

/// One rendered section, `_None._` when the population is empty.
fn section(lines: &mut Vec<String>, title: &str, entries: &[ClauseDischarge]) {
    lines.push(format!("## {title}"));
    lines.push(String::new());
    if entries.is_empty() {
        // An empty population is stated, never omitted. FR-046's report exists
        // to expose every input population; a section that disappears when it
        // is empty reads as a complete report over a narrower input.
        lines.push("_None._".to_owned());
        lines.push(String::new());
        return;
    }
    for item in entries {
        let outputs = if item.expected_outputs.is_empty() {
            "no declared outputs".to_owned()
        } else {
            item.expected_outputs
                .iter()
                .map(|value| format!("`{value}`"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let reason = match &item.reason {
            Some(reason) => format!("; {reason}"),
            None => String::new(),
        };
        lines.push(format!(
            "- `{}` — {}; {outputs}{reason}",
            item.clause_id,
            item.force.as_str()
        ));
    }
    lines.push(String::new());
}

/// `entry(clause, state, reason?, fact?)`.
fn entry(
    clause: &ClauseBinding,
    state: DischargeState,
    reason: Option<String>,
    fact: Option<&DischargeFact>,
) -> ClauseDischarge {
    ClauseDischarge {
        clause_id: clause.clause_id.clone(),
        force: clause.force,
        state,
        expected_outputs: clause.expected_outputs.clone(),
        // `...(reason ? { reason } : {})` is a TRUTHINESS test, so a reason
        // that renders as the empty string omits the key rather than emitting
        // `"reason": ""`. `reasonFor` produces exactly that when every reason
        // the binder gave carries an empty message.
        reason: reason.filter(|reason| !reason.is_empty()),
        fact: fact.cloned(),
    }
}

/// `reasonFor(clause)`.
fn reason_for(clause: &ClauseBinding) -> String {
    if clause.reasons.is_empty() {
        "applicability is unresolved".to_owned()
    } else {
        clause
            .reasons
            .iter()
            .map(|reason| reason.message.as_str())
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// `parseFact(value)`.
///
/// The reading order is the retained one and is load-bearing for which message
/// a caller sees: `kind`, then `clauseId`, then the whole attestation, and
/// only THEN the exact-keys check. So a fact carrying both an unknown field
/// and an empty `authority` is refused for the authority.
fn parse_fact(value: &serde_json::Value) -> Checked<DischargeFact> {
    let fact = object("discharge fact", Some(value))?;
    let kind = one_of(
        fact.get("kind"),
        "kind",
        &[
            ("direct", FactKind::Direct),
            ("disposition", FactKind::Disposition),
        ],
    )?;
    let clause_id = string_value("clauseId", fact.get("clauseId"))?;
    let attestation = parse_attestation(fact.get("attestation"))?;
    match kind {
        FactKind::Direct => {
            exact(
                fact,
                "direct discharge fact",
                &["kind", "clauseId", "evidenceRefs", "attestation"],
            )?;
            Ok(DischargeFact::Direct(DirectDischargeFact {
                clause_id,
                evidence_refs: non_empty_list("evidenceRefs", fact.get("evidenceRefs"))?,
                attestation,
            }))
        }
        FactKind::Disposition => {
            exact(
                fact,
                "disposition discharge fact",
                &[
                    "kind",
                    "clauseId",
                    "decision",
                    "rationale",
                    "approvalRef",
                    "attestation",
                ],
            )?;
            Ok(DischargeFact::Disposition(DispositionFact {
                clause_id,
                decision: one_of(
                    fact.get("decision"),
                    "decision",
                    &[
                        ("accepted_risk", DispositionDecision::AcceptedRisk),
                        (
                            "temporary_exception",
                            DispositionDecision::TemporaryException,
                        ),
                        ("delegated", DispositionDecision::Delegated),
                    ],
                )?,
                rationale: string_value("rationale", fact.get("rationale"))?,
                approval_ref: string_value("approvalRef", fact.get("approvalRef"))?,
                attestation,
            }))
        }
    }
}

/// `parseAttestation(value)`.
fn parse_attestation(value: Option<&serde_json::Value>) -> Checked<DischargeAttestation> {
    let attestation = object("attestation", value)?;
    exact(
        attestation,
        "attestation",
        &[
            "attestedBy",
            "authority",
            "attestedAt",
            "expiresAt",
            "sourceRevision",
            "evidenceDigest",
        ],
    )?;
    let attested_by = string_value("attestedBy", attestation.get("attestedBy"))?;
    let authority = string_value("authority", attestation.get("authority"))?;
    let source_revision = string_value("sourceRevision", attestation.get("sourceRevision"))?;
    let attested_at = string_value("attestedAt", attestation.get("attestedAt"))?;
    let expires_at = string_value("expiresAt", attestation.get("expiresAt"))?;
    instant("attestedAt", &attested_at)?;
    instant("expiresAt", &expires_at)?;
    let evidence_digest = string_value("evidenceDigest", attestation.get("evidenceDigest"))?;
    if !is_sha256_digest(&evidence_digest) {
        return reject("attestation evidenceDigest must be sha256:<64 lowercase hex>");
    }
    Ok(DischargeAttestation {
        attested_by,
        authority,
        attested_at,
        expires_at,
        source_revision,
        evidence_digest,
    })
}

/// `/^sha256:[0-9a-f]{64}$/`, without a regex engine.
///
/// Lowercase only: `is_ascii_hexdigit` would accept `SHA` in uppercase hex and
/// silently widen the digest alphabet on a field whose whole job is identity.
fn is_sha256_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

/// `currentAttestation(attestation, asOf)`: `None` when the attestation is
/// current, otherwise the reason it is not.
///
/// # Errors
///
/// Never in practice — both instants were validated by `parseAttestation` —
/// but the retained code re-parses here too, so the fallible signature is
/// kept rather than a panic being introduced where the oracle has none.
fn current_attestation(attestation: &DischargeAttestation, as_of: i64) -> Checked<Option<String>> {
    let attested_at = instant("attestedAt", &attestation.attested_at)?;
    let expires_at = instant("expiresAt", &attestation.expires_at)?;
    if expires_at <= attested_at {
        return Ok(Some(
            "attestation expiry is not after attestation".to_owned(),
        ));
    }
    if attested_at > as_of {
        return Ok(Some("attestation is in the future".to_owned()));
    }
    if expires_at <= as_of {
        return Ok(Some("discharge fact is expired".to_owned()));
    }
    Ok(None)
}

/// `instant(name, value)`: the shape and calendar gate, then the number.
///
/// [`instant_epoch_millis`] is the whole implementation. The retained
/// `instant()` in `discharge.ts` and the one in `argument.ts` are the same
/// eight lines — the same regex, the same finiteness check — so a second
/// parser here would be a second thing to keep in agreement with quoin#436's
/// impossible-day fix, on the exact predicate whose divergence that ticket
/// records.
fn instant(name: &str, value: &str) -> Checked<i64> {
    match instant_epoch_millis(value) {
        Some(millis) => Ok(millis),
        None => reject(format!("{name} must be an ISO-8601 instant")),
    }
}

/// `object(name, value)`: an object, and not an array.
fn object<'a>(
    name: &str,
    value: Option<&'a serde_json::Value>,
) -> Checked<&'a serde_json::Map<String, serde_json::Value>> {
    match value.and_then(serde_json::Value::as_object) {
        Some(object) => Ok(object),
        None => reject(format!("{name} must be an object")),
    }
}

/// `exact(value, name, fields)`: no unknown key, and none missing.
fn exact(
    value: &serde_json::Map<String, serde_json::Value>,
    name: &str,
    fields: &[&str],
) -> Checked<()> {
    // `serde_json::Map` is a `BTreeMap` here (`preserve_order` is deliberately
    // off, rust-style), so two unknown keys are reported in sorted order where
    // the retained code reports them in insertion order. The ACCEPTANCE
    // decision is identical; only which of several bad keys is named differs,
    // and quoin#373 records that verdicts are contractual and prose is not.
    for key in value.keys() {
        if !fields.contains(&key.as_str()) {
            return reject(format!("{name} has unknown field {key}"));
        }
    }
    for field in fields {
        if !value.contains_key(*field) {
            return reject(format!("{name} is missing {field}"));
        }
    }
    Ok(())
}

/// `stringValue(name, value)`: a string, non-empty once JavaScript's `trim`
/// has run — and the UNTRIMMED value is what comes back.
///
/// Both halves matter. `"  reviewer-1  "` is accepted and stored with its
/// spaces, so a port that returned `value.trim()` would have emitted a
/// different `attestedBy` into the report than the oracle does. And the
/// emptiness test is JavaScript's set, not Rust's: see
/// [`crate::argument::js_trim_is_empty`].
///
/// One message for both failures, unlike [`crate::argument`]'s `string_at`,
/// which splits them — the retained `stringValue` really does say
/// `must not be empty` for a number.
fn string_value(name: &str, value: Option<&serde_json::Value>) -> Checked<String> {
    match value.and_then(serde_json::Value::as_str) {
        Some(text) if !js_trim_is_empty(text) => Ok(text.to_owned()),
        _ => reject(format!("{name} must not be empty")),
    }
}

/// `nonEmptyList(name, value)`: a non-empty array of non-empty, unique strings.
fn non_empty_list(name: &str, value: Option<&serde_json::Value>) -> Checked<Vec<String>> {
    let Some(array) = value.and_then(serde_json::Value::as_array) else {
        return reject(format!("{name} must contain non-empty values"));
    };
    if array.is_empty() {
        return reject(format!("{name} must contain non-empty values"));
    }
    let mut values = Vec::with_capacity(array.len());
    for item in array {
        values.push(string_value(name, Some(item))?);
    }
    let mut seen: Vec<&String> = Vec::with_capacity(values.len());
    for text in &values {
        if seen.contains(&text) {
            return reject(format!("{name} must contain unique values"));
        }
        seen.push(text);
    }
    Ok(values)
}

/// `oneOf(value, name, allowed)`: membership, checked at run time.
fn one_of<T: Copy>(
    value: Option<&serde_json::Value>,
    name: &str,
    allowed: &[(&str, T)],
) -> Checked<T> {
    if let Some(text) = value.and_then(serde_json::Value::as_str) {
        for (candidate, mapped) in allowed {
            if *candidate == text {
                return Ok(*mapped);
            }
        }
    }
    let names: Vec<&str> = allowed.iter().map(|(name, _)| *name).collect();
    reject(format!("{name} must be one of {}", names.join(", ")))
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{
        BuildDischargeRequest, build_discharge_report, instant, is_sha256_digest, string_value,
    };

    fn digest() -> String {
        format!("sha256:{}", "a".repeat(64))
    }

    fn binding() -> quoin_quire_types::ClauseBindingReport {
        serde_json::from_value(serde_json::json!({
            "schemaVersion": "clause-binding-v1",
            "clauseSet": {
                "authority": "example.invalid",
                "id": "synthetic-widget-rules",
                "version": "1.0.0"
            },
            "clauseSetDigest": digest(),
            "context": { "product": "widget", "deployment": "test" },
            "clauses": [
                {"clauseId": "SYN-001", "force": "mandatory", "outcome": "binding",
                 "reasons": [], "expectedOutputs": ["test-result"]},
                {"clauseId": "SYN-005", "force": "permitted", "outcome": "not_binding",
                 "reasons": [], "expectedOutputs": []}
            ]
        }))
        .expect("the fixture matches the pinned wire shape")
    }

    fn attestation() -> serde_json::Value {
        serde_json::json!({
            "attestedBy": "reviewer-1",
            "authority": "quality-lead",
            "attestedAt": "2026-08-01T00:00:00.000Z",
            "expiresAt": "2026-09-01T00:00:00.000Z",
            "sourceRevision": "0123456789abcdef",
            "evidenceDigest": digest()
        })
    }

    fn direct() -> serde_json::Value {
        serde_json::json!({
            "kind": "direct",
            "clauseId": "SYN-001",
            "evidenceRefs": ["evidence://run/one"],
            "attestation": attestation()
        })
    }

    fn request(facts: Vec<serde_json::Value>) -> BuildDischargeRequest {
        BuildDischargeRequest {
            binding: binding(),
            facts,
            as_of: "2026-08-15T00:00:00.000Z".to_owned(),
        }
    }

    /// `currentAttestation` orders instants, so `instant()` must return the
    /// same NUMBER `Date.parse` does, not merely agree about validity.
    ///
    /// The literals are `Date.parse`'s own output, read out of node and
    /// written down — a re-derivation would agree with itself no matter what
    /// the code did.
    #[test]
    fn instant_returns_the_number_date_parse_returns() {
        let parsed = |value: &str| instant("asOf", value).expect("a valid instant");
        assert_eq!(parsed("1970-01-01T00:00:00Z"), 0);
        assert_eq!(parsed("2026-08-15T00:00:00.000Z"), 1_786_752_000_000);
        assert_eq!(parsed("2026-08-01T00:00:00.000Z"), 1_785_542_400_000);
        assert_eq!(parsed("1969-12-31T23:59:59.999Z"), -1);
        assert_eq!(parsed("2028-02-29T00:00:00Z"), 1_835_395_200_000);
        // `+05:30` is ahead of UTC, so the UTC instant is earlier than the
        // digits read. Getting this sign backwards would move an expiry by
        // twice the offset and silently reopen or discharge a clause.
        assert_eq!(parsed("2026-08-15T05:30:00+05:30"), 1_786_752_000_000);
        assert_eq!(parsed("2026-08-14T19:00:00-05:00"), 1_786_752_000_000);
        // A `Date` holds integer milliseconds; the rest is truncated.
        assert_eq!(parsed("2026-08-15T00:00:00.0009Z"), 1_786_752_000_000);
        // quoin#436: `Date.parse` ROLLED this to March 2 rather than refusing.
        assert_eq!(
            instant("asOf", "2026-02-30T00:00:00Z")
                .expect_err("an impossible day")
                .0,
            "asOf must be an ISO-8601 instant"
        );
    }

    /// The retained `stringValue` trims only to TEST; it returns the original.
    #[test]
    fn string_value_tests_the_trimmed_string_and_returns_the_untrimmed_one() {
        let padded = serde_json::json!("  reviewer-1  ");
        assert_eq!(
            string_value("attestedBy", Some(&padded)).expect("padding is not emptiness"),
            "  reviewer-1  "
        );
        // JavaScript trims U+FEFF and Rust does not: the oracle refuses this.
        assert!(string_value("attestedBy", Some(&serde_json::json!("\u{FEFF}"))).is_err());
        // JavaScript does NOT trim U+0085 and Rust does: the oracle accepts it.
        assert!(string_value("attestedBy", Some(&serde_json::json!("\u{0085}"))).is_ok());
        // A non-string gets the same message the retained code gives.
        assert_eq!(
            string_value("authority", Some(&serde_json::json!(7)))
                .unwrap_err()
                .0,
            "authority must not be empty"
        );
    }

    #[test]
    fn the_digest_alphabet_is_lowercase_hex_only() {
        assert!(is_sha256_digest(&digest()));
        assert!(!is_sha256_digest(&format!("sha256:{}", "A".repeat(64))));
        assert!(!is_sha256_digest(&format!("sha256:{}", "a".repeat(63))));
        assert!(!is_sha256_digest("not-a-digest"));
    }

    /// The parse loop runs to completion before the duplicate check, so a
    /// malformed later fact beats a duplicate earlier one.
    #[test]
    fn a_malformed_fact_is_refused_before_a_duplicate_is_noticed() {
        let mut malformed = direct();
        malformed["kind"] = serde_json::json!("invented");
        let error = build_discharge_report(&request(vec![direct(), direct(), malformed]))
            .expect_err("the third fact is malformed");
        assert_eq!(error.0, "kind must be one of direct, disposition");
    }

    /// `parseFact` reads the attestation BEFORE `exact`, so the attestation
    /// message wins when a fact is wrong in both ways.
    #[test]
    fn the_attestation_is_read_before_the_unknown_field_check() {
        let mut fact = direct();
        fact["inventedScore"] = serde_json::json!(100);
        fact["attestation"]["authority"] = serde_json::json!("");
        let error = build_discharge_report(&request(vec![fact])).expect_err("both predicates fail");
        assert_eq!(error.0, "authority must not be empty");
    }

    /// `asOf` is echoed verbatim; only the comparison uses the parsed number.
    #[test]
    fn as_of_carries_the_original_request_string() {
        let mut input = request(vec![direct()]);
        input.as_of = "2026-08-15T05:30:00.000+05:30".to_owned();
        let report = build_discharge_report(&input).expect("an offset instant is valid");
        assert_eq!(report.as_of, "2026-08-15T05:30:00.000+05:30");
        // And it was compared as the UTC instant it names, which is inside the
        // attestation window, so the clause discharged rather than reopening.
        assert_eq!(report.binding.direct.len(), 1);
    }

    /// The retained `entry()` spreads on TRUTHINESS, so an all-empty reason
    /// set omits the key rather than emitting `"reason": ""`.
    #[test]
    fn an_empty_joined_reason_omits_the_key_entirely() {
        let mut input = request(vec![]);
        input.binding.clauses[0].outcome = quoin_quire_types::ClauseBindingOutcome::Unresolved;
        input.binding.clauses[0].reasons = vec![quoin_quire_types::ClauseBindingReason {
            code: "unnamed".to_owned(),
            dimension: None,
            message: String::new(),
        }];
        let report = build_discharge_report(&input).expect("an empty message is not an error");
        let value = serde_json::to_value(&report).expect("it serialises");
        assert!(
            value["unresolved"][0]
                .as_object()
                .unwrap()
                .get("reason")
                .is_none(),
            "an empty reason must not appear as a key"
        );
    }

    /// `reason` and `fact` are absent keys, never nulls.
    #[test]
    fn a_discharged_entry_omits_reason_and_an_undischarged_one_omits_fact() {
        let report = build_discharge_report(&request(vec![direct()])).expect("valid");
        let value = serde_json::to_value(&report).expect("it serialises");
        let discharged = value["binding"]["direct"][0].as_object().unwrap();
        assert!(discharged.get("reason").is_none());
        assert!(discharged.get("fact").is_some());
        let not_binding = value["notBinding"][0].as_object().unwrap();
        assert!(not_binding.get("reason").is_none());
        assert!(not_binding.get("fact").is_none());
    }

    /// The internally tagged enum must put `kind` beside the payload, not
    /// around it.
    #[test]
    fn a_fact_serialises_with_its_discriminant_flat() {
        let report = build_discharge_report(&request(vec![direct()])).expect("valid");
        let value = serde_json::to_value(&report).expect("it serialises");
        assert_eq!(value["binding"]["direct"][0]["fact"], direct());
    }

    /// Clause-ordered entries first, then fact-ordered ones.
    #[test]
    fn unused_facts_are_clause_ordered_then_fact_ordered() {
        let mut unknown = direct();
        unknown["clauseId"] = serde_json::json!("SYN-999");
        let mut spent_on_not_binding = direct();
        spent_on_not_binding["clauseId"] = serde_json::json!("SYN-005");

        // The unknown-clause fact is supplied FIRST; it must still be listed
        // last, because the retained code walks the clauses before the facts.
        let report = build_discharge_report(&request(vec![unknown, spent_on_not_binding]))
            .expect("both facts are well formed");
        let listed: Vec<&str> = report
            .unused_facts
            .iter()
            .map(|fact| fact.clause_id.as_str())
            .collect();
        assert_eq!(listed, ["SYN-005", "SYN-999"]);
    }

    /// `context` is a copy, and an ordered one.
    #[test]
    fn context_is_copied_and_serialises_in_a_stable_order() {
        let report = build_discharge_report(&request(vec![])).expect("valid");
        assert_eq!(report.context, binding().context);
        let text = serde_json::to_string(&report.context).expect("it serialises");
        assert_eq!(text, r#"{"deployment":"test","product":"widget"}"#);
    }
}
