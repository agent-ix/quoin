// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! quoin's quire edge, as a Cargo dependency (quoin#379, EPIC quoin#373 Stage 1).
//!
//! This crate is the Rust successor to `src/quire/`. It covers every capability
//! that directory exposes, by calling [`quire_rs`] **directly** instead of
//! executing a `quire` binary and reading its stdout.
//!
//! ## What the boundary change removes
//!
//! `src/quire/` is 1,342 lines, and roughly half of it exists only because the
//! boundary was a subprocess with no shared types:
//!
//! | `src/quire/` | here |
//! |---|---|
//! | `exec.ts` — executable resolution, `QUOIN_EXPECTED_QUIRE_SHA256` bytes pinning, `maxBuffer`, three-way termination taxonomy | **gone.** A linked crate has no path to resolve, no bytes to pin at runtime, no pipe to overflow and no exit status. The pin is `rev` in `Cargo.toml`; see [`engine`] |
//! | `contract.ts` — five vendored JSON Schemas, a SHA-256 per file, a source revision, a `minimumCli` floor | **gone.** See [`engine`] for what each of the three guarantees becomes |
//! | `validate.ts` — ajv compilation and `ContractViolation` | [`payload`], where the type a payload is read into is the engine's own |
//! | `types.ts` / `assurance.ts` — hand-written mirrors of the payloads | re-exported engine types, plus one hand-written payload the engine deliberately does not serialize (see [`properties`]) |
//! | `index.ts` — the one import site | this module's re-exports |
//!
//! `exec.ts` is **retained** in the TypeScript tree until Stage 8 and nothing
//! here deletes it (FR-018): it is still the template `src/core/exec.ts` was
//! copied from.
//!
//! ## What the boundary change does not remove
//!
//! The resource bound. `exec.ts` carries a 64 MiB ceiling because a
//! 1,090,714-byte `coverage --json` payload killed six quoin commands
//! (agent-ix/quoin#164). A library call cannot die that way — and quoin still
//! reads stored artifacts of unbounded size, so the ceiling moves rather than
//! disappearing. See [`payload::PayloadLimit`].
//!
//! ## Capabilities
//!
//! | capability | here | replaces |
//! |---|---|---|
//! | coverage rollup | [`coverage::compute`] | `runQuire(["coverage", …])` + `parseCoverage` |
//! | criterion classification | [`properties::classify`] | `runQuireAllowFailure(["properties", …])` + `parseProperties` |
//! | document validation | [`validate::run`] | `runQuireBatch(["validate", …])` |
//! | clause-set evaluation and diff | [`clauses::evaluate`], [`clauses::diff`] | `parseClauseBinding` |
//! | assurance export | [`assurance::build`], [`assurance::read`] | `parseAssurance` / `validateAssurance` |
//! | payload ingest under a ceiling | [`payload`] | `QUIRE_MAX_BUFFER` + `validate*` |
//! | instrument identity | [`engine`] | `quireVersion`, `checkVersionPremise`, `QUIRE_CONTRACT` |

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]
#![warn(clippy::pedantic)]

pub mod assurance;
pub mod clauses;
pub mod coverage;
pub mod engine;
pub mod error;
pub mod ids;
pub mod modules;
pub mod payload;
pub mod properties;
pub mod validate;

#[cfg(test)]
mod testing;

pub use error::{Error, ErrorCode, Result};
pub use ids::{
    ClauseSetAuthority, ClauseSetId, ClauseSetRef, ClauseSetVersion, DocumentPath, ModuleRoot,
    RepositoryId, RevisionId, ScopeRoot,
};
pub use modules::{ModuleSelection, Notice, NoticeKind};
pub use payload::{ClauseBindingPayload, CoveragePayload, PayloadLimit};

/// The engine types this crate hands back, re-exported so a consumer needs one
/// import site — the job `src/quire/index.ts` did.
///
/// These are **not** redeclarations. `quoin_quire::model::CoverageReport` *is*
/// `quire_rs::CoverageReport`; there is no second definition that could drift
/// from the payload the engine emits, which is the whole point of the Cargo
/// edge and the reason the vendored-schema machinery does not come across.
pub mod model {
    pub use quire_rs::assurance::{
        AssuranceArtifact, AssuranceLocator, AssuranceModulePremise, AssuranceObligation,
        AssuranceRelation, AssuranceSchemaPremise, AssuranceSymbol,
    };
    pub use quire_rs::coverage::{
        CoverageDiagnostic, CoverageTotals, CriteriaCounts, GroupCounts, GroupCounts as Group,
        ImplementsRecord, MintedTargetRecord, NoSymbolRow, SharedTraceId, StatusLie, UnbackedRow,
        UndeclaredStatus, UntrackedSymbol, VocabularyValueRecord,
    };
    pub use quire_rs::metric::{Metric, MetricShape};
    pub use quire_rs::obligation::{CriterionObligation, Obligation};
    pub use quire_rs::symbols::trace::{BindingCensus, UnmatchedTag};
    pub use quire_rs::{
        AcceptedAssurancePremises, AssuranceError, AssuranceExport, AssuranceSource, ClauseBinding,
        ClauseBindingReport, ClauseForce, ClauseSet, ClauseSetDiff, ClauseSetKey, CoverageReport,
    };
}
