// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The evidence auditor (FR-032) and the verification-method advisor (FR-031).
//!
//! # Why one crate
//!
//! Nothing ships the advisor without the auditor: every consumer imports both
//! in the same file, the advisor's `fault-detection-*` characteristics are
//! computed by the auditor's own [`audit::scores_for`], and both read one
//! merged [`catalog::MethodCatalog`]. Splitting them would mean either
//! duplicating the score definition — so the auditor's finding and the
//! advisor's recommendation could disagree about what a score is — or minting
//! a third crate for two functions. The catalog itself already had to leave
//! `advisor/` once, because the auditor reaching into it was one half of an
//! import cycle Cargo forbids (agent-ix/quoin#376).
//!
//! # ADR-0011 invariant 1
//!
//! `src/auditor/audit.ts:14` states it plainly: *"the auditor runs nothing; it
//! reads the store and reports"*. Here that is not a convention. See
//! [`inert`]: the auditor's inputs implement a sealed [`Inert`] marker whose
//! supertrait is `serde::de::DeserializeOwned`, so a process handle, a file
//! descriptor, a closure or a `dyn Trait` **cannot be a field of an auditor
//! input** — none of them deserialize. Execution is unrepresentable in the
//! type, not merely unused by the code.
//!
//! The one place this crate touches the world is
//! [`catalog::ModuleCatalogSource`], a trait the caller supplies. It is not
//! [`Inert`] and cannot be reached from anything that is.
//!
//! # Entry points
//!
//! * [`audit::audit`] — the report.
//! * [`advise::advise`] — the recommendation.
//! * [`catalog::load_method_catalog`] — the merged catalog both read.
//! * [`audit::ratchet`] / [`audit::delta`] — the gate and the per-PR change.

#![forbid(unsafe_code)]

pub mod advise;
pub mod audit;
pub mod catalog;
pub mod error;
pub mod inert;
pub mod jsvalue;

pub use advise::{
    Advice, MatchReason, ObligationEvidence, ObligationFacts, PropertyShape, Recommendation,
    UNCATALOGUED_METHOD_REASON, UncataloguedMethods, advise, advise_all, archetype_of,
    characteristics_of, evidence_for, facts_for, mintable_characteristics,
    uncatalogued_authored_methods,
};
pub use audit::{
    AuditInput, Delta, MOCK_SUBJECT_FLOOR, audit, delta, finding_key, ratchet, scores_for,
};
pub use catalog::{
    DiskModuleCatalogSource, MemoryModuleCatalogSource, MethodCatalog, ModuleCatalogSource,
    ModuleRoot, VerificationMethod, load_method_catalog, method_classes,
};
pub use error::{AuditorError, AuditorErrorCode};
pub use inert::Inert;
