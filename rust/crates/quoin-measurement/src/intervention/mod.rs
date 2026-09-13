// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Intervention experiments: the record, its refusal vocabulary and its report.
//!
//! | retained TypeScript | here |
//! | --- | --- |
//! | `intervention-types.ts` | [`record`] |
//! | `intervention-report.ts` | [`report`] |
//! | `intervention.ts:33-45` | [`ids`] |
//! | `intervention.ts:47-63` | [`validate`] |
//! | `intervention.ts:65-113` | [`semantics`] |
//! | `intervention.ts:66-114,218-423` | [`intake`] |
//! | `agent-eval-intervention.ts` | [`agent_eval`] |
//!
//! The record declarations and the report renderer are quoin#469's half;
//! everything else is quoin#471's. The raw-evidence accounting that
//! `intervention.ts:115-216` also carries is not here — it is
//! [`crate::raw_evidence`], where quoin#468 put it, because operational
//! records need it too.
//!
//! # Why the record-id encoding has its own module
//!
//! [`ids`] is eleven lines of TypeScript. It is also the only place in this
//! crate where caller-supplied text becomes a file name, and the only place
//! where two encodings must provably never collide. Buried mid-file it reads
//! as a formatting helper; as a module with its own tests measured against
//! RFC 4648's published vectors, it reads as what it is.

pub mod agent_eval;
pub mod ids;
pub mod intake;
pub mod record;
pub mod report;
pub mod semantics;
pub mod validate;
