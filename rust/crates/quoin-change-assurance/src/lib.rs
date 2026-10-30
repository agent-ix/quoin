// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
#![forbid(unsafe_code)]
//! Change-assurance records, proof attestations, verification receipts and the
//! ix-flow decision history (quoin FR-063, FR-064, FR-065).
//!
//! # What this crate claims, and what it does not
//!
//! Everything here is **retained content and recorded attribution**. It is not
//! authentication, not authorization, not a signature, and not a claim that a
//! named person did anything: an actor recorded against a review decision is
//! attribution, retained because a reader will want it, and never a permission.
//! Records are addressed by `(record_id, revision)` and attestations by
//! `attestation_id`; nothing is addressed by content.
//!
//! Nothing here runs a producer, an auditor, a workflow, Git, or a network
//! call. A verification reads retained evidence and says what it finds. When
//! this crate reports that a proof passed, it is reporting that an attestation
//! someone else produced says so and that the retained output has the size it declares.
//!
//! # Where the rules live
//!
//! - [`model`] — the four shapes and the closed reason vocabulary.
//! - [`records`] — preparing records, and the strict N-1 parent chain.
//! - [`attestations`] — reading one producer's attestation.
//! - [`verify`] — the verification, and the receipt it always produces.
//! - [`intake`] — retaining and reading evidence, behind one trait.
//! - [`error`] — every refusal, with a stable code.
//!
//! # Canonicalization comes from `quoin-store`
//!
//! This crate contains no canonicalizer and no JSON serializer. JCS bytes,
//! canonical bytes and strict parsing all come from [`quoin_store`], which is
//! the single implementation FR-100-CON-4 requires;
//! `tests/tc_455_boundary.rs` asserts over this crate's own sources that no
//! second one has appeared.
//!
//! ```
//! use quoin_change_assurance::intake::{EvidenceStore, memory::MemoryEvidenceStore};
//!
//! let mut store = MemoryEvidenceStore::new();
//! assert_eq!(store.recover_staging().unwrap(), 0);
//! ```

pub mod attestations;
pub mod error;
pub mod ids;
pub mod intake;
pub mod model;
pub mod records;
pub mod schemas;
pub mod verify;

pub use crate::error::{
    ChangeAssuranceError, ChangeAssuranceErrorCode, FieldFailure, LineageFailure, Subject,
};
pub use crate::ids::{
    AttestationId, EventId, Identity, ImpactIdentity, NonEmptyText, ObligationId, ProofId,
    RecordId, Revision, RunId, SourceId, StatementId,
};
pub use crate::intake::{
    EvidenceStore, Published, RetainedPair, intake_attestation, read_attestation,
    read_change_record, write_change_record,
};
pub use crate::model::attestation::ProofAttestation;
pub use crate::model::outcome::{Check, Outcome, Reason};
pub use crate::model::receipt::{ProofResult, VerificationReceipt};
pub use crate::model::record::ChangeAssuranceRecord;
pub use crate::records::{prepare_change_record, read_change_record_json, verify_lineage};
pub use crate::verify::input::VerificationInput;
pub use crate::verify::{verify_change_assurance, verify_receipt};
