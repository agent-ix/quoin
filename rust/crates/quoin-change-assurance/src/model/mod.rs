// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The four sealed shapes, and the reading vocabulary they share.
//!
//! One module per shape, plus [`json`] for the closed-object reader every
//! shape uses and [`outcome`] for the verdict vocabulary three of them carry.
//! Nothing here touches a filesystem: the shapes are values, and where they
//! are retained is [`crate::intake`]'s question.

pub mod attestation;
pub mod attestation_shape;
pub mod decision;
pub mod json;
pub mod outcome;
pub mod receipt;
pub mod record;

pub use attestation::{
    EnvironmentValue, ProducerResult, ProofAttestation, RetainedOutput, ToolBinding,
    UnsealedAttestation,
};
pub use decision::{
    ActorKind, DecisionHistory, DecisionPayload, RetainedDecisionEvent, ReviewDecision, ReviewEvent,
};
pub use json::compare_utf16;
pub use outcome::{Check, Outcome, Reason, normalize_reasons, outcome_for_reasons};
pub use receipt::{
    AuditFinding, ProofResult, ReceiptChecks, ReceiptDecisionEvent, ReceiptUnknown,
    UnsealedReceipt, VerificationReceipt,
};
pub use record::{
    ChangeAssuranceRecord, CommandBinding, Completeness, Definition, Disposition, ImpactSnapshot,
    ProofObligation, RecordSubject, ReviewWorkflow, SourceConnection, SourceKind, Statement,
    Unknown, UnsealedRecord, normalize_record,
};
