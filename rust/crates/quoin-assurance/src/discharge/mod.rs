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
//! # There is exactly one door into [`DischargeFact`], and it validates
//!
//! [`DischargeFact`] is what `parseFact` *produces*, so a DERIVED
//! `Deserialize` would be a second, permissive door into the same struct that
//! skips every predicate below — and that door was open. It was not
//! theoretical: [`ClauseDischarge::fact`] carries a [`DischargeFact`] into
//! [`DischargeBinding`] and so into [`DischargeReport`], which `quoin-core`
//! reads straight off untrusted stdin for `assurance.render_discharge` and as
//! the `discharge` field of `assurance.build_authored_argument`. A report
//! nothing computed — empty `authority`, an `attestedAt` that is not an
//! instant, an expiry before it, an `evidenceDigest` that is not one, no
//! evidence at all — was accepted whole, echoed into the emitted view, and
//! reported `supported`, because `openReasons` only asks whether the `open`
//! and `unresolved` populations are empty.
//!
//! Deleting the derive would only have moved the rule to call-site discipline.
//! So [`DischargeFact`] and [`DischargeAttestation`] carry
//! a hand-written [`serde::Deserialize`] instead: it reads the permissive wire value
//! and hands it to `parseFact` / `parseAttestation` (this crate's `parse` module), which are the SAME
//! predicates a request goes through. Hand-written and not
//! `#[serde(try_from = "serde_json::Value")]`, which reads identically but
//! also tells `schemars` to publish both types as `unknown`, emptying the
//! boundary schema of the shape it exists to state. [`DirectDischargeFact`] and [`DispositionFact`] carry no
//! `Deserialize` at all — they exist only as the payload of a
//! [`DischargeFact`], and a standalone derive on either would be the second
//! door again, one level down.
//!
//! `tc_447_450` replays the forged report and asserts the refusal.
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

mod build;
#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod fixtures;
mod parse;
mod render;
mod report;

pub use build::build_discharge_report;
pub use render::render_discharge_report;
pub use report::{
    BuildDischargeRequest, ClauseDischarge, DirectDischargeFact, DischargeAttestation,
    DischargeBinding, DischargeError, DischargeFact, DischargeReport, DischargeSchemaVersion,
    DischargeState, DispositionDecision, DispositionFact, FactKind, UnusedDischargeFact,
    UnusedFactReason,
};
