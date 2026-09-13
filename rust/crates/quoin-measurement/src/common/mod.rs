// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The vocabulary both measurement record families share.
//!
//! `operational-types.ts` imports `RawEvidenceReference` from
//! `intervention-types.ts` — ported here as
//! [`recorded_evidence::RecordedEvidenceReference`], named apart from
//! quoin#468's minted [`crate::raw_evidence::RawEvidenceReference`] — and then
//! redeclares `subject`, `producer` and the environment map inline: the same
//! shapes, written twice. This module is the one declaration of each, which is
//! the plan's §13.2 criterion applied at port time rather than left for a
//! later deduplication pass.
//!
//! [`schema`] is the same idea one level up: the schema pass itself — how a
//! validator error becomes a finding, and how a refusal's findings are
//! ordered — is one rendering, not one per family.
//!
//! The crossing between [`quoin_store`]'s JSON model and `serde_json`'s is
//! *not* here: it is [`crate::json_bridge`], declared once for the whole
//! crate. quoin#471 and quoin#472 each landed one; they are one.

pub mod identity;
pub mod literal_bool;
pub mod producer;
pub mod recorded_evidence;
pub mod scalar;
pub(crate) mod schema;
pub mod wire_enum;
